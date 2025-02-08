//! PostgreSQL Event Listener
//!
//! This module provides an implementation of a PostgreSQL event listener.
//! It allows listening events when they are persisted in the event store.
//! It assures that the events are delivered at least once, so the implementation
//! of the `EventListener` trait should handle duplicated events delivery in case of failures.
#[cfg(test)]
mod tests;

pub(crate) mod id_indexer;

use crate::{Error, PgEventId};
use async_trait::async_trait;
use disintegrate::{Event, EventListener, EventStore, StreamQuery};
use disintegrate_serde::Serde;
use futures::future::join_all;
use futures::{try_join, Future, StreamExt};
use sqlx::{PgPool, Postgres, Row, Transaction};
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
    executors: Vec<Box<dyn EventListenerExecutor<E>>>,
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
    pub fn register_listener<QE>(
        mut self,
        event_listener: impl EventListener<PgEventId, QE> + 'static,
        config: PgEventListenerConfig,
    ) -> Self
    where
        QE: TryFrom<E> + Into<E> + Event + Send + Sync + Clone + 'static,
        <QE as TryFrom<E>>::Error: StdError + Send + Sync,
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
            setup(&self.event_store.pool).await?;
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
pub struct PgEventListenerError {
    last_processed_event_id: PgEventId,
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
pub struct PgEventListenerConfig {
    poll: Duration,
    fetch_size: usize,
    notifier_enabled: bool,
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
            fetch_size: usize::MAX,
            notifier_enabled: false,
        }
    }

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
}

#[async_trait]
trait EventListenerExecutor<E: Event + Clone> {
    async fn init(&self) -> Result<(), Error>;
    fn run(&self) -> (Option<ExecutorWaker<E>>, JoinHandle<Result<(), Error>>);
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
    event_handler: Arc<L>,
    config: PgEventListenerConfig,
    wake_channel: (watch::Sender<bool>, watch::Receiver<bool>),
    shutdown_token: CancellationToken,
    _event_store_events: PhantomData<E>,
    _event_listener_events: PhantomData<QE>,
}

impl<L, QE, E, S> PgEventListerExecutor<L, QE, E, S>
where
    E: Event + Clone + Sync + Send + 'static,
    S: Serde<E> + Clone + Send + Sync + 'static,
    QE: TryFrom<E> + Event + 'static + Send + Sync + Clone,
    <QE as TryFrom<E>>::Error: StdError + 'static + Send + Sync,
    L: EventListener<PgEventId, QE> + 'static,
{
    pub fn new(
        event_store: PgEventStore<E, S>,
        event_handler: L,
        shutdown_token: CancellationToken,
        config: PgEventListenerConfig,
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
        let mut events_stream = self.event_store.stream(&query).take(self.config.fetch_size);

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
            if self.shutdown_token.is_cancelled() {
                break;
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

    async fn execute(&self) -> Result<(), Error> {
        let result = self.try_execute().await;
        match result {
            Err(sqlx::Error::Io(_)) | Err(sqlx::Error::PoolTimedOut) => Ok(()),
            Err(err) => Err(Error::Database(err)),
            _ => Ok(()),
        }
    }

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
impl<L, QE, E, S> EventListenerExecutor<E> for PgEventListerExecutor<L, QE, E, S>
where
    E: Event + Clone + Sync + Send + 'static,
    S: Serde<E> + Clone + Send + Sync + 'static,
    QE: TryFrom<E> + Into<E> + Event + 'static + Send + Sync + Clone,
    <QE as TryFrom<E>>::Error: StdError + 'static + Send + Sync,
    L: EventListener<PgEventId, QE> + 'static,
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

impl<L, QE, E, S> Clone for PgEventListerExecutor<L, QE, E, S>
where
    QE: TryFrom<E> + Event + Send + Sync + Clone,
    <QE as TryFrom<E>>::Error: Send + Sync,
    E: Event + Clone + Sync + Send,
    S: Serde<E> + Clone + Send + Sync,
    L: EventListener<PgEventId, QE>,
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

struct ExecutorWaker<E: Event + Clone> {
    wake_tx: watch::Sender<bool>,
    query: StreamQuery<PgEventId, E>,
}

impl<E: Event + Clone> ExecutorWaker<E> {
    fn wake(&self, event: &str) {
        if self.query.matches_event(event) {
            self.wake_tx.send_replace(true);
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
