//! PostgreSQL Event Store
//!
//! This module provides an implementation of the `EventStore` trait using PostgreSQL as the underlying storage.
//! It allows storing and retrieving events from a PostgreSQL database.
mod append;
mod query;
#[cfg(test)]
mod tests;

use append::{InsertEventSequenceBuilder, InsertEventsBuilder};
use futures::stream::BoxStream;
use query::CriteriaBuilder;
use sqlx::{PgPool, Row};
use std::error::Error as StdError;
use std::sync::Arc;
use tokio::sync::Semaphore;

use std::marker::PhantomData;

use crate::{Error, PgEventId};
use async_stream::stream;
use async_trait::async_trait;
use disintegrate::StreamQuery;
use disintegrate::{DomainIdentifierInfo, EventStore};
use disintegrate::{Event, PersistedEvent};
use disintegrate_serde::Serde;

use futures::StreamExt;

/// PostgreSQL event store implementation.
#[derive(Clone)]
pub struct PgEventStore<E, S>
where
    S: Serde<E> + Send + Sync,
{
    pub(crate) pool: PgPool,
    concurrent_appends: Arc<tokio::sync::Semaphore>,
    serde: S,
    event_type: PhantomData<E>,
}

impl<E, S> PgEventStore<E, S>
where
    S: Serde<E> + Send + Sync,
    E: Event,
{
    /// Initializes the PostgreSQL DB and returns a new instance of `PgEventStore`.
    ///
    /// # Arguments
    ///
    /// * `pool` - The PostgreSQL connection pool.
    /// * `serde` - The serialization implementation for the event payload.
    pub async fn new(pool: PgPool, serde: S) -> Result<Self, Error> {
        setup::<E>(&pool).await?;
        Ok(Self::new_uninitialized(pool, serde))
    }
    /// Creates a new instance of `PgEventStore`.
    ///
    /// This constructor does not initialize the database or add the
    /// `domain_identifier` columns necessary for `disintegrate` to function properly.
    /// If you need to initialize the database, use `PgEventStore::new` instead.
    ///
    /// If you plan to use this constructor, ensure that the `disintegrate` is
    /// properly initialized. Refer to the SQL files in the "event_store/sql" directory
    /// to recreate the default structure. Additionally, all `domain_identifier` columns
    /// and their corresponding indexes must be created manually.
    ///
    /// # Arguments
    ///
    /// * `pool` - The PostgreSQL connection pool.
    /// * `serde` - The serialization implementation for the event payload.
    pub fn new_uninitialized(pool: PgPool, serde: S) -> Self {
        const MAX_APPENDS_CONNECTIONS_PERCENT: f64 = 0.5;
        let concurrent_appends = Arc::new(Semaphore::new(
            (pool.options().get_max_connections() as f64 * MAX_APPENDS_CONNECTIONS_PERCENT).ceil()
                as usize,
        ));
        Self {
            pool,
            concurrent_appends,
            serde,
            event_type: PhantomData,
        }
    }

    /// Limits the maximum number of concurrent appends based on the PostgreSQL connection pool.
    ///
    /// By default, `PgEventStore` allows up to 50% of the available database connections
    /// to be used for concurrent appends. This method allows adjusting that limit.
    ///
    /// The number of concurrent appends is determined by multiplying the total maximum
    /// connections of the `PgPool` by the specified `percentage`.
    ///
    /// # Arguments
    ///
    /// * `percentage` - A floating-point number (0.0 to 1.0) representing the
    ///   proportion of the total database connections allocated for concurrent appends.
    ///
    /// # Returns
    ///
    /// Returns a modified `PgEventStore` instance with the updated append concurrency limit.
    pub fn with_max_appends_connections_percent(mut self, percentage: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&percentage),
            "percentage must be between 0 and 1"
        );

        self.concurrent_appends = Arc::new(Semaphore::new(
            (self.pool.options().get_max_connections() as f64 * percentage).ceil() as usize,
        ));
        self
    }
}

/// Implementation of the event store using PostgreSQL.
///
/// This module provides the implementation of the `EventStore` trait for `PgEventStore`,
/// allowing interaction with a PostgreSQL event store. It enables streaming events based on
/// a query and appending new events to the event store.
#[async_trait]
impl<E, S> EventStore<PgEventId, E> for PgEventStore<E, S>
where
    E: Event + Send + Sync,
    S: Serde<E> + Send + Sync,
{
    type Error = Error;

    /// Streams events based on the provided query.
    ///
    /// This function fetches events from the PostgreSQL event store based on the provided
    /// `query`. It constructs a SQL query using the `SqlEventsCriteriaBuilder` and executes
    /// the query using the `sqlx` crate. The fetched events are then converted into
    /// `PersistedEvent` instances and streamed as a boxed stream.
    ///
    /// # Arguments
    ///
    /// * `query` - The stream query specifying the criteria for filtering events.
    ///
    /// # Returns
    ///
    /// A `Result` containing a boxed stream of `PersistedEvent` that matches the query criteria,
    /// or an error of type `Self::Error`.
    fn stream<'a, QE>(
        &'a self,
        query: &'a StreamQuery<PgEventId, QE>,
    ) -> BoxStream<'a, Result<PersistedEvent<PgEventId, QE>, Self::Error>>
    where
        QE: TryFrom<E> + Event + 'static + Clone + Send + Sync,
        <QE as TryFrom<E>>::Error: StdError + 'static + Send + Sync,
    {
        stream! {
            let epoch: i64 = sqlx::query_scalar("SELECT event_store_current_epoch()").fetch_one(&self.pool).await?;
            let sql = format!("SELECT event_id, payload FROM event WHERE event_id <= {epoch} AND ({}) ORDER BY event_id ASC", CriteriaBuilder::new(query).build());

            for await row in sqlx::query(&sql)
            .fetch(&self.pool) {
                let row = row?;
                let id = row.get(0);

                let payload = self.serde.deserialize(row.get(1))?;
                yield Ok(PersistedEvent::new(id, payload.try_into().map_err(|e| Error::QueryEventMapping(Box::new(e)))?));
            }
        }
        .boxed()
    }

    /// Appends new events to the event store.
    ///
    /// This function inserts the provided `events` into the PostgreSQL event store by performing
    /// two separate inserts. First, it inserts the events into the `event_sequence` table to reclaim
    /// a set of IDs for the events. Then, it inserts the events into the `event` table along with
    /// their IDs, event types, domain identifiers, and payloads. Finally, it marks the event IDs as `consumed`
    /// in the event sequence table. If marking the event IDs as consumed fails (e.g., another process has already consumed the IDs),
    /// a conflict error is raised. This conflict indicates that the data retrieved by the query is stale,
    /// meaning that the events generated are no longer valid due to being generated from an old version
    /// of the event store.
    ///
    /// # Arguments
    ///
    /// * `events` - A vector of events to be appended.
    /// * `query` - The stream query specifying the criteria for filtering events.
    /// * `version` - The ID of the last consumed event.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `PersistedEvent` representing the appended events,
    /// or an error of type `Self::Error`.
    async fn append<QE>(
        &self,
        events: Vec<E>,
        query: StreamQuery<PgEventId, QE>,
        version: PgEventId,
    ) -> Result<Vec<PersistedEvent<PgEventId, E>>, Self::Error>
    where
        E: Clone + 'async_trait,
        QE: Event + Clone + Send + Sync,
    {
        let _permit = self.concurrent_appends.acquire().await?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT event_store_begin_epoch()")
            .execute(&mut *tx)
            .await?;
        let mut persisted_events = Vec::with_capacity(events.len());
        let mut persisted_events_ids: Vec<PgEventId> = Vec::with_capacity(events.len());
        for event in events {
            let mut staged_event_insert = InsertEventSequenceBuilder::new(&event);
            let row = staged_event_insert.build().fetch_one(&self.pool).await?;
            persisted_events_ids.push(row.get(0));
            persisted_events.push(PersistedEvent::new(row.get(0), event));
        }

        let Some(last_event_id) = persisted_events_ids.last().copied() else {
            return Ok(vec![]);
        };
        let joined_event_ids = persisted_events_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sqlx::query(&format!(r#"UPDATE event_sequence es SET consumed = consumed + 1, committed = (es.event_id = ANY('{{{joined_event_ids}}}'))
                       FROM (SELECT event_id FROM event_sequence WHERE event_id IN ({joined_event_ids}) 
                       OR ((consumed = 0 OR committed = true) 
                       AND (event_id <= {last_event_id} AND ({}))) ORDER BY event_id FOR UPDATE) upd WHERE es.event_id = upd.event_id"#,
                    CriteriaBuilder::new(&query.change_origin(version)).build()))
            .execute(&mut *tx)
            .await
            .map_err(map_update_event_id_err)?;

        InsertEventsBuilder::new(persisted_events.as_slice(), &self.serde)
            .build()
            .execute(&self.pool)
            .await?;

        tx.commit().await?;

        Ok(persisted_events)
    }

    /// Appends a batch of events to the PostgreSQL-backed event store **without** verifying  
    /// whether new events have been added since the last read.  
    ///
    /// # Arguments
    ///
    /// * `events` - A vector of events to be appended.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `PersistedEvent` representing the appended events,
    /// or an error of type `Self::Error`.
    async fn append_without_validation(
        &self,
        events: Vec<E>,
    ) -> Result<Vec<PersistedEvent<PgEventId, E>>, Self::Error>
    where
        E: Clone + 'async_trait,
    {
        let mut persisted_events = Vec::with_capacity(events.len());
        let mut persisted_events_ids: Vec<PgEventId> = Vec::with_capacity(events.len());
        for event in events {
            let mut sequence_insert = InsertEventSequenceBuilder::new(&event)
                .with_consumed(true)
                .with_committed(true);
            let row = sequence_insert.build().fetch_one(&self.pool).await?;
            persisted_events_ids.push(row.get(0));
            persisted_events.push(PersistedEvent::new(row.get(0), event));
        }

        InsertEventsBuilder::new(persisted_events.as_slice(), &self.serde)
            .build()
            .execute(&self.pool)
            .await?;

        Ok(persisted_events)
    }
}

pub async fn setup<E: Event>(pool: &PgPool) -> Result<(), Error> {
    const RESERVED_NAMES: &[&str] = &["event_id", "payload", "event_type", "inserted_at"];

    sqlx::query(include_str!("event_store/sql/table_event.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!("event_store/sql/idx_event_type.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!("event_store/sql/table_event_sequence.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!("event_store/sql/idx_event_sequence_type.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!(
        "event_store/sql/idx_event_sequence_committed.sql"
    ))
    .execute(pool)
    .await?;
    sqlx::query(include_str!(
        "event_store/sql/fn_event_store_current_epoch.sql"
    ))
    .execute(pool)
    .await?;
    sqlx::query(include_str!(
        "event_store/sql/fn_event_store_begin_epoch.sql"
    ))
    .execute(pool)
    .await?;

    for domain_identifier in E::SCHEMA.domain_identifiers {
        if RESERVED_NAMES.contains(&domain_identifier.ident) {
            panic!("Domain identifier name {domain_identifier} is reserved. Please use a different name.", domain_identifier = domain_identifier.ident);
        }
        add_domain_identifier_column(pool, "event", domain_identifier).await?;
        add_domain_identifier_column(pool, "event_sequence", domain_identifier).await?;
    }
    Ok(())
}

/// Maps the `sqlx::Error` to `Error::UpdateEventIdError`.
fn map_update_event_id_err(err: sqlx::Error) -> Error {
    if let sqlx::Error::Database(ref description) = err {
        if description.code().as_deref() == Some("23514") {
            return Error::Concurrency;
        }
    }
    Error::Database(err)
}

async fn add_domain_identifier_column(
    pool: &PgPool,
    table: &str,
    domain_identifier: &DomainIdentifierInfo,
) -> Result<(), Error> {
    let column_name = domain_identifier.ident;
    let sql_type = match domain_identifier.type_info {
        disintegrate::IdentifierType::String => "TEXT",
        disintegrate::IdentifierType::i64 => "BIGINT",
        disintegrate::IdentifierType::Uuid => "UUID",
    };
    sqlx::query(&format!(
        "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS {column_name} {sql_type}"
    ))
    .execute(pool)
    .await?;

    sqlx::query(&format!(
        "CREATE INDEX IF NOT EXISTS idx_{table}_{column_name} ON {table} USING HASH ({column_name}) WHERE {column_name} IS NOT NULL"
    ))
    .execute(pool)
    .await?;
    Ok(())
}
