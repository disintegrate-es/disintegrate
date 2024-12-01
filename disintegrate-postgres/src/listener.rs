//! PostgreSQL Event Listener
//!
//! This module provides an implementation of a PostgreSQL event listener.
//! It allows listening events when they are persisted in the event store.
//! It assures that the events are delivered at least once, so the implementation
//! of the `EventListener` trait should handle duplicated events delivery in case of failures.
#[cfg(test)]
mod tests;

use crate::{Error, PgEventId};
use async_trait::async_trait;
use disintegrate::{Event, EventListener, EventStore};
use disintegrate_serde::Serde;
use futures::future::join_all;
use futures::{try_join, Future, StreamExt};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::error::Error as StdError;
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
        event_listener: impl EventListener<PgEventId, QE> + 'static,
        config: PgEventListenerConfig,
    ) -> Self
    where
        QE: TryFrom<E> + Event + Send + Sync + Clone + 'static,
        <QE as TryFrom<E>>::Error: StdError + Send + Sync,
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
        setup(&self.event_store.pool).await?;
        let mut handles = vec![];
        let listener_enabled = self.executors.iter().any(|e| e.config().listener_enabled());
        let (events_listener_tx, events_listener_rx) = tokio::sync::watch::channel(true);
        if listener_enabled {
            let pool = self.event_store.pool.clone();
            let shutdown = self.shutdown_token.clone();
            let watch_new_events = tokio::spawn(async move {
                loop {
                    let mut listener = sqlx::postgres::PgListener::connect_with(&pool).await?;
                    listener.listen("new_events").await?;
                    loop {
                        tokio::select! {
                            msg = listener.try_recv() => {
                                match msg {
                                    Ok(_) => { events_listener_tx.send_replace(true); },
                                    Err(err @ sqlx::Error::PoolClosed) => return Err(Error::Database(err)),
                                    Err(_) => break,
                                }
                            }
                            _ = shutdown.cancelled() => return Ok::<(), Error>(()),
                        }
                    }
                }
            });
            handles.push(watch_new_events);
        }
        for executor in self.executors {
            executor.init().await?;
            let executor = executor.clone();
            let shutdown = self.shutdown_token.clone();
            let mut events_listener_rx = events_listener_rx.clone();
            let listener_enabled = executor.config().listener_enabled();
            let handle = tokio::spawn(async move {
                let mut poll = tokio::time::interval(executor.config().poll());
                poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    tokio::select! {
                        Some(Ok(())) = async { if listener_enabled {  let res = events_listener_rx.changed().await; Some(res) } else { None } } => executor.execute().await?,
                        _ = poll.tick() => executor.execute().await?,
                        _ = shutdown.cancelled() => return Ok::<(), Error>(()),
                    };
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
pub struct PgEventListenerError {
    last_processed_event_id: PgEventId,
}

/// PostgreSQL listener Configuration
///
/// # Properties:
///
/// * `poll`: The `poll` property represents the interval at which the
///   listener should poll for new events from the event store. This determines how frequently the
///   event handler will handles new events.
/// * `batch_size`: The `batch_size` property determines the number of events to be processed in a single batch.
#[derive(Clone)]
pub struct PgEventListenerConfig {
    poll: Duration,
    batch_size: usize,
    listener_enabled: bool,
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
            batch_size: 100,
            listener_enabled: false,
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

    /// Sets the db listener.
    ///
    /// # Returns
    ///
    /// The updated `PgEventListenerConfig` instance with the db listener set.
    pub fn with_listener(mut self) -> Self {
        self.listener_enabled = true;
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

    /// Returns the batch size for processing events.
    ///
    /// # Returns
    ///
    /// The batch size.
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Returns if the db listener is enabled or disabled.
    ///
    /// # Returns
    ///
    /// True if the db listener is enabled, false otherwise.
    pub fn listener_enabled(&self) -> bool {
        self.listener_enabled
    }
}

#[async_trait]
trait EventListenerExecutor {
    fn config(&self) -> &PgEventListenerConfig;
    async fn init(&self) -> Result<(), Error>;
    async fn execute(&self) -> Result<(), Error>;
}

struct PgEventListerExecutor<L, QE, E, S>
where
    QE: TryFrom<E> + Event + Send + Sync + Clone,
    <QE as TryFrom<E>>::Error: Send + Sync,
    E: Event + Clone + Sync + Send,
    S: Serde<E> + Clone + Send + Sync,
    L: EventListener<PgEventId, QE>,
{
    event_store: PgEventStore<E, S>,
    event_handler: L,
    config: PgEventListenerConfig,
    _event_store_events: PhantomData<E>,
    _event_listener_events: PhantomData<QE>,
}

impl<L, QE, E, S> PgEventListerExecutor<L, QE, E, S>
where
    E: Event + Clone + Sync + Send,
    S: Serde<E> + Clone + Send + Sync,
    QE: TryFrom<E> + Event + 'static + Send + Sync + Clone,
    <QE as TryFrom<E>>::Error: StdError + 'static + Send + Sync,
    L: EventListener<PgEventId, QE>,
{
    pub fn new(
        event_store: PgEventStore<E, S>,
        event_handler: L,
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

    async fn lock_event_listener(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Option<PgEventId>, sqlx::Error> {
        Ok(sqlx::query(
            r#"
                SELECT last_processed_event_id 
                FROM event_listener
                WHERE id = $1  
                FOR UPDATE SKIP LOCKED 
                "#,
        )
        .bind(self.event_handler.id())
        .fetch_optional(&mut **tx)
        .await?
        .map(|r| r.get(0)))
    }

    async fn release_event_listener(
        &self,
        result: Result<PgEventId, PgEventListenerError>,
        mut tx: Transaction<'_, Postgres>,
    ) -> Result<(), sqlx::Error> {
        let last_processed_event_id = match result {
            Ok(last_processed_event_id) => last_processed_event_id,
            Err(PgEventListenerError {
                last_processed_event_id,
            }) => last_processed_event_id,
        };
        sqlx::query(
            "UPDATE event_listener SET last_processed_event_id = $1, updated_at = now() WHERE id = $2",
        )
        .bind(last_processed_event_id)
        .bind(self.event_handler.id())
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    }

    pub async fn handle_events_from(
        &self,
        mut last_processed_event_id: PgEventId,
    ) -> Result<PgEventId, PgEventListenerError> {
        let query = self
            .event_handler
            .query()
            .clone()
            .change_origin(last_processed_event_id);
        let mut events_stream = self.event_store.stream(&query).take(self.config.batch_size);

        while let Some(event) = events_stream.next().await {
            let event = event.map_err(|_err| PgEventListenerError {
                last_processed_event_id,
            })?;
            let event_id = event.id();
            match self.event_handler.handle(event).await {
                Ok(_) => last_processed_event_id = event_id,
                Err(_) => {
                    return Err(PgEventListenerError {
                        last_processed_event_id,
                    })
                }
            }
        }

        Ok(last_processed_event_id)
    }

    pub async fn try_execute(&self) -> Result<(), sqlx::Error> {
        let mut tx = self.event_store.pool.begin().await?;
        let Some(last_processed_id) = self.lock_event_listener(&mut tx).await? else {
            return Ok(());
        };
        let result = self.handle_events_from(last_processed_id).await;
        self.release_event_listener(result, tx).await
    }
}

#[async_trait]
impl<L, QE, E, S> EventListenerExecutor for PgEventListerExecutor<L, QE, E, S>
where
    E: Event + Clone + Sync + Send,
    S: Serde<E> + Clone + Send + Sync,
    QE: TryFrom<E> + Event + 'static + Send + Sync + Clone,
    <QE as TryFrom<E>>::Error: StdError + 'static + Send + Sync,
    L: EventListener<PgEventId, QE>,
{
    fn config(&self) -> &PgEventListenerConfig {
        &self.config
    }

    async fn init(&self) -> Result<(), Error> {
        let mut tx = self.event_store.pool.begin().await?;
        sqlx::query("INSERT INTO event_listener (id, last_processed_event_id) VALUES ($1, 0) ON CONFLICT (id) DO NOTHING")
                .bind(self.event_handler.id())
                .execute(&mut *tx)
                .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn execute(&self) -> Result<(), Error> {
        let result = self.try_execute().await;
        match result {
            Err(sqlx::Error::Io(_)) | Err(sqlx::Error::PoolTimedOut) => Ok(()),
            Err(err) => Err(Error::Database(err)),
            _ => Ok(()),
        }
    }
}

async fn setup(pool: &PgPool) -> Result<(), Error> {
    sqlx::query(include_str!("listener/sql/table_event_listener.sql"))
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
