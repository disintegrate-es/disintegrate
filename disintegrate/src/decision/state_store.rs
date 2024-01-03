//! State Store provides components for retrieving decision states and persisting decision changes.
use futures::TryStreamExt;
use serde::{de::DeserializeOwned, Serialize};

use super::state::{MultiState, MultiStateSnapshot, StatePart};
use crate::BoxDynError;
use crate::EventStore;
use crate::StateQuery;
use crate::{Event, PersistedEvent, StreamQuery};
use async_trait::async_trait;
use std::error::Error as StdError;
use std::ops::Deref;

/// A decision state store.
///
/// It provides methods for loading decision states and persisting decision changes.
#[async_trait]
pub trait DecisionStateStore<S, E: Event + Clone> {
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
    async fn load(&self, state_query: S) -> Result<S, BoxDynError>;
    /// Persists the decision changes.
    ///
    /// # Parameters
    ///
    /// - `query_state`: The query state used to load the decision state.
    /// - `events`: A vector of events representing the changes to be stored.
    /// - `validation_query`: An optional stream query to perform validation of the state.
    /// - `state_version`: The version of the used by the decision. It is the version returned by the `load` method.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `PersistedEvent` or an error if the persist fails.
    async fn persist(
        &self,
        query_state: S,
        events: Vec<E>,
        validation_query: Option<StreamQuery<E>>,
        state_version: i64,
    ) -> Result<Vec<PersistedEvent<E>>, BoxDynError>;
}

/// A snapshotter.
///
/// Snapshots optimize the retrieval of `StatePart` by storing and loading partial or complete
/// representations of previously computed states.
///
/// Snapshots serve as a mechanism to enhance performance, enabling quicker access to cached states
/// and reducing the need for redundant recalculations of identical state queries.
#[async_trait]
pub trait StateSnapshotter {
    /// Loads a snapshot of a state part. If the snapshot is not present of invalid, it returns the provided `default`.
    ///
    /// - `default`: The default state to be used if no snapshot is available.
    ///
    /// Returns the loaded or default `StatePart<S>`.
    async fn load_snapshot<S>(&self, default: StatePart<S>) -> StatePart<S>
    where
        S: Send + Sync + DeserializeOwned + StateQuery + 'static;

    /// Stores a snapshot of the provided state part.
    ///
    /// - `state`: The state to be stored as a snapshot.
    ///
    /// Returns a `Result` indicating the success or failure of the operation.
    async fn store_snapshot<S>(&self, state: &StatePart<S>) -> Result<(), BoxDynError>
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
pub struct WithSnapshot<T: StateSnapshotter + Clone> {
    backend: T,
}
impl<T: StateSnapshotter + Clone> WithSnapshot<T> {
    pub fn new(backend: T) -> Self {
        WithSnapshot { backend }
    }
}

impl<T: StateSnapshotter + Clone> SnapshotConfig for WithSnapshot<T> {}

impl<T: StateSnapshotter + Clone> Deref for WithSnapshot<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.backend
    }
}

/// Represents an event sourced decision state store. It loads and stores decision states from events in a event store.
#[derive(Clone)]
pub struct EventSourcedDecisionStateStore<ES: Clone, SN: SnapshotConfig + Clone> {
    event_store: ES,
    snapshot: SN,
}

impl<ES: Clone, SN: SnapshotConfig + Clone> EventSourcedDecisionStateStore<ES, SN> {
    pub fn new(event_store: ES, snapshot: SN) -> EventSourcedDecisionStateStore<ES, SN> {
        EventSourcedDecisionStateStore {
            event_store,
            snapshot,
        }
    }

    async fn mutate_state<S, E>(&self, mut state_query: S) -> Result<S, BoxDynError>
    where
        ES: EventStore<E> + Clone + Sync + Send,
        <ES as EventStore<E>>::Error: StdError + Send + Sync + 'static,
        E: Event + Clone + Send + Sync + 'static,
        S: MultiState<E> + Send + Sync + 'static,
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
impl<ES, E, S> DecisionStateStore<S, E> for EventSourcedDecisionStateStore<ES, NoSnapshot>
where
    ES: EventStore<E> + Clone + Sync + Send,
    <ES as EventStore<E>>::Error: StdError + Send + Sync + 'static,
    E: Event + Clone + Send + Sync + 'static,
    S: MultiState<E> + Send + Sync + 'static,
{
    async fn load(&self, state_query: S) -> Result<S, BoxDynError> {
        self.mutate_state(state_query).await
    }

    async fn persist(
        &self,
        state_query: S,
        events: Vec<E>,
        validation_query: Option<StreamQuery<E>>,
        last_event_id: i64,
    ) -> Result<Vec<PersistedEvent<E>>, BoxDynError> {
        let query = validation_query.unwrap_or_else(|| state_query.query_all());
        Ok(self
            .event_store
            .append(events, query, last_event_id)
            .await?)
    }
}

#[async_trait]
impl<ES, E, S, B> DecisionStateStore<S, E> for EventSourcedDecisionStateStore<ES, WithSnapshot<B>>
where
    B: StateSnapshotter + Send + Sync + Clone,
    ES: EventStore<E> + Clone + Sync + Send,
    <ES as EventStore<E>>::Error: StdError + Send + Sync + 'static,
    E: Event + Clone + Send + Sync + 'static,
    S: MultiState<E> + MultiStateSnapshot<B> + Send + Sync + 'static,
{
    async fn load(&self, mut state_query: S) -> Result<S, BoxDynError> {
        state_query.load_all(&self.snapshot.backend).await;
        let state = self.mutate_state(state_query).await?;
        state.store_all(&self.snapshot.backend).await?;
        Ok(state)
    }

    async fn persist(
        &self,
        from_state: S,
        events: Vec<E>,
        validation_query: Option<StreamQuery<E>>,
        last_event_id: i64,
    ) -> Result<Vec<PersistedEvent<E>>, BoxDynError> {
        let query = validation_query.unwrap_or_else(|| from_state.query_all());
        Ok(self
            .event_store
            .append(events, query, last_event_id)
            .await?)
    }
}

#[cfg(test)]
mod test {
    use futures::executor::block_on;

    use super::*;
    use crate::{utils::tests::*, IntoState, IntoStatePart, MultiState};

    #[test]
    fn it_loads_query_state() {
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
        let state = (cart("c1", []), cart("c2", [])).into_state_part();
        let state = block_on(state_store.load(state)).unwrap();
        assert_eq!(MultiState::<ShoppingCartEvent>::version(&state), 3);
        let (cart1, cart2) = state;
        assert_eq!(cart1.version(), 2);
        assert_eq!(cart1.applied_events(), 2);
        assert_eq!(cart1.into_state(), cart("c1", []));
        assert_eq!(cart2.version(), 3);
        assert_eq!(cart2.applied_events(), 1);
        assert_eq!(cart2.into_state(), cart("c2", ["p3".to_owned()]));
    }

    #[test]
    fn it_persists_decision_changes() {
        let mut mock_store = MockDatabase::new();

        mock_store
            .expect_append()
            .once()
            .return_once(|_, _: StreamQuery<ShoppingCartEvent>, _| {
                vec![PersistedEvent::new(1, item_added_event("p2", "c1"))]
            });

        let event_store = MockEventStore::new(mock_store);
        let state_store = EventSourcedDecisionStateStore::new(event_store, NoSnapshot);
        let state = (Cart::new("c1"), Cart::new("c2")).into_state_part();
        block_on(state_store.persist(state, vec![item_added_event("p2", "c1")], None, 1)).unwrap();
    }

    #[test]
    fn it_loads_query_state_from_snapshot() {
        let mut mock_store = MockDatabase::new();

        mock_store.expect_stream().once().return_once(|_| {
            event_stream([item_added_event("p3", "c1"), item_added_event("p4", "c2")])
        });

        let mut snapshotter = MockStateSnapshotter::new();
        snapshotter
            .expect_load_snapshot()
            .once()
            .withf(|q: &StatePart<Cart>| q.cart_id == "c1")
            .returning(|_| cart("c1", ["p1".to_owned()]).into_state_part());
        snapshotter
            .expect_load_snapshot()
            .once()
            .withf(|q: &StatePart<Cart>| q.cart_id == "c2")
            .returning(|_| cart("c2", ["p2".to_owned()]).into_state_part());
        snapshotter
            .expect_store_snapshot()
            .once()
            .withf(|q: &StatePart<Cart>| q.cart_id == "c1")
            .returning(|_| Ok(()));
        snapshotter
            .expect_store_snapshot()
            .once()
            .withf(|q: &StatePart<Cart>| q.cart_id == "c2")
            .returning(|_| Ok(()));

        let event_store = MockEventStore::new(mock_store);
        let state_store =
            EventSourcedDecisionStateStore::new(event_store, WithSnapshot::new(snapshotter));
        let state = (cart("c1", []), cart("c2", [])).into_state_part();
        let state = block_on(state_store.load(state)).unwrap();

        assert_eq!(MultiState::<ShoppingCartEvent>::version(&state), 2);
        let (cart1, cart2) = state;
        assert_eq!(cart1.version(), 1);
        assert_eq!(cart1.applied_events(), 1);
        assert_eq!(
            cart1.into_state(),
            cart("c1", ["p1".to_owned(), "p3".to_owned()])
        );
        assert_eq!(cart2.version(), 2);
        assert_eq!(cart2.applied_events(), 1);
        assert_eq!(
            cart2.into_state(),
            cart("c2", ["p2".to_owned(), "p4".to_owned()])
        );
    }

    #[test]
    fn it_returns_the_max_version_of_the_loaded_snapshots() {
        let mut mock_store = MockDatabase::new();

        mock_store
            .expect_stream()
            .once()
            .return_once(|_: &StreamQuery<ShoppingCartEvent>| event_stream([]));

        let mut snapshotter = MockStateSnapshotter::new();
        snapshotter
            .expect_load_snapshot()
            .once()
            .withf(|q: &StatePart<Cart>| q.cart_id == "c1")
            .returning(|_| StatePart::new(5, cart("c1", [])));
        snapshotter
            .expect_load_snapshot()
            .once()
            .withf(|q: &StatePart<Cart>| q.cart_id == "c2")
            .returning(|_| StatePart::new(3, cart("c2", [])));
        snapshotter
            .expect_store_snapshot()
            .times(2)
            .returning(|_: &StatePart<Cart>| Ok(()));

        let event_store = MockEventStore::new(mock_store);
        let state_store =
            EventSourcedDecisionStateStore::new(event_store, WithSnapshot::new(snapshotter));
        let state = (cart("c1", []), cart("c2", [])).into_state_part();
        let state = block_on(state_store.load(state)).unwrap();

        assert_eq!(MultiState::<ShoppingCartEvent>::version(&state), 5);
    }
}
