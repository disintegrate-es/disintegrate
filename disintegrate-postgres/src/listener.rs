//! PostgreSQL Event Listener
//!
//! This module provides an implementation of a PostgreSQL event listener.
//! It allows listening events when they are persisted in the event store.
//! It assures that the events are delivered at least once, so the implementation
//! of the `EventListener` trait should handle duplicated events delivery in case of failures.
#[cfg(test)]
mod tests;

use crate::Error;
use async_trait::async_trait;
use disintegrate::{Event, EventListener, EventStore};
use disintegrate_serde::Serde;
use futures::future::join_all;
use futures::{try_join, Future, StreamExt};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::event_store::PgEventStore;

/// PostgreSQL event listener implementation.
pub struct PgEventListener<E, S>
where
    E: Event + Clone,
    S: Serde<E> + Send + Sync,
{
    executors: Vec<Arc<dyn EventListenerExecutor + Sync + Send>>,
    event_store: PgEventStore<E, S>,
    shutdown_token: CancellationToken,
}

impl<E, S> PgEventListener<E, S>
where
    E: Event + Clone + Send + Sync + 'static,
    S: Serde<E> + Clone + Send + Sync + 'static,
{
    /// Creates a new `PgEventListener` that listens to the events coming from the provided `PgEventStore`
    ///
    /// # Parameters
    ///
    /// * `event_store`: An instance of `PgEventStore` representing the event store for the listener.
    ///
    /// # Returns
    ///
    /// A new `PgEventListener` instance.
    pub fn builder(event_store: PgEventStore<E, S>) -> Self {
        Self {
            event_store,
            executors: vec![],
            shutdown_token: CancellationToken::new(),
        }
    }

    /// Registers an event listener to the `PgEventListener`.
    ///
    /// # Parameters
    ///
    /// * `event_listner`: An implementation of the `EventListener` trait for the specified event type `QE`.
    /// * `config`: A `PgEventListenerConfig` instance representing the configuration for the event listener.
    ///
    /// # Returns
    ///
    /// The updated `PgEventListener` instance with the registered event handler.
    pub fn register_listener<QE>(
        mut self,
        event_listener: impl EventListener<QE> + 'static,
        config: PgEventListenerConfig,
    ) -> Self
    where
        QE: TryFrom<E> + Event + Send + Sync + Clone + 'static,
        <QE as TryFrom<E>>::Error: std::fmt::Debug + Send,
    {
        self.executors.push(Arc::new(PgEventListerExecutor::new(
            self.event_store.clone(),
            event_listener,
            config,
        )));
        self
    }

    /// Starts the listener process for all registered event listeners.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success or failure of the listener process.
    pub async fn start(self) -> Result<(), Error> {
        let mut handles = vec![];
        for executor in self.executors {
            executor.init().await?;
            let executor = executor.clone();
            let pool = self.event_store.pool.clone();
            let shutdown = self.shutdown_token.clone();
            let handle = tokio::spawn(async move {
                let mut poll = tokio::time::interval(executor.config().poll());
                if executor.config().listener_enabled() {
                    let mut listener = sqlx::postgres::PgListener::connect_with(&pool).await?;
                    listener.listen(executor.event_listener_id()).await?;
                    loop {
                        tokio::select! {
                            _ = poll.tick() => executor.execute().await?,
                            msg = listener.try_recv() => {
                                match msg {
                                    Ok(_) => executor.execute().await?,
                                    Err(err @ sqlx::Error::PoolClosed) => return Err(Error::Database(err)),
                                    _ => (),
                                }
                            },
                            _ = shutdown.cancelled() => return Ok(()),
                        };
                    }
                } else {
                    loop {
                        tokio::select! {
                            _ = poll.tick() => executor.execute().await?,
                            _ = shutdown.cancelled() => return Ok(()),
                        };
                    }
                }
            });
            handles.push(handle);
        }
        join_all(handles).await;
        Ok(())
    }

    /// Starts the listener process for all the registered event listeners with a shutdown signal.
    ///
    /// # Parameters
    ///
    /// * `shutdown`: A future that represents the shutdown signal.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success or failure of the listener process.
    pub async fn start_with_shutdown<F: Future<Output = ()> + Send + 'static>(
        self,
        shutdown: F,
    ) -> Result<(), Error> {
        let shutdown_token = self.shutdown_token.clone();
        let shutdown_handle = async move {
            shutdown.await;
            shutdown_token.cancel();
            Ok::<(), Error>(())
        };
        try_join!(self.start(), shutdown_handle).map(|_| ())
    }
}

#[derive(Debug)]
pub enum PgEventListenerError {
    Failed { last_processed_event_id: i64 },
    Query,
}

/// PostgreSQL listener Configuration
///
/// # Properties:
///
/// * `poll`: The `poll` property represents the interval at which the
/// listener should poll for new events from the event store. This determines how frequently the
/// event handler will handles new events.
/// * `listener_enabled`: The `listener_enabled` property determines whether or not the PostgreSQL database listener
/// is enabled. If it is enabled, the listener will receive notifications from the database
/// when an event has been stored
/// * `batch_size`: The `batch_size` property determines the number of events to be processed in a single batch.
#[derive(Clone)]
pub struct PgEventListenerConfig {
    poll: Duration,
    listener_enabled: bool,
    batch_size: usize,
}

impl PgEventListenerConfig {
    /// Creates a new `PgEventListenerConfig` with the specified poll interval.
    ///
    /// # Parameters
    ///
    /// * `poll`: The poll interval.
    ///
    /// # Returns
    ///
    /// A new `PgEventListenerConfig` instance.
    pub fn poller(poll: Duration) -> Self {
        Self {
            poll,
            listener_enabled: false,
            batch_size: 100,
        }
    }

    /// Creates a new `PgEventListenerConfig` with the specified poll interval and enables the PostgreSQL notification listener.
    ///
    /// # Parameters
    ///
    /// * `poll`: The poll interval.
    ///
    /// # Returns
    ///
    /// A new `PgEventListenerConfig` instance with the listener enabled.
    pub fn poller_with_listener(poll: Duration) -> Self {
        Self {
            listener_enabled: true,
            ..Self::poller(poll)
        }
    }

    /// Sets the batch size for processing events in the listener.
    ///
    /// # Parameters
    ///
    /// * `batch_size`: The number of events to process in each batch.
    ///
    /// # Returns
    ///
    /// The updated `PgEventListenerConfig` instance with the batch size set.
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Returns the poll interval for the event listener.
    ///
    /// # Returns
    ///
    /// The poll interval.
    pub fn poll(&self) -> Duration {
        self.poll
    }

    /// Returns whether the Postgres notification is enabled for the event listener.
    ///
    /// # Returns
    ///
    /// `true` if the listener is enabled, `false` otherwise.
    pub fn listener_enabled(&self) -> bool {
        self.listener_enabled
    }

    /// Returns the batch size for processing events.
    ///
    /// # Returns
    ///
    /// The batch size.
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }
}

#[async_trait]
trait EventListenerExecutor {
    fn config(&self) -> &PgEventListenerConfig;
    fn event_listener_id(&self) -> &str;
    async fn init(&self) -> Result<(), Error>;
    async fn execute(&self) -> Result<(), Error>;
}

struct PgEventListerExecutor<L, QE, E, S>
where
    QE: TryFrom<E> + Event + Send + Sync + Clone,
    <QE as TryFrom<E>>::Error: std::fmt::Debug + Send,
    E: Event + Clone + Sync + Send,
    S: Serde<E> + Clone + Send + Sync,
    L: EventListener<QE>,
{
    event_store: PgEventStore<E, S>,
    event_handler: L,
    config: PgEventListenerConfig,
    _event_store_events: PhantomData<E>,
    _event_listener_events: PhantomData<QE>,
}

impl<P, QE, E, S> PgEventListerExecutor<P, QE, E, S>
where
    E: Event + Clone + Sync + Send,
    S: Serde<E> + Clone + Send + Sync,
    QE: TryFrom<E> + Event + Send + Sync + Clone,
    <QE as TryFrom<E>>::Error: std::fmt::Debug + Send,
    P: EventListener<QE>,
{
    pub fn new(
        event_store: PgEventStore<E, S>,
        event_handler: P,
        config: PgEventListenerConfig,
    ) -> Self {
        Self {
            event_store,
            event_handler,
            config,
            _event_store_events: PhantomData,
            _event_listener_events: PhantomData,
        }
    }

    async fn is_out_of_sync(&self, pool: &PgPool) -> Result<bool, sqlx::Error> {
        let (is_out_of_sync,) = sqlx::query_as::<_, (bool,)>(
            r#"
                SELECT (s.last_processed_event_id <> p.last_event_id) 
                FROM event_listener_status s
                JOIN event_listener p on (s.id = p.id) 
                WHERE s.id = $1
                "#,
        )
        .bind(self.event_handler.id())
        .fetch_one(pool)
        .await?;
        Ok(is_out_of_sync)
    }

    async fn lock_event_listener(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Option<i64>, sqlx::Error> {
        Ok(sqlx::query(
            r#"
                SELECT last_processed_event_id 
                FROM event_listener_status 
                WHERE id = $1  
                FOR UPDATE SKIP LOCKED 
                "#,
        )
        .bind(self.event_handler.id())
        .fetch_optional(tx)
        .await?
        .map(|r| r.get(0)))
    }

    async fn release_event_listener(
        &self,
        result: Result<i64, PgEventListenerError>,
        mut tx: Transaction<'_, Postgres>,
    ) -> Result<(), sqlx::Error> {
        let last_processed_event_id = match result {
            Ok(last_processed_event_id) => last_processed_event_id,
            Err(PgEventListenerError::Failed {
                last_processed_event_id,
            }) => last_processed_event_id,
            Err(PgEventListenerError::Query) => return Ok(()),
        };
        sqlx::query(
            "UPDATE event_listener_status SET last_processed_event_id = $1, updated_at = now() WHERE id = $2",
        )
        .bind(last_processed_event_id)
        .bind(self.event_handler.id())
        .execute(&mut tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn handle_events_from(
        &self,
        mut last_processed_event_id: i64,
    ) -> Result<i64, PgEventListenerError> {
        let query = self
            .event_handler
            .query()
            .clone()
            .change_origin(last_processed_event_id + 1);
        let mut events_stream = self
            .event_store
            .stream(&query)
            .map_err(|_| PgEventListenerError::Query)?
            .take(self.config.batch_size);

        while let Some(event) = events_stream.next().await {
            let event_id = event.id();
            match self.event_handler.handle(event).await {
                Ok(_) => last_processed_event_id = event_id,
                Err(_) => {
                    return Err(PgEventListenerError::Failed {
                        last_processed_event_id,
                    })
                }
            }
        }

        Ok(last_processed_event_id)
    }

    pub async fn try_execute(&self) -> Result<(), sqlx::Error> {
        if !self.is_out_of_sync(&self.event_store.pool).await? {
            return Ok(());
        }
        let mut tx = self.event_store.pool.begin().await?;
        let Some(last_processed_id) = self.lock_event_listener(&mut tx).await? else {
            return Ok(());
        };
        let result = self.handle_events_from(last_processed_id).await;
        self.release_event_listener(result, tx).await?;
        Ok(())
    }
}

#[async_trait]
impl<L, QE, E, S> EventListenerExecutor for PgEventListerExecutor<L, QE, E, S>
where
    E: Event + Clone + Sync + Send,
    S: Serde<E> + Clone + Send + Sync,
    QE: TryFrom<E> + Event + Send + Sync + Clone,
    <QE as TryFrom<E>>::Error: std::fmt::Debug + Send,
    L: EventListener<QE>,
{
    fn config(&self) -> &PgEventListenerConfig {
        &self.config
    }

    fn event_listener_id(&self) -> &str {
        self.event_handler.id()
    }

    async fn init(&self) -> Result<(), Error> {
        let mut tx = self.event_store.pool.begin().await?;
        let event_names = E::NAMES;
        sqlx::query("insert into event_listener_status (id, last_processed_event_id) values ($1, -1) on conflict (id) do nothing")
                .bind(self.event_handler.id())
                .execute(&mut tx)
                .await?;
        sqlx::query("insert into event_listener (id, event_types, last_event_id) values ($1, $2,  0) on conflict (id) do update set event_types = $2")
                .bind(self.event_handler.id())
                .bind(event_names)
                .execute(&mut tx)
                .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn execute(&self) -> Result<(), Error> {
        let result = self.try_execute().await;
        if let Err(err) = result {
            match err {
                sqlx::Error::Io(_) | sqlx::Error::PoolTimedOut => Ok(()),
                _ => Err(Error::Database(err)),
            }
        } else {
            Ok(())
        }
    }
}

pub async fn setup(pool: &PgPool) -> Result<(), Error> {
    sqlx::query(include_str!("listener/sql/table_event_listener.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!(
        "listener/sql/idx_event_listener_event_types.sql"
    ))
    .execute(pool)
    .await?;
    sqlx::query(include_str!("listener/sql/table_event_listener_status.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!("listener/sql/fn_update_event_listener.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!("listener/sql/trigger_event_insert.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!("listener/sql/fn_notify_event_listener.sql"))
        .execute(pool)
        .await?;
    sqlx::query(include_str!(
        "listener/sql/trigger_notify_event_listener.sql"
    ))
    .execute(pool)
    .await?;
    Ok(())
}
