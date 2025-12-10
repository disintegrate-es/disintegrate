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
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;
use std::error::Error as StdError;

use std::marker::PhantomData;

use crate::{Error, PgEventId};
use async_stream::stream;
use async_trait::async_trait;
use disintegrate::{DomainIdentifierInfo, EventStore};
use disintegrate::{Event, PersistedEvent};
use disintegrate::{StreamItem, StreamQuery};
use disintegrate_serde::Serde;

use futures::StreamExt;

/// PostgreSQL event store implementation.
#[derive(Clone)]
pub struct PgEventStore<E, S>
where
    S: Serde<E> + Send + Sync,
{
    pub(crate) pool: PgPool,
    sequence_pool: PgPool,
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
    pub async fn try_new(pool: PgPool, serde: S) -> Result<Self, Error> {
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
        let main_connections = pool.options().get_max_connections();

        // Allocate 25% of connections to sequence pool, minimum 2
        let sequence_connections = std::cmp::max(2, (main_connections as f32 * 0.25).ceil() as u32);

        let sequence_pool = PgPoolOptions::new()
            .max_connections(sequence_connections)
            .connect_lazy_with((*pool.connect_options()).clone());

        Self {
            pool,
            sequence_pool,
            serde,
            event_type: PhantomData,
        }
    }

    /// Configures the maximum number of connections for the sequence pool.
    ///
    /// By default, `PgEventStore` allocates 25% of the main pool connections
    /// to the sequence pool. This method allows adjusting that allocation.
    ///
    /// # Arguments
    ///
    /// * `connections` - The number of connections to allocate to the sequence pool.
    ///
    /// # Returns
    ///
    /// Returns a modified `PgEventStore` instance with the updated sequence pool size.
    pub fn with_sequence_pool_connections(mut self, connections: u32) -> Self {
        self.sequence_pool = PgPoolOptions::new()
            .max_connections(connections.min(self.pool.options().get_max_connections()))
            .connect_lazy_with((*self.pool.connect_options()).clone());
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
    ) -> BoxStream<'a, Result<StreamItem<PgEventId, QE>, Self::Error>>
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
                let payload: QE = payload.try_into().map_err(|e| Error::QueryEventMapping(Box::new(e)))?;
                yield Ok(PersistedEvent::new(id, payload).into());
            }
            yield Ok(StreamItem::End(epoch))
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
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT event_store_begin_epoch()")
            .execute(&mut *tx)
            .await?;
        let mut sequence_insert = InsertEventSequenceBuilder::new(&events);
        let event_ids: Vec<PgEventId> = sequence_insert
            .build()
            .fetch_all(&self.sequence_pool)
            .await?
            .into_iter()
            .map(|r| r.get(0))
            .collect();

        let Some(last_event_id) = event_ids.last().copied() else {
            return Ok(vec![]);
        };

        sqlx::query(&format!(r#"UPDATE event_sequence es SET consumed = consumed + 1, committed = (es.event_id = ANY($1))
                       FROM (SELECT event_id FROM event_sequence WHERE event_id = ANY($1) 
                       OR ((consumed = 0 OR committed = true) 
                       AND (event_id <= $2 AND ({}))) ORDER BY event_id FOR UPDATE) upd WHERE es.event_id = upd.event_id"#,
                    CriteriaBuilder::new(&query.change_origin(version)).build()))
            .bind(&event_ids)
            .bind(last_event_id)
            .execute(&mut *tx)
            .await
            .map_err(map_concurrency_err)?;

        let persisted_events = event_ids
            .iter()
            .zip(events)
            .map(|(event_id, event)| PersistedEvent::new(*event_id, event))
            .collect::<Vec<_>>();
        InsertEventsBuilder::new(persisted_events.as_slice(), &self.serde)
            .build()
            .execute(&mut *tx)
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
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT event_store_begin_epoch()")
            .execute(&mut *tx)
            .await?;
        let mut sequence_insert = InsertEventSequenceBuilder::new(&events).with_consumed(true);
        let event_ids: Vec<PgEventId> = sequence_insert
            .build()
            .fetch_all(&self.sequence_pool)
            .await?
            .into_iter()
            .map(|r| r.get(0))
            .collect();

        sqlx::query("UPDATE event_sequence es SET committed = true WHERE event_id = ANY($1)")
            .bind(&event_ids)
            .execute(&mut *tx)
            .await
            .map_err(map_concurrency_err)?;

        let persisted_events = event_ids
            .iter()
            .zip(events)
            .map(|(event_id, event)| PersistedEvent::new(*event_id, event))
            .collect::<Vec<_>>();
        InsertEventsBuilder::new(persisted_events.as_slice(), &self.serde)
            .build()
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

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

fn map_concurrency_err(err: sqlx::Error) -> Error {
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
        "CREATE INDEX IF NOT EXISTS idx_{table}_{column_name} ON {table} ({column_name}) WHERE {column_name} IS NOT NULL"
    ))
    .execute(pool)
    .await?;
    Ok(())
}
