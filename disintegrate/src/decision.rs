//! A Decision serves as a building block for developing the business logic of an application.

use crate::stream_query::StreamQuery;
use crate::EventStore;
use crate::{event::Event, PersistedEvent};
use futures::TryStreamExt;
use std::error::Error as StdError;

/// Represents a business decision taken from the occurred events.
pub trait Decision: Send + Sync {
    type Event: Event + Clone + Send + Sync;
    type State: State + Clone + Send + Sync;
    type Error: Send + Sync;

    /// Returns the default state that will be mutated using the event store.
    /// In case there are no events in the event store that match the query of the state,
    /// this default state will be used to make the decision.
    fn default_state(&self) -> Self::State;

    /// Retrieves the stream query used to validate the decision.
    /// If the validation query is `None`, the StreamQuery of the default state will be used for validation.
    /// This means that the decision will only be confirmed if new events that would make the state outdated are not found.
    /// However, if a `validation_query` is provided, it will be used to confirm the decision.
    /// This allows narrowing down the set of events that could invalidate the decision.
    ///
    /// For example, in a banking system, deposit events should not invalidate withdrawals if the account balance is already sufficient.
    /// In this case, the query of the state needs to include both withdraw and deposit events to compute the available balance,
    /// but only withdraw events should invalidate the decision.
    /// In other words, once we have confirmed that the account has a sufficient amount,
    /// only a withdraw event can reduce the balance below the requested amount and invalidate the decision.
    fn validation_query(&self) -> Option<StreamQuery<<Self::State as State>::Event>>;

    /// Process the decision from the mutated state.
    fn process(&self, state: &Self::State) -> Result<Vec<Self::Event>, Self::Error>;
}

pub struct PersistedDecision<S: State, E: Event> {
    state: S,
    events: Vec<PersistedEvent<E>>,
}

impl<S: State, E: Event> PersistedDecision<S, E> {
    /// Returns the state used to derive the decision
    pub fn state(&self) -> &S {
        &self.state
    }

    /// Returns the persisted events produced by the decision
    pub fn events(&self) -> &[PersistedEvent<E>] {
        &self.events
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error<ES, DE> {
    #[error("event store error: {0}")]
    EventStore(#[source] ES),
    #[error("domain error: {0}")]
    Domain(#[source] DE),
}

/// Executes business decisions.
#[derive(Clone)]
pub struct DecisionMaker<ES> {
    event_store: ES,
}

impl<ES> DecisionMaker<ES> {
    /// Creates a new instance of `DecisionMaker`.
    pub fn new<E>(event_store: ES) -> Self
    where
        E: Event + Clone + Sync + Send,
        ES: EventStore<E>,
    {
        Self { event_store }
    }

    /// Makes the given business decision.
    ///
    /// It then persists the resulting events in the event store
    /// and returns the persisted events along with the hydrated state.
    ///
    /// Note: the returned state is not mutated with the resulting decision events.
    /// This is because a decision may produce a different set of events
    /// compared to the events used by a State.
    pub async fn make<D, S, E, DE, QE>(
        &self,
        decision: D,
    ) -> Result<PersistedDecision<S, E>, Error<ES::Error, D::Error>>
    where
        E: Event + Clone + Sync + Send,
        ES: EventStore<E>,
        D: Decision<State = S, Event = DE>,
        DE: Into<E> + Event,
        S: State<Event = QE>,
        QE: TryFrom<E> + Event + 'static + Clone + Send + Sync,
        <QE as TryFrom<E>>::Error: StdError + 'static + Send + Sync,
        <ES as EventStore<E>>::Error: 'static,
        <D as Decision>::Error: 'static,
    {
        let default_state = decision.default_state();
        let (state, version) = self
            .event_store
            .stream(&default_state.query())
            .try_fold((default_state, 0), |(mut state, _), evt| async move {
                let applied_event_id = evt.id();
                state.mutate(evt.into_inner());
                Ok((state, applied_event_id))
            })
            .await
            .map_err(Error::EventStore)?;
        let changes = decision.process(&state).map_err(Error::Domain)?;
        let events = self
            .event_store
            .append(
                changes.into_iter().map(|change| change.into()).collect(),
                decision.validation_query().unwrap_or_else(|| state.query()),
                version,
            )
            .await
            .map_err(Error::EventStore)?;

        Ok(PersistedDecision { state, events })
    }
}

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

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use futures::{
        executor::block_on,
        stream::{self, BoxStream},
        StreamExt,
    };
    use mockall::{automock, mock};
    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::{domain_identifiers, query, DomainIdentifierSet, EventSchema};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "event_type", rename_all = "snake_case")]
    enum ShoppingCartEvent {
        ItemAdded {
            item_id: String,
            cart_id: String,
            quantity: u32,
        },
        ItemUpdated {
            item_id: String,
            cart_id: String,
            quantity: u32,
        },
        ItemRemoved {
            item_id: String,
            cart_id: String,
            quantity: u32,
        },
    }

    impl Event for ShoppingCartEvent {
        const SCHEMA: EventSchema = EventSchema {
            types: &["ItemAdded", "ItemUpdated", "ItemRemoved"],
            domain_identifiers: &["cart_id", "product_id"],
        };
        fn name(&self) -> &'static str {
            match self {
                ShoppingCartEvent::ItemAdded { .. } => "ItemAdded",
                ShoppingCartEvent::ItemUpdated { .. } => "ItemUpdated",
                ShoppingCartEvent::ItemRemoved { .. } => "ItemRemoved",
            }
        }
        fn domain_identifiers(&self) -> DomainIdentifierSet {
            match self {
                ShoppingCartEvent::ItemAdded {
                    item_id, cart_id, ..
                } => domain_identifiers! {item_id: item_id, cart_id: cart_id},
                ShoppingCartEvent::ItemUpdated {
                    item_id, cart_id, ..
                } => domain_identifiers! {item_id: item_id, cart_id: cart_id},
                ShoppingCartEvent::ItemRemoved {
                    item_id, cart_id, ..
                } => domain_identifiers! {item_id: item_id, cart_id: cart_id},
            }
        }
    }

    struct DummyEventStore<D> {
        database: D,
    }

    #[derive(Debug)]
    struct Error;

    #[automock]
    trait Database {
        fn stream<QE: Event + Clone + 'static + Send + Sync>(
            &self,
            query: &StreamQuery<QE>,
        ) -> Vec<Result<PersistedEvent<QE>, Error>>;

        fn append<QE: Event + Clone + 'static + Send + Sync>(
            &self,
            events: Vec<ShoppingCartEvent>,
            query: StreamQuery<QE>,
            last_event_id: i64,
        ) -> Vec<PersistedEvent<ShoppingCartEvent>>;
    }

    #[async_trait]
    impl<D: Database + Sync> EventStore<ShoppingCartEvent> for DummyEventStore<D> {
        type Error = Error;

        fn stream<'a, QE>(
            &'a self,
            query: &'a StreamQuery<QE>,
        ) -> BoxStream<Result<PersistedEvent<QE>, Self::Error>>
        where
            QE: TryFrom<ShoppingCartEvent> + Event + 'static + Clone + Send + Sync,
            <QE as TryFrom<ShoppingCartEvent>>::Error: StdError + 'static + Send + Sync,
        {
            stream::iter(self.database.stream(query)).boxed()
        }

        async fn append<QE>(
            &self,
            events: Vec<ShoppingCartEvent>,
            query: StreamQuery<QE>,
            last_event_id: i64,
        ) -> Result<Vec<PersistedEvent<ShoppingCartEvent>>, Self::Error>
        where
            QE: Event + 'static + Clone + Send + Sync,
        {
            Ok(self.database.append(events, query, last_event_id))
        }
    }

    #[derive(Default, Debug, Clone, Eq, PartialEq)]
    struct Cart {
        cart_id: String,
        items: HashMap<String, u32>,
    }

    impl Cart {
        pub fn new(cart_id: &str) -> Self {
            Self {
                cart_id: cart_id.into(),
                ..Default::default()
            }
        }
    }

    impl State for Cart {
        type Event = ShoppingCartEvent;

        fn query(&self) -> StreamQuery<Self::Event> {
            query!(ShoppingCartEvent, cart_id == self.cart_id)
        }

        fn mutate(&mut self, event: Self::Event) {
            match event {
                ShoppingCartEvent::ItemAdded {
                    item_id, quantity, ..
                } => {
                    self.items.insert(item_id, quantity);
                }
                ShoppingCartEvent::ItemRemoved { item_id, .. } => {
                    self.items.remove(&item_id);
                }
                ShoppingCartEvent::ItemUpdated {
                    item_id, quantity, ..
                } => {
                    self.items.insert(item_id, quantity);
                }
            }
        }
    }

    #[derive(Debug)]
    enum CartError {}

    mock! {
            AddItem{}
            impl Decision for AddItem {
                type Event = ShoppingCartEvent;
                type State = Cart;
                type Error = CartError;

            fn default_state(&self) -> <Self as Decision>::State;
            fn validation_query(&self) -> Option<StreamQuery<ShoppingCartEvent>>;
            fn process(&self, _state: &<Self as Decision>::State) -> Result<Vec<<Self as Decision>::Event>, <Self as Decision>::Error>;
        }
    }

    #[test]
    fn it_should_hydrate_state_and_persist_event() {
        let mut mock_database = MockDatabase::new();

        mock_database.expect_stream().once().return_once(|_| {
            vec![
                Ok(PersistedEvent::new(
                    1,
                    ShoppingCartEvent::ItemAdded {
                        item_id: "p1".to_owned(),
                        cart_id: "c1".to_owned(),
                        quantity: 2,
                    },
                )),
                Ok(PersistedEvent::new(
                    2,
                    ShoppingCartEvent::ItemAdded {
                        item_id: "p2".to_owned(),
                        cart_id: "c1".to_owned(),
                        quantity: 3,
                    },
                )),
            ]
        });

        mock_database.expect_append().once().return_once(
            |_, _: StreamQuery<ShoppingCartEvent>, _| {
                vec![PersistedEvent::new(
                    3,
                    ShoppingCartEvent::ItemAdded {
                        item_id: "p3".to_owned(),
                        cart_id: "c1".to_owned(),
                        quantity: 1,
                    },
                )]
            },
        );

        let mut mock_add_item = MockAddItem::new();
        mock_add_item
            .expect_default_state()
            .once()
            .return_once(|| Cart::new("c1"));
        mock_add_item
            .expect_validation_query()
            .once()
            .return_once(|| None);
        mock_add_item.expect_process().once().return_once(|_| {
            Ok(vec![ShoppingCartEvent::ItemAdded {
                cart_id: "c1".to_string(),
                item_id: "p3".to_string(),
                quantity: 1,
            }])
        });

        let event_store = DummyEventStore {
            database: mock_database,
        };

        let decision_maker = DecisionMaker::new(event_store);

        let result = block_on(decision_maker.make(mock_add_item));

        let persisted_decision = result.unwrap();

        let expected_cart_state = Cart {
            cart_id: "c1".to_owned(),
            items: vec![("p1".to_owned(), 2), ("p2".to_owned(), 3)]
                .into_iter()
                .collect(),
        };
        assert_eq!(persisted_decision.state(), &expected_cart_state);

        let persisted_events = persisted_decision.events();
        assert_eq!(
            *persisted_events[0],
            ShoppingCartEvent::ItemAdded {
                item_id: "p3".to_owned(),
                cart_id: "c1".to_owned(),
                quantity: 1,
            }
        );
    }
}
