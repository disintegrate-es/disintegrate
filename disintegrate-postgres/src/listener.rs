//! PostgreSQL Event Listener
//!
//! This module provides an implementation of a PostgreSQL event listener.
//! It allows listening events when they are persisted in the event store.
//! It assures that the events are delivered at least once, so the implementation
//! of the `EventListener` trait should handle duplicated events delivery in case of failures.
#[cfg(test)]
mod tests;

pub(crate) mod id_indexer;

use crate::{Error, Migrator, PgEventId};
use async_trait::async_trait;
use disintegrate::{Event, EventListener, StreamItem, StreamQuery};
use disintegrate_serde::Serde;
use futures::future::join_all;
use futures::{try_join, Future, StreamExt};
use sqlx::{Postgres, Row, Transaction};
use std::error::Error as StdError;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::event_store::PgEventStore;

/// PostgreSQL event listener implementation.
pub struct PgEventListener<E, S>
where
    E: Event + Clone,
    S: Serde<E> + Send + Sync,
{
    executors: Vec<Box<dyn EventListenerExecutor<E> + Send + Sync>>,
    event_store: PgEventStore<E, S>,
    intialize: bool,
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
            intialize: true,
        }
    }

    /// Marks the event listener as uninitialized, indicating that the database setup is already
    /// done.
    ///
    /// This method sets the `initialize` flag to `false`. When the flag is unset, the listener will not
    /// initialize the database. If you set `initialize` to `false`, you must ensure that the
    /// database is initialized before running the listener. Check the SQL files in the `listener/sql` folder
    /// to initialize the database.
    ///
    /// # Returns
    ///
    /// The updated `PgEventListener` instance with the `uninitialized` flag set.
    pub fn uninitialized(mut self) -> Self {
        self.intialize = false;
        self
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
    pub fn register_listener<QE, L>(
        mut self,
        event_listener: L,
        config: PgEventListenerConfig<impl Retry<L::Error> + 'static>,
    ) -> Self
    where
        L: EventListener<PgEventId, QE> + 'static,
        QE: TryFrom<E> + Into<E> + Event + Send + Sync + Clone + 'static,
        <QE as TryFrom<E>>::Error: StdError + Send + Sync,
        L::Error: Send + Sync + 'static,
    {
        self.executors.push(Box::new(PgEventListerExecutor::new(
            self.event_store.clone(),
            event_listener,
            self.shutdown_token.clone(),
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
        if self.intialize {
            Migrator::new(self.event_store.clone())
                .init_listener()
                .await?;
        }
        let mut handles = vec![];
        let mut wakers = vec![];
        for executor in self.executors {
            executor.init().await?;
            let (waker, task) = executor.run();
            if let Some(waker) = waker {
                wakers.push(waker);
            }
            handles.push(task);
        }
        if !wakers.is_empty() {
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
                                    Ok(Some(notification)) => {
                                        for waker in &wakers {
                                            waker.wake(notification.payload());
                                        }
                                    },
                                    Ok(None) => {},
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

/// Represents the different error kinds that can occur in the event listener lifecycle.
pub enum PgEventListenerErrorKind<HE> {
    /// Error occurred while initializing the database transaction for the event listener.
    InitTransaction { source: Error },
    /// Error occurred while acquiring a lock for the event listener in the database.
    AquireLock { source: Error },
    /// Error occurred while fetching the next event from the event store.
    /// Contains the last successfully processed event ID.
    FetchNextEvent {
        source: Error,
        last_processed_event_id: PgEventId,
    },
    /// Error occurred in the user-provided event handler.
    /// Contains the last successfully processed event ID and the handler's error.
    Handler {
        source: HE,
        last_processed_event_id: PgEventId,
    },
    /// Error occurred while releasing the lock or updating the last processed event ID in the database.
    ReleaseLock {
        source: Error,
        last_processed_event_id: PgEventId,
    },
}

#[derive(Debug)]
/// Error type for the event listener, parameterized by the handler error type.
pub struct PgEventListenerError<HE> {
    pub kind: PgEventListenerErrorKind<HE>,
    pub listener_id: String,
}

/// Decision returned by a retry policy for error handling.
pub enum RetryDecision {
    /// Stop the event listner.
    Abort,
    /// Wait for the specified duration before retrying again.
    Wait { duration: Duration },
}

/// Trait for implementing retry policies for event listener errors.
pub trait Retry<HE>: Clone + Send + Sync {
    fn retry(&self, error: &PgEventListenerError<HE>, attempts: usize) -> RetryDecision;
}

/// PostgreSQL listener Configuration.
///
/// # Properties:
///
/// * `poll`: The `poll` property represents the interval at which the
///   listener should poll for new events from the event store. This determines how frequently the
///   event handler will handles new events.
/// * `notifier_enabled`: The `notifier_enabled` indicates if the listener is configured to handle events in "real time".
#[derive(Clone)]
pub struct PgEventListenerConfig<R> {
    poll: Duration,
    fetch_size: usize,
    notifier_enabled: bool,
    retry: R,
}

impl PgEventListenerConfig<AbortRetry> {
    /// Creates a new `PgEventListenerConfig` with the specified poll interval.
    ///
    /// # Parameters
    ///
    /// * `poll`: The poll interval.
    ///
    /// # Returns
    ///
    /// A new `PgEventListenerConfig` instance.
    pub fn poller(poll: Duration) -> PgEventListenerConfig<AbortRetry> {
        PgEventListenerConfig {
            poll,
            fetch_size: usize::MAX,
            notifier_enabled: false,
            retry: AbortRetry,
        }
    }
}

impl<R> PgEventListenerConfig<R> {
    /// Sets the fetch size for the event listener.
    /// The fetch size determines the number of events to fetch from the event store at a time.
    ///
    /// # Parameters
    ///
    /// * `fetch_size`: The number of events to fetch from the event store at a time.
    ///
    /// # Returns
    ///
    /// A new `PgEventListenerConfig` instance.
    pub fn fetch_size(mut self, fetch_size: usize) -> Self {
        self.fetch_size = fetch_size;
        self
    }

    /// Sets the db notifier.
    ///
    /// # Returns
    ///
    /// The updated `PgEventListenerConfig` instance with the db notifier set.
    /// When the db notifier is enabled, the event listener will handle events in "real time".
    pub fn with_notifier(mut self) -> Self {
        self.notifier_enabled = true;
        self
    }

    /// Sets the retry policy for the event listener.
    ///
    /// # Parameters
    /// * `retry`: The retry policy to use.
    ///
    /// # Returns
    /// The updated `PgEventListenerConfig` instance with the retry policy set.
    pub fn with_retry(mut self, retry: R) -> Self {
        self.retry = retry;
        self
    }
}
/// A retry policy that always aborts on error.
#[derive(Clone, Default)]
pub struct AbortRetry;

impl<HE> Retry<HE> for AbortRetry {
    fn retry(&self, _error: &PgEventListenerError<HE>, _attempts: usize) -> RetryDecision {
        RetryDecision::Abort
    }
}

#[async_trait]
trait EventListenerExecutor<E: Event + Clone> {
    async fn init(&self) -> Result<(), Error>;
    fn run(&self) -> (Option<ExecutorWaker<E>>, JoinHandle<Result<(), Error>>);
}

/// Executor for a registered event listener, handling polling, notification, and error management.
struct PgEventListerExecutor<L, QE, E, S, R>
where
    QE: TryFrom<E> + Event + Send + Sync + Clone,
    <QE as TryFrom<E>>::Error: Send + Sync,
    E: Event + Clone + Sync + Send,
    S: Serde<E> + Clone + Send + Sync,
    L: EventListener<PgEventId, QE>,
    R: Retry<L::Error>,
    L::Error: Send + Sync + 'static,
{
    event_store: PgEventStore<E, S>,
    event_handler: Arc<L>,
    config: PgEventListenerConfig<R>,
    wake_channel: (watch::Sender<bool>, watch::Receiver<bool>),
    shutdown_token: CancellationToken,
    _event_store_events: PhantomData<E>,
    _event_listener_events: PhantomData<QE>,
}

impl<L, QE, E, S, R> PgEventListerExecutor<L, QE, E, S, R>
where
    E: Event + Clone + Sync + Send + 'static,
    S: Serde<E> + Clone + Send + Sync + 'static,
    QE: TryFrom<E> + Event + 'static + Send + Sync + Clone,
    <QE as TryFrom<E>>::Error: StdError + 'static + Send + Sync,
    L: EventListener<PgEventId, QE> + 'static,
    R: Retry<L::Error> + 'static,
    L::Error: Send + Sync + 'static,
{
    /// Creates a new executor for the given event handler and configuration.
    pub fn new(
        event_store: PgEventStore<E, S>,
        event_handler: L,
        shutdown_token: CancellationToken,
        config: PgEventListenerConfig<R>,
    ) -> Self {
        Self {
            event_store,
            event_handler: Arc::new(event_handler),
            config,
            wake_channel: watch::channel(true),
            shutdown_token,
            _event_store_events: PhantomData,
            _event_listener_events: PhantomData,
        }
    }

    /// Attempts to acquire a lock for this listener in the database, returning the last processed event ID if successful.
    async fn acquire_listener(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Option<PgEventId>, sqlx::Error> {
        Ok(sqlx::query("SELECT last_processed_event_id FROM event_listener WHERE id = $1 FOR UPDATE SKIP LOCKED")
        .bind(self.event_handler.id())
        .fetch_optional(&mut **tx)
        .await?
        .map(|r| r.get(0)))
    }

    /// Releases the lock for this listener and updates the last processed event ID in the database.
    async fn release_listener(
        &self,
        result: Result<PgEventId, PgEventListenerError<L::Error>>,
        mut tx: Transaction<'_, Postgres>,
    ) -> Result<(), PgEventListenerError<L::Error>> {
        let last_processed_event_id = match result {
            Ok(last_processed_event_id) => last_processed_event_id,
            Err(PgEventListenerError {
                kind:
                    PgEventListenerErrorKind::FetchNextEvent {
                        last_processed_event_id,
                        ..
                    },
                ..
            })
            | Err(PgEventListenerError {
                kind:
                    PgEventListenerErrorKind::Handler {
                        last_processed_event_id,
                        ..
                    },
                ..
            }) => last_processed_event_id,
            Err(e) => Err(e)?,
        };
        sqlx::query(
            "UPDATE event_listener SET last_processed_event_id = $1, updated_at = now() WHERE id = $2",
        )
        .bind(last_processed_event_id)
        .bind(self.event_handler.id())
        .execute(&mut *tx)
        .await.map_err(|e| PgEventListenerError::<L::Error>{
            kind: PgEventListenerErrorKind::ReleaseLock {
                source: e.into(),
                last_processed_event_id
            },
            listener_id: self.event_handler.id().to_string(),
        })?;
        tx.commit()
            .await
            .map_err(|e| PgEventListenerError::<L::Error> {
                kind: PgEventListenerErrorKind::ReleaseLock {
                    source: e.into(),
                    last_processed_event_id,
                },
                listener_id: self.event_handler.id().to_string(),
            })?;
        Ok(())
    }

    /// Handles events from the event store, starting from the given event ID.
    async fn handle_events_from(
        &self,
        mut last_processed_event_id: PgEventId,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<PgEventId, PgEventListenerError<L::Error>> {
        let query = self
            .event_handler
            .query()
            .clone()
            .change_origin(last_processed_event_id);

        let mut stream = self
            .event_store
            .stream_with(&mut **tx, &query)
            .take(self.config.fetch_size);

        while let Some(item) = stream.next().await {
            let item = item.map_err(|e| PgEventListenerError::<L::Error> {
                kind: PgEventListenerErrorKind::FetchNextEvent {
                    source: e,
                    last_processed_event_id,
                },
                listener_id: self.event_handler.id().to_string(),
            })?;

            let event_id = item.id();

            match item {
                StreamItem::End(_) => {
                    last_processed_event_id = event_id;
                    break;
                }
                StreamItem::Event(event) => {
                    self.event_handler
                        .handle(event)
                        .await
                        .map_err(|e| PgEventListenerError {
                            kind: PgEventListenerErrorKind::Handler {
                                source: e,
                                last_processed_event_id,
                            },
                            listener_id: self.event_handler.id().to_string(),
                        })?;

                    last_processed_event_id = event_id;
                }
            }
            if self.shutdown_token.is_cancelled() {
                break;
            }
        }

        Ok(last_processed_event_id)
    }

    /// Attempts to execute the event handling logic once, with error mapping and lock management.
    pub async fn try_execute(&self) -> Result<(), PgEventListenerError<L::Error>> {
        let mut tx = self
            .event_store
            .pool
            .begin()
            .await
            .map_err(|e| PgEventListenerError {
                kind: PgEventListenerErrorKind::InitTransaction { source: e.into() },
                listener_id: self.event_handler.id().to_string(),
            })?;
        let Some(last_processed_id) =
            self.acquire_listener(&mut tx)
                .await
                .map_err(|e| PgEventListenerError {
                    kind: PgEventListenerErrorKind::AquireLock { source: e.into() },
                    listener_id: self.event_handler.id().to_string(),
                })?
        else {
            return Ok(()); // Another instance is processing
        };
        let result = self.handle_events_from(last_processed_id, &mut tx).await;

        self.release_listener(result, tx).await
    }

    /// Executes the event handler with retry logic according to the configured policy.
    async fn execute(&self) -> Result<(), Error> {
        let mut attempts = 0;
        loop {
            match self.try_execute().await {
                Ok(_) => break,
                Err(err) => match self.config.retry.retry(&err, attempts) {
                    RetryDecision::Abort => break,
                    RetryDecision::Wait { duration } => {
                        attempts += 1;
                        tokio::time::sleep(duration).await;
                    }
                },
            }
        }
        Ok(())
    }

    /// Spawns the event handler as a background task, polling and/or waiting for notifications.
    pub fn spawn_task(self) -> JoinHandle<Result<(), Error>> {
        let shutdown = self.shutdown_token.clone();
        let mut poll = tokio::time::interval(self.config.poll);
        poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut wake_tx = self.wake_channel.1.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Ok(()) =  wake_tx.changed() => self.execute().await?,
                    _ = poll.tick() => self.execute().await?,
                    _ = shutdown.cancelled() => return Ok::<(), Error>(()),
                };
            }
        })
    }
}

#[async_trait]
impl<L, QE, E, S, R> EventListenerExecutor<E> for PgEventListerExecutor<L, QE, E, S, R>
where
    E: Event + Clone + Sync + Send + 'static,
    S: Serde<E> + Clone + Send + Sync + 'static,
    QE: TryFrom<E> + Into<E> + Event + 'static + Send + Sync + Clone,
    <QE as TryFrom<E>>::Error: StdError + 'static + Send + Sync,
    L: EventListener<PgEventId, QE> + 'static,
    R: Retry<L::Error> + 'static,
    L::Error: Send + Sync + 'static,
{
    async fn init(&self) -> Result<(), Error> {
        let mut tx = self.event_store.pool.begin().await?;
        sqlx::query("INSERT INTO event_listener (id, last_processed_event_id) VALUES ($1, 0) ON CONFLICT (id) DO NOTHING")
                .bind(self.event_handler.id())
                .execute(&mut *tx)
                .await?;
        tx.commit().await?;
        Ok(())
    }

    fn run(&self) -> (Option<ExecutorWaker<E>>, JoinHandle<Result<(), Error>>) {
        let waker = if self.config.notifier_enabled {
            Some(ExecutorWaker {
                wake_tx: self.wake_channel.0.clone(),
                query: self.event_handler.query().cast().clone(),
            })
        } else {
            None
        };
        (waker, self.clone().spawn_task())
    }
}

impl<L, QE, E, S, R> Clone for PgEventListerExecutor<L, QE, E, S, R>
where
    QE: TryFrom<E> + Event + Send + Sync + Clone,
    <QE as TryFrom<E>>::Error: Send + Sync,
    E: Event + Clone + Sync + Send,
    S: Serde<E> + Clone + Send + Sync,
    L: EventListener<PgEventId, QE>,
    R: Retry<L::Error>,
    L::Error: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            event_store: self.event_store.clone(),
            event_handler: Arc::clone(&self.event_handler),
            config: self.config.clone(),
            wake_channel: self.wake_channel.clone(),
            shutdown_token: self.shutdown_token.clone(),
            _event_store_events: PhantomData,
            _event_listener_events: PhantomData,
        }
    }
}

/// Waker for executor, used to notify about new events matching a query.
struct ExecutorWaker<E: Event + Clone> {
    wake_tx: watch::Sender<bool>,
    query: StreamQuery<PgEventId, E>,
}

impl<E: Event + Clone> ExecutorWaker<E> {
    /// Wakes the executor if the event matches the query.
    fn wake(&self, event: &str) {
        if self.query.matches_event(event) {
            self.wake_tx.send_replace(true);
        }
    }
}
