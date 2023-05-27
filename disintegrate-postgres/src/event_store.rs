//! PostgreSQL Event Store
//!
//! This module provides an implementation of the `EventStore` trait using PostgreSQL as the underlying storage.
//! It allows storing and retrieving events from a PostgreSQL database.
mod sql_criteria_builder;
#[cfg(test)]
mod tests;

use sql_criteria_builder::SqlEventsCriteriaBuilder;

use std::marker::PhantomData;

use crate::Error;
use async_stream::stream;
use async_trait::async_trait;
use disintegrate::stream_query::StreamQuery;
use disintegrate::EventStore;
use disintegrate::{Event, PersistedEvent};
use disintegrate_serde::Serde;
use futures::{stream::BoxStream, StreamExt};
use sqlx::{types::Json, PgPool};

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
{
    /// Creates a new instance of `PgEventStore`.
    ///
    /// # Arguments
    ///
    /// * `pool` - The PostgreSQL connection pool.
    /// * `serde` - The serialization implementation for the event payload.
    pub fn new(pool: PgPool, serde: S) -> Self {
        Self {
            pool,
            serde,
            event_type: PhantomData,
        }
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
    ) -> Result<BoxStream<PersistedEvent<QE>>, Self::Error>
    where
        QE: TryFrom<E> + Event + Send + Sync + Clone,
        <QE as TryFrom<E>>::Error: std::fmt::Debug + Send,
    {
        let sql_criteria = SqlEventsCriteriaBuilder::new(query).build();
        let sql = format!("SELECT id, payload FROM event WHERE {sql_criteria} ORDER BY id ASC");

        Ok(stream! {
            for await row in sqlx::query_as::<_, (i64, Vec<u8>)>(&sql)
            .fetch(&self.pool) {
                let row = row.unwrap();
                let id = row.0;
                let payload = self.serde.deserialize(row.1).unwrap();
                yield PersistedEvent::new(id, payload.try_into().unwrap());
            }
        }
        .boxed())
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
        QE: Event + Clone + Send,
    {
        let mut persisted_events = Vec::new();
        for event in events {
            let (id,) = sqlx::query_as::<_, (i64,)>(
                "INSERT INTO event_sequence (event_type, domain_identifiers)
                VALUES ($1, $2)
                RETURNING id",
            )
            .bind(event.name())
            .bind(Json(event.domain_identifiers()))
            .fetch_one(&self.pool)
            .await?;
            persisted_events.push(PersistedEvent::new(id, event));
        }

        let mut tx = self.pool.begin().await?;
        for event in &persisted_events {
            sqlx::query(
                "INSERT INTO event (id, event_type, domain_identifiers, payload)
                VALUES ($1, $2, $3, $4)",
            )
            .bind(event.id())
            .bind(event.name())
            .bind(Json(event.domain_identifiers()))
            .bind(self.serde.serialize((**event).clone()))
            .execute(&mut tx)
            .await?;
        }
        let last_persisted_event_id = persisted_events
            .last()
            .map(|e| e.id())
            .unwrap_or(last_event_id);
        let update_state_criteria = SqlEventsCriteriaBuilder::new(&query)
            .with_origin(last_event_id + 1)
            .with_last_event_id(last_persisted_event_id)
            .build();
        sqlx::query(&format!(
            "UPDATE event_sequence SET consumed = consumed + 1 where {update_state_criteria}"
        ))
        .execute(&mut tx)
        .await
        .map_err(map_update_event_id_err)?;
        tx.commit().await?;

        Ok(persisted_events)
    }
}

pub async fn setup(pool: &PgPool) -> Result<(), Error> {
    sqlx::query(include_str!("event_store/sql/table_event.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!("event_store/sql/idx_event_type.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!(
        "event_store/sql/idx_event_domain_identifiers.sql"
    ))
    .execute(pool)
    .await?;
    sqlx::query(include_str!("event_store/sql/table_event_sequence.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!("event_store/sql/idx_event_sequence_type.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!(
        "event_store/sql/idx_event_sequence_domain_identifiers.sql"
    ))
    .execute(pool)
    .await?;
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
