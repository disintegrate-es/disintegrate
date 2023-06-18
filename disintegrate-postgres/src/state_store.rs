//! # PostgreSQL State Store
//!
//! This module provides an implementation of a StateStore for a PostgreSQL database.
//! It allows storing and retrieving business concepts, known as states, which are hydrated from event store events.
use crate::Error;
use crate::PgEventStore;
use async_trait::async_trait;
use disintegrate::state::{Hydrated, State, StateStore};
use disintegrate::Event;
use disintegrate::EventStore;
use disintegrate::PersistedEvent;
use disintegrate_serde::Serde;
use futures::StreamExt;

/// Implementation of the `StateStore` trait for a PostgreSQL event store.
///
/// This struct allows hydrating and saving states using a PostgreSQL event store.
/// It requires the event type `E` to implement the `Event` traits.
/// Additionally, it requires the serializer type `Srd` to implement Serde traits.
#[async_trait]
impl<E, Srd> StateStore<E> for PgEventStore<E, Srd>
where
    E: Event + Clone + Send + Sync,
    Srd: Serde<E> + Send + Sync,
{
    type Error = Error;
    /// Hydrates the given state using the events stored in the PostgreSQL event store.
    ///
    /// It retrieves events from the event store and applies them to the default state,
    /// resulting in a hydrated state. The function returns a `Hydrated` object containing
    /// the hydrated state and the version of the state.
    ///
    /// # Arguments
    ///
    /// * `default` - The default state to be hydrated.
    ///
    /// # Returns
    ///
    /// A `Result` containing the hydrated state wrapped in a `Hydrated` object and the version
    /// of the state if successful, or an `Error` if an error occurs during the hydration process.
    async fn hydrate<QE, S>(&self, default: S) -> Result<Hydrated<S>, Self::Error>
    where
        S: State<Event = QE>,
        QE: TryFrom<E> + Event + Clone + Send + Sync,
        <QE as TryFrom<E>>::Error: std::fmt::Debug + Send,
    {
        let (state, version) = self
            .stream(&default.query())
            .unwrap()
            .fold((default, 0), |(mut state, _), e| async move {
                let applied_event_id = e.id();
                state.mutate(e.into_inner());
                (state, applied_event_id)
            })
            .await;
        Ok(Hydrated::new(state, version))
    }

    /// Persists the changes derived from the given state into the PostgreSQL event store.
    ///
    /// It appends the changes as events to the event store,
    /// ensuring there are no conflicts.
    ///
    /// # Arguments
    ///
    /// * `state` - The hydrated state.
    /// * `changes` - The changes to be saved.
    async fn save<QE, S>(
        &self,
        state: &Hydrated<S>,
        changes: Vec<QE>,
    ) -> Result<Vec<PersistedEvent<E>>, Self::Error>
    where
        S: State,
        QE: Into<E> + Event + Clone + Send + Sync,
    {
        self.append(
            changes.into_iter().map(|e| e.into()).collect::<Vec<E>>(),
            state.query(),
            state.version(),
        )
        .await
    }
}
