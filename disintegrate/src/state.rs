//! A state object represents a business concept built from events.
use async_trait::async_trait;

use crate::stream_query::StreamQuery;
use crate::{event::Event, PersistedEvent};
use std::ops::{Deref, DerefMut};

/// Defines the behavior of State objects.
///
/// A state object represents a business concept and is built from events.
/// The associated `Event` type represents the events that
/// can mutate the state.
pub trait State: Clone + Send + Sync {
    type Event: Event + Clone + Send + Sync;

    /// Returns the stream query used to retrieve relevant events for building the state.
    fn query(&self) -> StreamQuery<Self::Event>;

    /// Mutates the state object based on the provided event.
    fn mutate(&mut self, event: Self::Event);
}

/// Defines the behavior of a state storage mechanism.
///
/// It provides methods for hydrating and saving state objects. The associated `Error` type
/// represents the possible errors that can occur during state storage operations.
#[async_trait]
pub trait StateStore<E>
where
    E: Event,
{
    type Error: std::error::Error + Send + Sync + 'static;
    /// Hydrates a state object from the state store.
    ///
    /// It takes a default state object as a parameter and returns a `Hydrated` object
    /// that contains the hydrated state and its version. The `default` parameter is used
    /// when the state object is not found in the store.
    async fn hydrate<QE, S: State<Event = QE>>(
        &self,
        default: S,
    ) -> Result<Hydrated<S>, Self::Error>
    where
        QE: TryFrom<E> + Event + Clone + Send + Sync,
        <QE as TryFrom<E>>::Error: std::fmt::Debug + Send;

    /// Persists the changes derived from the given state into the event store.
    async fn save<CE, S: State>(
        &self,
        state: &Hydrated<S>,
        changes: Vec<CE>,
    ) -> Result<Vec<PersistedEvent<E>>, Self::Error>
    where
        CE: Into<E> + Event + Clone + Send + Sync;
}

/// Wrapper of a state object along with its version.
///
/// It is used by the `StateStore` trait to represent a hydrated state object that is
/// retrieved from the store.
pub struct Hydrated<S: State> {
    state: S,
    version: i64,
}

impl<S: State> Hydrated<S> {
    /// Creates a new `Hydrated` instance with the provided state and version.
    pub fn new(state: S, version: i64) -> Self {
        Self { state, version }
    }
    /// Returns the version of the state object.
    pub fn version(&self) -> i64 {
        self.version
    }
}

impl<S: State> Deref for Hydrated<S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<S: State> DerefMut for Hydrated<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}
