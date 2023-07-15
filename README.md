# Disintegrate

Disintegrate is a Rust library that provides an alternative approach to building domain objects from an event stream. While supporting traditional aggregates, Disintegrate introduces a novel method that allows for more flexibility and adaptability in modeling business rules.

## Why Disintegrate?

Disintegrate offers a fresh perspective on designing business applications by shifting away from relying solely on aggregates. Instead, it enables developers to construct business concepts directly from an event stream. This approach allows for decentralized and dynamic architectures that can evolve over time.

By leveraging the event stream as the foundation, Disintegrate empowers developers to build models that capture the essence of business events without the need for multiple versions of the same event within aggregates. This reduces duplication and complexity, leading to cleaner and more maintainable code.

## Key Features

* **Event Stream Modeling**: Disintegrate enables the construction of business concepts directly from an event stream, providing a more flexible and decentralized approach.

* **Support for Aggregates**: While promoting a new approach, Disintegrate still supports traditional aggregates, allowing developers to transition gradually or utilize both methodologies in their applications.

* **Adaptability to Changing Business Rules**: Disintegrate allows for easier evolution and adaptation to evolving business rules over time. By decoupling models from aggregates, developers gain the freedom to adjust and refine business concepts without heavy dependencies.

## Usage 

To add Disintegrate to your project, follow these steps:

1. Add `disintegrate` and `disintegrate-postgres` as a dependencies in your `Cargo.toml` file:

    ```toml
    [dependencies]
    disintegrate = "0.5.0"
    disintegrate-postgres = "0.5.0"
    ```

    * Disintegrate provides several features that you can enable based on your project requirements. You can include them in your `Cargo.toml` file as follows:

    ```toml
    [dependencies]
    disintegrate = { version = "0.5.0", features = ["macros", "serde-prost"] }
    disintegrate-postgres = { version = "0.5.0", features = ["listener"] }
    ```

    * The macros feature enables the use of derive macros to simplify events implementations.

    * For events serialization and deserialization, Disintegrate supports different serialization formats through the Serde ecosystem. You can enable the desired format by including the corresponding feature:

        * To enable JSON serialization, use the `serde-json` feature: `features = ["serde-json"]`.
        * To enable Avro serialization, use the `serde-avro` feature: `features = ["serde-avro"]`.
        * To enable Prost serialization, use the `serde-prost` feature: `features = ["serde-prost"]`.
        * To enable Protocol Buffers serialization, use the `serde-protobuf` feature: `features = ["serde-protobuf"]`.

    * If you're using the PostgreSQL event store backend and want to use the listener mechanism, you can enable the `listener` feature: `disintegrate-postgres = {version = "0.5.0", features = ["listener"]}`.

2. Define the list of events in your application. You can use the Event Storming technique to identify the events that occur in your system. Here's an example of defining events using Disintegrate:

    ```rust,ignore
    use disintegrate::macros::Event;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, Event, Serialize, Deserialize)]
    #[group(UserEvent, [UserCreated])]
    #[group(CartEvent, [ItemAdded, ItemRemoved, ItemUpdated])]
    pub enum DomainEvent {
        UserCreated {
            #[id]
            user_id: String,
            name: String,
        },
        ItemAdded {
            #[id]
            user_id: String,
            #[id]
            item_id: String,
            quantity: u32,
        },
        ItemRemoved {
            #[id]
            user_id: String,
            #[id]
            item_id: String,
        },
        ItemUpdated {
            #[id]
            user_id: String,
            #[id]
            item_id: String,
            new_quantity: u32,
        },
    }
    ```

    In this example, we define an enum `DomainEvent` using the `#[derive(Event)]` attribute. The enum represents various events that can occur in your application. The `#[group]` attribute specifies the event groups, such as `UserEvent` and `CartEvent`, and their corresponding variants. This allows you to organize events into logical groups. The `#[id]` attribute on fields allows you to specify the domain identifiers of each event, which are used for filtering relevant events for a state.

3. Create a state that implements the `State` trait. A state represents a business concept in your application and contains the information necessary to make decisions based on a group of events. Here's an example of a state:

    ```rust,ignore
    use crate::event::CartEvent;
    use disintegrate::{query, State, StreamQuery};
    use std::collections::HashSet;
    use thiserror::Error;

    #[derive(Clone, Eq, Hash, PartialEq)]
    pub struct Item {
        id: String,
        quantity: u32,
    }

    impl Item {
        fn new(id: String, quantity: u32) -> Self {
            Item { id, quantity }
        }
    }

    #[derive(Default, Clone)]
    pub struct Cart {
        user_id: String,
        items: HashSet<Item>,
    }

    impl State for Cart {
        type Event = CartEvent;

        fn query(&self) -> StreamQuery<Self::Event> {
            query!(CartEvent, user_id == self.user_id)
        }

        fn mutate(&mut self, event: Self::Event) {
            match event {
                CartEvent::ItemAdded {
                    item_id, quantity, ..
                } => {
                    self.items.insert(Item::new(item_id, quantity));
                }
                CartEvent::ItemRemoved { item_id, .. } => {
                    self.items.retain(|item| item.id != *item_id);
                }
                CartEvent::ItemUpdated {
                    item_id,
                    new_quantity,
                    ..
                } => {
                    self.items.replace(Item::new(item_id, new_quantity));
                }
            }
        }
    }

    impl Cart {
        pub fn new(user_id: &str) -> Self {
            Self {
                user_id: user_id.into(),
                items: HashSet::new(),
            }
        }
    }
    ```

    In this example, we define a `Cart` struct that implements the `State` trait. The `Cart` struct represents the state of a shopping cart and keeps track of the items added by users. 

4. Create a struct that implements the `Decision` trait. This struct represents a business decision and is responsible for validating inputs and generating a list of changes. The resulting changes will be stored by the `DecisionMaker` in the event store:
    ```rust,ignore
    #[derive(Debug, Error)]
    pub enum CartError {
        // cart errors
    }

    pub struct AddItem {
        user_id: String,
        item_id: String,
        quantity: u32,
    }

    impl AddItem {
        pub fn new(user_id: String, item_id: String, quantity: u32) -> Self {
            Self {
                user_id,
                item_id,
                quantity,
            }
        }
    }

    /// Implement your business logic
    impl Decision for AddItem {
        type Event = CartEvent;
        type State = Cart;
        type Error = CartError;

        fn default_state(&self) -> Self::State {
            Cart::new(&self.user_id)
        }

        fn validation_query(&self) -> Option<StreamQuery<CartEvent>> {
            None
        }

        fn process(&self, _state: &Self::State) -> Result<Vec<Self::Event>, Self::Error> {
            // check your business constraints...
            Ok(vec![CartEvent::ItemAdded {
                user_id: self.user_id.clone(),
                item_id: self.item_id.to_string(),
                quantity: self.quantity,
            }])
        }
    }
    ```
    In the provided examples, decisions are used as commands that are executed against a state built from the event store. A `Decision` defines the default `State`, which will be mutated using the events contained in the event store.

    In cases where no events are found in the event store, the default `State` is used as a starting point to make the decision. This scenario arises when the decision is taken for the first time, and there is no historical data available to build a `State`.

5. Instantiate an event store, create the `AddItem` decision, and invoke `make` method on `DecisionMaker`:

    ```rust,ignore
    mod cart;
    mod event;

    use cart::AddItem;
    use event::DomainEvent;

    use anyhow::{Ok, Result};
    use disintegrate::{serde::json::Json, DecisionMaker};
    use disintegrate_postgres::PgEventStore;
    use sqlx::{postgres::PgConnectOptions, PgPool};

    #[tokio::main]
    async fn main() -> Result<()> {
        dotenv::dotenv().unwrap();

        // Create a PostgreSQL poll
        let connect_options = PgConnectOptions::new();
        let pool = PgPool::connect_with(connect_options).await?;

        // Create a serde for serialize and deserialize events
        let serde = Json::<DomainEvent>::default();

        // Create a PostgreSQL event store
        let event_store = PgEventStore::new(pool, serde).await?;

        // Create a DecisionMaker 
        let decision_maker = DecisionMaker::new(event_store);

        // Make the decision. This performs the business decision and persists 
        // the changes into the event store
        decision_maker
            .make(AddItem::new("user-1".to_string(), "item-1".to_string(), 4))
            .await?;
        Ok(())
    }
    ```

For a complete example, take a look at `examples` folder to get a better understanding of how to use Disintegrate in a real-world application.

## License

This project is licensed under the [MIT License](LICENSE).

## Contribution

Contributions are welcome! If you find any issues or have suggestions for improvement, please feel free to open an issue or submit a pull request.

Please make sure to follow the [Contributing Guidelines](CONTRIBUTING.md) when making contributions to this project.

We appreciate your help in making Disintegrate better!

## Acknowledgments

Disintegrate is inspired by the ideas presented in the talk [Kill Aggregate!](https://www.youtube.com/watch?v=DhhxKoOpJe0) by Sara Pellegrini, exploring new possibilities for modeling business concepts from event streams. We would like to express our gratitude to the speaker for sharing their insights and sparking innovative thinking within the software development community.

While preserving the core concepts from the video, Disintegrate introduces additional features that enrich the developer experience and bring the ideas into practical implementation:

1. **Postgres implementation**: Disintegrate provides a working implementation of the concepts discussed in the video.

2. **Powerful query system**: In the video, queries were constructed using a list of domain identifiers and event types. Disintegrate takes this capability to the next level by empowering developers to create more sophisticated queries that can address advanced use cases.

3. **Validation queries**: While acknowledging the value of the video's approach in utilizing a query to validate the state's integrity, Disintegrate takes it a step further to enhance this aspect. In the video, the same query was used for both building the state and the append API. However, Disintegrate introduces a powerful feature known as `Validation` queries, which empowers developers with fine-grained control over decision invalidation when new events are stored in the event store. This proves particularly useful in scenarios such as the banking example. For example, when making a withdrawal decision, the account balance needs to be computed, requiring the inclusion of deposit events in the state. However, a deposit event should not invalidate a withdrawal decision, even if it changes the state. In such cases, validation must be performed on a subset of events necessary for building the state.

4. **Decision concept**: Disintegrate introduces the concept of `Decision`, which serve as building block for developing application business logic while adhering to the SOLID principles. A `Decision` can be seen as small aggregate that focus on specific use case. Consequently, when a new use case emerges, it is possible to extend the application by adding a new `Decision` without modifying existing ones.
