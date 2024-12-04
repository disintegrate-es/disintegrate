//! State Store provides components for retrieving decision states and persisting decision changes.
use serde::{de::DeserializeOwned, Serialize};

use super::state::{MultiState, MultiStateSnapshot, StatePart};
use super::{IntoState, IntoStatePart};
use crate::event::EventId;
use crate::BoxDynError;
use crate::EventStore;
use crate::StateQuery;
use crate::{Event, PersistedEvent, StreamQuery};
use async_trait::async_trait;
use futures::TryStreamExt;
use std::error::Error as StdError;
use std::ops::Deref;

/// Represents the state loaded from the event store, along with its version.
///
/// This struct is used to encapsulate the state and its version, which can be used
/// to ensure that the state is up-to-date when making decisions or persisting changes.
///
/// # Type Parameters
///
/// - `S`: The type of the state.
pub struct LoadedState<ID: EventId, S> {
    /// The loaded state.
    pub(crate) state: S,
    /// The version of the loaded state.
    pub(crate) version: ID,
}

impl<ID: EventId, S> LoadedState<ID, S> {
    /// Returns a reference to the loaded state.
    pub fn state(&self) -> &S {
        &self.state
    }

    /// Returns the version of the loaded state.
    pub fn version(&self) -> ID {
        self.version
    }
}

/// A decision state store.
///
/// It provides methods for loading decision states and persisting decision changes.
#[async_trait]
pub trait DecisionStateStore<ID: EventId, S, E: Event + Clone> {
    /// Loads the decision state based on the provided state query.
    ///
    /// This method retrieves the current state from the storage backend, along with
    /// its version.
    ///
    /// # Parameters
    ///
    /// - `state_query`: The query object representing the desired state to hydrate.
    ///
    /// # Returns
    ///
    /// the loaded state, or an error if the load fails.
    async fn load(&self, state_query: S) -> Result<LoadedState<ID, S>, BoxDynError>;
    /// Persists the decision changes to the event store.
    ///
    /// # Parameters
    ///
    /// - `loaded_state`: The current state loaded from the event store, used to check if the events to be persisted have been produced from a non-stale state.
    /// - `events`: A vector of events representing the changes to be stored.
    /// - `validation_query`: An optional stream query used to validate the state before persisting changes.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `PersistedEvent` if the operation is successful, or an error if the persist operation fails.
    async fn persist(
        &self,
        loaded_state: LoadedState<ID, S>,
        events: Vec<E>,
        validation_query: Option<StreamQuery<ID, E>>,
    ) -> Result<Vec<PersistedEvent<ID, E>>, BoxDynError>;
}

/// A snapshotter.
///
/// Snapshots optimize the retrieval of `StatePart` by storing and loading partial or complete
/// representations of previously computed states.
///
/// Snapshots serve as a mechanism to enhance performance, enabling quicker access to cached states
/// and reducing the need for redundant recalculations of identical state queries.
#[async_trait]
pub trait StateSnapshotter<ID: EventId> {
    /// Loads a snapshot of a state part. If the snapshot is not present of invalid, it returns the provided `default`.
    ///
    /// - `default`: The default state to be used if no snapshot is available.
    ///
    /// Returns the loaded or default `StatePart<S>`.
    async fn load_snapshot<S>(&self, default: StatePart<ID, S>) -> StatePart<ID, S>
    where
        S: Send + Sync + DeserializeOwned + StateQuery + 'static;

    /// Stores a snapshot of the provided state part.
    ///
    /// - `state`: The state to be stored as a snapshot.
    ///
    /// Returns a `Result` indicating the success or failure of the operation.
    async fn store_snapshot<S>(&self, state: &StatePart<ID, S>) -> Result<(), BoxDynError>
    where
        S: Send + Sync + Serialize + StateQuery + 'static;
}

/// Snapshot configuration indicating how the snapshot of a `StatePart` must be performed.
pub trait SnapshotConfig {}

/// Indicates that the snapshot is disabled.
#[derive(Clone, Copy)]
pub struct NoSnapshot;

impl SnapshotConfig for NoSnapshot {}

/// Indicates that the snapshot is enabled and handled by the provided backend.
#[derive(Clone, Copy)]
pub struct WithSnapshot<ID: EventId, T: StateSnapshotter<ID> + Clone> {
    backend: T,
    event_id: std::marker::PhantomData<ID>,
}
impl<ID: EventId, T: StateSnapshotter<ID> + Clone> WithSnapshot<ID, T> {
    pub fn new(backend: T) -> Self {
        WithSnapshot {
            backend,
            event_id: std::marker::PhantomData,
        }
    }
}

impl<ID: EventId, T: StateSnapshotter<ID> + Clone> SnapshotConfig for WithSnapshot<ID, T> {}

impl<ID: EventId, T: StateSnapshotter<ID> + Clone> Deref for WithSnapshot<ID, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.backend
    }
}

/// Represents an event sourced decision state store. It loads and stores decision states from events in a event store.
#[derive(Clone)]
pub struct EventSourcedDecisionStateStore<ID, E, ES, SN>
where
    ID: EventId,
    E: Event + Clone + Send + Sync,
    ES: EventStore<ID, E> + Clone + Sync + Send,
    SN: SnapshotConfig + Clone,
{
    event_store: ES,
    snapshot: SN,
    event_id_type: std::marker::PhantomData<ID>,
    event_type: std::marker::PhantomData<E>,
}

impl<ID, E, ES, SN> EventSourcedDecisionStateStore<ID, E, ES, SN>
where
    ID: EventId,
    E: Event + Clone + Send + Sync,
    ES: EventStore<ID, E> + Clone + Sync + Send,
    SN: SnapshotConfig + Clone,
{
    pub fn new(event_store: ES, snapshot: SN) -> Self {
        EventSourcedDecisionStateStore {
            event_store,
            snapshot,
            event_id_type: std::marker::PhantomData,
            event_type: std::marker::PhantomData,
        }
    }

    async fn mutate_state<S>(&self, mut state_query: S) -> Result<S, BoxDynError>
    where
        ES: EventStore<ID, E> + Clone + Sync + Send,
        <ES as EventStore<ID, E>>::Error: StdError + Send + Sync + 'static,
        S: MultiState<ID, E> + Send + Sync + 'static,
        E: 'static,
    {
        let query = state_query.query_all();
        let mut event_stream = self.event_store.stream(&query);
        while let Some(event) = event_stream.try_next().await? {
            state_query.mutate_all(event);
        }
        Ok(state_query)
    }
}

#[async_trait]
impl<ID, ES, E, S> DecisionStateStore<ID, S, E>
    for EventSourcedDecisionStateStore<ID, E, ES, NoSnapshot>
where
    ES: EventStore<ID, E> + Clone + Sync + Send,
    <ES as EventStore<ID, E>>::Error: StdError + Send + Sync + 'static,
    ID: EventId,
    E: Event + Clone + Send + Sync + 'static,
    S: Send + Sync + Serialize + DeserializeOwned + IntoStatePart<ID, S> + 'static,
    <S as IntoStatePart<ID, S>>::Target:
        Send + Sync + Serialize + DeserializeOwned + IntoState<S> + MultiState<ID, E>,
{
    async fn load(&self, state_query: S) -> Result<LoadedState<ID, S>, BoxDynError> {
        let mutated_state = self.mutate_state(state_query.into_state_part()).await?;
        let version = mutated_state.version();
        Ok(LoadedState {
            state: mutated_state.into_state(),
            version,
        })
    }

    async fn persist(
        &self,
        loaded_state: LoadedState<ID, S>,
        events: Vec<E>,
        validation_query: Option<StreamQuery<ID, E>>,
    ) -> Result<Vec<PersistedEvent<ID, E>>, BoxDynError> {
        let query =
            validation_query.unwrap_or_else(|| loaded_state.state.into_state_part().query_all());
        Ok(self
            .event_store
            .append(events, query, loaded_state.version)
            .await?)
    }
}

#[async_trait]
impl<ID, ES, E, S, B> DecisionStateStore<ID, S, E>
    for EventSourcedDecisionStateStore<ID, E, ES, WithSnapshot<ID, B>>
where
    ID: EventId,
    B: StateSnapshotter<ID> + Send + Sync + Clone,
    ES: EventStore<ID, E> + Clone + Sync + Send,
    <ES as EventStore<ID, E>>::Error: StdError + Send + Sync + 'static,
    E: Event + Clone + Send + Sync + 'static,
    S: Send + Sync + Serialize + DeserializeOwned + IntoStatePart<ID, S> + 'static,
    <S as IntoStatePart<ID, S>>::Target: Send
        + Sync
        + Serialize
        + DeserializeOwned
        + IntoState<S>
        + MultiState<ID, E>
        + MultiStateSnapshot<ID, B>,
{
    async fn load(&self, state_query: S) -> Result<LoadedState<ID, S>, BoxDynError> {
        let mut state_query = state_query.into_state_part();
        state_query.load_all(&self.snapshot.backend).await;
        let state = self.mutate_state(state_query).await?;
        state.store_all(&self.snapshot.backend).await?;
        let version = state.version();
        Ok(LoadedState {
            state: state.into_state(),
            version,
        })
    }

    async fn persist(
        &self,
        loaded_state: LoadedState<ID, S>,
        events: Vec<E>,
        validation_query: Option<StreamQuery<ID, E>>,
    ) -> Result<Vec<PersistedEvent<ID, E>>, BoxDynError> {
        let query =
            validation_query.unwrap_or_else(|| loaded_state.state.into_state_part().query_all());
        Ok(self
            .event_store
            .append(events, query, loaded_state.version)
            .await?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{utils::tests::*, IntoStatePart};

    #[tokio::test]
    async fn it_loads_query_state() {
        let mut mock_store = MockDatabase::new();

        mock_store.expect_stream().once().return_once(|_| {
            event_stream([
                item_added_event("p1", "c1"),
                item_removed_event("p1", "c1"),
                item_added_event("p3", "c2"),
            ])
        });

        let event_store = MockEventStore::new(mock_store);
        let state_store = EventSourcedDecisionStateStore::new(event_store, NoSnapshot);
        let state = (cart("c1", []), cart("c2", []));
        let state = state_store.load(state).await.unwrap();
        let LoadedState {
            state: (cart1, cart2),
            version,
        } = state;
        assert_eq!(version, 3);
        assert_eq!(cart1, cart("c1", []));
        assert_eq!(cart2, cart("c2", ["p3".to_owned()]));
    }

    #[tokio::test]
    async fn it_persists_decision_changes() {
        let mut mock_store = MockDatabase::new();

        mock_store.expect_append().once().return_once(
            |_, _: StreamQuery<i64, ShoppingCartEvent>, _| {
                vec![PersistedEvent::new(1, item_added_event("p2", "c1"))]
            },
        );

        let event_store = MockEventStore::new(mock_store);
        let state_store = EventSourcedDecisionStateStore::new(event_store, NoSnapshot);
        let state = (Cart::new("c1"), Cart::new("c2"));
        let loaded_state = LoadedState { state, version: 1 };
        state_store
            .persist(loaded_state, vec![item_added_event("p2", "c1")], None)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn it_loads_query_state_from_snapshot() {
        let mut mock_store = MockDatabase::new();

        mock_store.expect_stream().once().return_once(|_| {
            event_stream([item_added_event("p3", "c1"), item_added_event("p4", "c2")])
        });

        let mut snapshotter = MockStateSnapshotter::new();
        snapshotter
            .expect_load_snapshot()
            .once()
            .withf(|q: &StatePart<i64, Cart>| q.cart_id == "c1")
            .returning(|_| cart("c1", ["p1".to_owned()]).into_state_part());
        snapshotter
            .expect_load_snapshot()
            .once()
            .withf(|q: &StatePart<i64, Cart>| q.cart_id == "c2")
            .returning(|_| cart("c2", ["p2".to_owned()]).into_state_part());
        snapshotter
            .expect_store_snapshot()
            .once()
            .withf(|q: &StatePart<i64, Cart>| q.cart_id == "c1")
            .returning(|_| Ok(()));
        snapshotter
            .expect_store_snapshot()
            .once()
            .withf(|q: &StatePart<i64, Cart>| q.cart_id == "c2")
            .returning(|_| Ok(()));

        let event_store = MockEventStore::new(mock_store);
        let state_store =
            EventSourcedDecisionStateStore::new(event_store, WithSnapshot::new(snapshotter));
        let state = (cart("c1", []), cart("c2", []));
        let LoadedState {
            state: (cart1, cart2),
            version,
        } = state_store.load(state).await.unwrap();

        assert_eq!(version, 2);
        assert_eq!(cart1, cart("c1", ["p1".to_owned(), "p3".to_owned()]));
        assert_eq!(cart2, cart("c2", ["p2".to_owned(), "p4".to_owned()]));
    }

    #[tokio::test]
    async fn it_returns_the_max_version_of_the_loaded_snapshots() {
        let mut mock_store = MockDatabase::new();

        mock_store
            .expect_stream()
            .once()
            .return_once(|_: &StreamQuery<i64, ShoppingCartEvent>| event_stream([]));

        let mut snapshotter = MockStateSnapshotter::new();
        snapshotter
            .expect_load_snapshot()
            .once()
            .withf(|q: &StatePart<i64, Cart>| q.cart_id == "c1")
            .returning(|_| StatePart::new(5, cart("c1", [])));
        snapshotter
            .expect_load_snapshot()
            .once()
            .withf(|q: &StatePart<i64, Cart>| q.cart_id == "c2")
            .returning(|_| StatePart::new(3, cart("c2", [])));
        snapshotter
            .expect_store_snapshot()
            .times(2)
            .returning(|_: &StatePart<i64, Cart>| Ok(()));

        let event_store = MockEventStore::new(mock_store);
        let state_store =
            EventSourcedDecisionStateStore::new(event_store, WithSnapshot::new(snapshotter));
        let state = (cart("c1", []), cart("c2", []));
        let LoadedState { version, .. } = state_store.load(state).await.unwrap();

        assert_eq!(version, 5);
    }
}
