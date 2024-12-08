//! A Decision serves as a building block for developing the business logic of an application.

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::event::EventId;
use crate::state_store::LoadedState;
use crate::stream_query::StreamQuery;
use crate::{event::Event, PersistedEvent};
use crate::{BoxDynError, IntoState, IntoStatePart, LoadState, MultiState};

/// Represents a business decision taken from a state built upon the occurred events.
pub trait Decision: Send + Sync {
    type Event: Event + Clone + Send + Sync;
    type StateQuery: Clone + Send + Sync;
    type Error: Send + Sync;

    /// Returns the state query to compute the decision state from the events in the event store.
    ///
    /// If there are no events that match the specified query, the default values of the state query is utilized to make the decision.
    fn state_query(&self) -> Self::StateQuery;

    /// Returns the stream query used to validate the decision.
    ///
    /// If the validation query is `None`, the state query will be used for validation.
    /// This means that the decision will only be confirmed if new events that would make the state query outdated are not found.
    /// However, if a `validation_query` is provided, it will be used to confirm the decision.
    /// This allows narrowing down the set of events that could invalidate the decision.
    ///
    /// For example, in a banking system, deposit events should not invalidate withdrawals if the account balance is already sufficient.
    /// In this case, the state query needs to include both withdraw and deposit events to compute the available balance,
    /// but only withdraw events should invalidate the decision.
    /// In other words, once we have confirmed that the account has a sufficient amount,
    /// only a withdraw event can reduce the balance below the requested amount and invalidate the decision.
    fn validation_query<ID: EventId>(&self) -> Option<StreamQuery<ID, Self::Event>> {
        None
    }

    /// Evaluates the decision based on the mutated state, ensuring that all business rules
    /// are verified against the current state. This method generates a series of events
    /// that capture the changes made by the decision, allowing the results to be
    /// persisted in the event store.
    ///
    /// # Parameters
    ///
    /// - `state`: A reference to the current state of the system, obtained through
    ///   the implementation of the `StateQuery` trait.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success of the process. If successful, it contains
    /// a vector of events representing the changes made. In case of an error, it
    /// contains details about the encountered issue.
    fn process(&self, state: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error>;
}

#[derive(thiserror::Error, Debug)]
pub enum Error<DE> {
    #[error("event store error: {0}")]
    EventStore(#[source] BoxDynError),
    #[error("state store error: {0}")]
    StateStore(#[source] BoxDynError),
    #[error("domain error: {0}")]
    Domain(#[source] DE),
}

/// The `DecisionMaker` struct is responsible for executing and persisting business decisions.
#[derive(Clone)]
pub struct DecisionMaker<SS> {
    state_store: SS,
}

impl<SS> DecisionMaker<SS> {
    /// Creates a new instance of `DecisionMaker`.
    ///
    /// # Parameters
    ///
    /// - `state_store`: The state store backend used by the `DecisionMaker` to load the current state
    ///   and persist the decision.
    pub fn new(state_store: SS) -> Self {
        Self { state_store }
    }

    /// Makes the given business decision, persisting the resulting events in the event store.
    ///
    /// # Parameters
    ///
    /// - `decision`: The business decision to be executed, implementing the `Decision` trait.
    ///
    /// # Returns
    ///
    /// A `Result` indicating the success of the decision-making process. If successful,
    /// it contains a vector of `PersistedEvent` representing the changes made. In case of
    /// an error, it contains details about the encountered issue.
    pub async fn make<D, S, ID, E>(
        &self,
        decision: D,
    ) -> Result<Vec<PersistedEvent<ID, E>>, Error<D::Error>>
    where
        ID: EventId,
        E: Event + Clone + Sync + Send + 'static,
        SS: LoadState<ID, S, E> + PersistDecision<ID, S, E>,
        D: Decision<StateQuery = S, Event = E>,
        S: Send + Sync + Serialize + DeserializeOwned + IntoStatePart<ID, S>,
        <S as IntoStatePart<ID, S>>::Target:
            Send + Sync + Serialize + DeserializeOwned + IntoState<S> + MultiState<ID, E>,
        <D as Decision>::Error: 'static,
    {
        let loaded_state = self
            .state_store
            .load(decision.state_query())
            .await
            .map_err(Error::StateStore)?;
        let changes = decision
            .process(&loaded_state.state)
            .map_err(Error::Domain)?;
        let events = self
            .state_store
            .persist(
                loaded_state,
                changes.into_iter().collect(),
                decision.validation_query(),
            )
            .await
            .map_err(Error::StateStore)?;

        Ok(events)
    }
}

#[cfg(test)]
mod test {
    use mockall::predicate::eq;

    use super::*;
    use crate::{utils::tests::*, EventSourcedStateStore, NoSnapshot, StateQuery};

    #[tokio::test]
    async fn it_processes_a_decision() {
        let mut database = MockDatabase::new();

        database.expect_stream().once().return_once(|_| {
            event_stream([item_added_event("p1", "c1"), item_removed_event("p1", "c1")])
        });

        let state_query = cart("c1", []).query().change_origin(0);
        database
            .expect_append()
            .with(
                eq(vec![item_added_event("p2", "c1")]),
                eq(state_query),
                eq(2),
            )
            .once()
            .return_once(|_, _, _| vec![PersistedEvent::new(3, item_added_event("p2", "c1"))]);

        let mut mock_add_item = MockDecision::new();
        mock_add_item
            .expect_state_query()
            .once()
            .return_once(|| cart("c1", []));
        mock_add_item
            .expect_validation_query()
            .once()
            .return_once(|| Option::<StreamQuery<i64, ShoppingCartEvent>>::None);
        mock_add_item
            .expect_process()
            .once()
            .return_once(|_| Ok(vec![item_added_event("p2", "c1")]));

        let event_store = MockEventStore::new(database);
        let state_store = EventSourcedStateStore::new(event_store, NoSnapshot);
        let decision_maker = DecisionMaker::new(state_store);

        decision_maker.make(mock_add_item).await.unwrap();
    }
}

/// Persists decision changes to the event store.
#[async_trait::async_trait]
pub trait PersistDecision<ID: EventId, S, E: Event + Clone> {
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
