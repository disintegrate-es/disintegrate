//! PostgreSQL Event Store
//!
//! This module provides an implementation of the `EventStore` trait using PostgreSQL as the underlying storage.
//! It allows storing and retrieving events from a PostgreSQL database.
mod append;
mod query;
#[cfg(test)]
mod tests;

use append::InsertEventsBuilder;
use futures::stream::BoxStream;
use query::CriteriaBuilder;
use sqlx::postgres::PgPool;
use sqlx::Row;
use std::error::Error as StdError;

use std::marker::PhantomData;

use crate::{Error, Migrator, PgEventId};
use async_stream::stream;
use async_trait::async_trait;
use disintegrate::EventStore;
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
    serde: S,
    event_type: PhantomData<E>,
}

impl<E, S> PgEventStore<E, S>
where
    S: Serde<E> + Send + Sync + Clone,
    E: Event + Clone,
{
    /// Initializes the PostgreSQL DB and returns a new instance of `PgEventStore`.
    ///
    /// # Arguments
    ///
    /// * `pool` - The PostgreSQL connection pool.
    /// * `serde` - The serialization implementation for the event payload.
    pub async fn try_new(pool: PgPool, serde: S) -> Result<Self, Error> {
        let event_store = Self::new_uninitialized(pool, serde);
        Migrator::new(event_store.clone())
            .init_event_store()
            .await?;
        Ok(event_store)
    }
    /// Creates a new instance of `PgEventStore`.
    ///
    /// This constructor does not initialize the database or add the
    /// `domain_id` columns necessary for `disintegrate` to function properly.
    /// If you need to initialize the database, use `PgEventStore::new` instead.
    ///
    /// If you plan to use this constructor, ensure that the `disintegrate` is
    /// properly initialized. Refer to the SQL files in the "event_store/sql" directory
    /// to recreate the default structure. Additionally, all `domain_id` columns
    /// and their corresponding indexes must be created manually.
    ///
    /// # Arguments
    ///
    /// * `pool` - The PostgreSQL connection pool.
    /// * `serde` - The serialization implementation for the event payload.
    pub fn new_uninitialized(pool: PgPool, serde: S) -> Self {
        Self {
            pool,
            serde,
            event_type: PhantomData,
        }
    }
}

impl<E, S> PgEventStore<E, S>
where
    S: Serde<E> + Send + Sync,
    E: Event + Send + Sync,
{
    /// Streams events based on the provided query and executor.
    ///
    /// # Arguments
    ///
    /// * `executor` - The sqlx executor to use for database operations.
    /// * `query` - The stream query specifying the criteria for filtering events.
    ///
    /// # Returns
    ///
    /// A `Result` containing a boxed stream of `PersistedEvent` that matches the query criteria,
    /// or an error of type `Error`.
    pub(crate) fn stream_with<'a, QE, EX>(
        &'a self,
        executor: EX,
        query: &'a StreamQuery<PgEventId, QE>,
    ) -> BoxStream<'a, Result<StreamItem<PgEventId, QE>, Error>>
    where
        EX: sqlx::PgExecutor<'a> + Send + Sync + 'a,
        QE: TryFrom<E> + Event + Clone + Send + Sync + 'static,
        <QE as TryFrom<E>>::Error: StdError + Send + Sync + 'static,
    {
        let sql = format!(
            r#"SELECT event.event_id, event.payload, epoch.__epoch_id
            FROM (SELECT COALESCE(MAX(event_id),0) AS __epoch_id FROM event) AS epoch
            LEFT JOIN event ON event.event_id <= epoch.__epoch_id AND ({criteria})
            ORDER BY event_id ASC"#,
            criteria = CriteriaBuilder::new(query).build()
        );

        stream! {
            let mut rows = sqlx::query(&sql).fetch(executor);
            let mut epoch_id: PgEventId = 0;
            while let Some(row) = rows.next().await {
                let row = row?;
                let event_id: Option<i64> = row.get(0);
                epoch_id = row.get(2);
                if let Some(event_id) = event_id {
                    let payload = self.serde.deserialize(row.get(1))?;
                    let payload: QE = payload
                        .try_into()
                        .map_err(|e| Error::QueryEventMapping(Box::new(e)))?;
                    yield Ok(StreamItem::Event(PersistedEvent::new(event_id, payload)));
                }
            }
            yield Ok(StreamItem::End(epoch_id));
        }
        .boxed()
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
        self.stream_with(&self.pool, query)
    }

    /// Appends new events to the event store.
    ///
    /// This function inserts the provided `events` into the PostgreSQL-backed event store.
    /// Before inserting, it queries the `event` table to ensure that no events have been
    /// appended since the given `version`. If newer events are found, a concurrency error
    /// is returned to prevent invalid state transitions.
    ///
    /// If the concurrency check succeeds, the events are inserted into the `event` table.
    ///
    /// # Arguments
    ///
    /// * `events` - The events to append to the event store.
    /// * `query` - The stream query that identifies the target event stream.
    /// * `version` - The ID of the last consumed event, used for optimistic concurrency control.
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
        sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
            .execute(&mut *tx)
            .await?;

        if sqlx::query_scalar(&format!(
            "SELECT EXISTS (SELECT 1 FROM event WHERE {})",
            CriteriaBuilder::new(&query.change_origin(version)).build()
        ))
        .fetch_one(&mut *tx)
        .await?
        {
            return Err(Error::Concurrency);
        }

        let mut insert = InsertEventsBuilder::new(&events, &self.serde);
        let event_ids: Vec<PgEventId> = insert
            .build()
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|r| r.get(0))
            .collect();

        let persisted_events = event_ids
            .iter()
            .zip(events)
            .map(|(event_id, event)| PersistedEvent::new(*event_id, event))
            .collect::<Vec<_>>();

        tx.commit().await.map_err(map_concurrency_err)?;

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

        let mut insert = InsertEventsBuilder::new(&events, &self.serde);
        let event_ids: Vec<PgEventId> = insert
            .build()
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|r| r.get(0))
            .collect();

        let persisted_events = event_ids
            .iter()
            .zip(events)
            .map(|(event_id, event)| PersistedEvent::new(*event_id, event))
            .collect::<Vec<_>>();

        tx.commit().await?;

        Ok(persisted_events)
    }
}

fn map_concurrency_err(err: sqlx::Error) -> Error {
    if let sqlx::Error::Database(ref description) = err {
        if description.code().as_deref() == Some("40001") {
            return Error::Concurrency;
        }
    }
    Error::Database(err)
}
