//! PostgreSQL Event Store
//!
//! This module provides an implementation of the `EventStore` trait using PostgreSQL as the underlying storage.
//! It allows storing and retrieving events from a PostgreSQL database.
mod insert_builder;
mod query_builder;
#[cfg(test)]
mod tests;

use insert_builder::InsertBuilder;
use query_builder::QueryBuilder;
use sqlx::Row;
use std::error::Error as StdError;

use std::marker::PhantomData;

use crate::Error;
use async_stream::stream;
use async_trait::async_trait;
use disintegrate::stream_query::StreamQuery;
use disintegrate::EventStore;
use disintegrate::{Event, PersistedEvent};
use disintegrate_serde::Serde;
use futures::stream::BoxStream;
use futures::StreamExt;
use sqlx::PgPool;

/// PostgreSQL event store implementation.
#[derive(Clone)]
pub struct PgEventStore<E, S>
where
    S: Serde<E> + Send + Sync,
{
    pub(crate) pool: PgPool,
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
        Ok(Self {
            pool,
            serde,
            event_type: PhantomData,
        })
    }
}

/// Implementation of the event store using PostgreSQL.
///
/// This module provides the implementation of the `EventStore` trait for `PgEventStore`,
/// allowing interaction with a PostgreSQL event store. It enables streaming events based on
/// a query and appending new events to the event store.
#[async_trait]
impl<E, S> EventStore<E> for PgEventStore<E, S>
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
        query: &'a StreamQuery<QE>,
    ) -> BoxStream<Result<PersistedEvent<QE>, Self::Error>>
    where
        QE: TryFrom<E> + Event + Send + Sync + Clone,
        <QE as TryFrom<E>>::Error: StdError + 'static + Send + Sync,
    {
        stream! {
            let mut sql = QueryBuilder::new(query, "SELECT event_id, payload FROM event WHERE ")
            .end_with("ORDER BY event_id ASC");

            for await row in sql.build()
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
    /// * `last_event_id` - The ID of the last consumed event.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `PersistedEvent` representing the appended events,
    /// or an error of type `Self::Error`.
    async fn append<QE>(
        &self,
        events: Vec<E>,
        query: StreamQuery<QE>,
        last_event_id: i64,
    ) -> Result<Vec<PersistedEvent<E>>, Self::Error>
    where
        E: Clone + 'async_trait,
        QE: Event + Clone + Send + Sync,
    {
        let mut persisted_events = Vec::new();
        for event in events {
            let mut sequence_insert =
                InsertBuilder::new(&event, "event_sequence").returning("event_id");
            let row = sequence_insert.build().fetch_one(&self.pool).await?;
            persisted_events.push(PersistedEvent::new(row.get(0), event));
        }

        let mut tx = self.pool.begin().await?;
        let last_persisted_event_id = persisted_events
            .last()
            .map(|e| e.id())
            .unwrap_or(last_event_id);
        let mut update_sql = QueryBuilder::new(
            &query,
            "UPDATE event_sequence SET consumed = consumed + 1 WHERE ",
        )
        .with_origin(last_event_id + 1)
        .with_last_event_id(last_persisted_event_id);

        update_sql
            .build()
            .execute(&mut tx)
            .await
            .map_err(map_update_event_id_err)?;
        for event in &persisted_events {
            let payload = self.serde.serialize((**event).clone());
            let mut event_insert = InsertBuilder::new(&**event, "event")
                .with_id(event.id())
                .with_payload(&payload);
            event_insert.build().execute(&mut tx).await?;
        }

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

    for domain_identifier in E::SCHEMA.domain_identifiers {
        if RESERVED_NAMES.contains(domain_identifier) {
            panic!("Domain identifier name {domain_identifier} is reserved. Please use a different name.");
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
    domain_identifier: &str,
) -> Result<(), Error> {
    sqlx::query(&format!(
        "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS {domain_identifier} TEXT"
    ))
    .execute(pool)
    .await?;

    sqlx::query(&format!(
        "CREATE INDEX IF NOT EXISTS idx_{table}_{domain_identifier} ON {table}({domain_identifier})"
    ))
    .execute(pool)
    .await?;
    Ok(())
}
