//! An `EventListener` implementation for indexing existing fields tagged with `#[id]`.
use std::{collections::BTreeMap, marker::PhantomData};

use async_trait::async_trait;
use disintegrate::{DomainIdentifierSet, Event, EventListener, PersistedEvent, StreamQuery};
use sqlx::{PgPool, Postgres};

use crate::PgEventId;

/// The `PgIdIndexer` is a helper to index existing fields that have been newly tagged with the `#[id]` attribute in events.
///
/// # Overview
///
/// The `PgIdIndexer` is an `EventListener` responsible for indexing fields in the event store
/// that are tagged with the `#[id]` attribute. This allows querying of old events based
/// on this new domain identifier once the indexing is complete. The `PgIdIndexer` listens to events and updates the
/// `event` table in the database with the appropriate values.
///
/// # Workflow
///
/// After you have tagged an existing field in your event structure with the `#[id]` attribute to mark it
/// as a domain identifier:
///
///   ```rust
///   use disintegrate_macros::Event;
///   use serde::{Serialize, Deserialize};
///   #[derive(Event, Clone, Serialize, Deserialize)]
///   struct MyEvent {
///       #[id]
///       existing_id: String,
///       other_field: String,
///   }
///   ```
///
/// 1. **Register the `PgIdIndexer` as an `EventListener`**: Integrate the indexer
///    with the event listener system to process the newly tagged domain identifier:
///
///    ```rust
///    use disintegrate_postgres::PgIdIndexer;
///    use disintegrate_postgres::PgEventListenerConfig;
///    use disintegrate_postgres::PgEventListener;
///    use disintegrate_postgres::PgEventStore;
///    use std::time::Duration;
///    use disintegrate_macros::Event;
///    use disintegrate::serde::json::Json;
///    use serde::{Serialize, Deserialize};
///    use sqlx::PgPool;
///
///    #[derive(Event, Clone, Serialize, Deserialize)]
///    struct MyEvent {
///        #[id]
///        existing_id: String,
///        other_field: String,
///    }
///
///    async fn setup_listener(pool: PgPool, event_store: PgEventStore<MyEvent, Json<MyEvent>>) {
///        let id_indexer = PgIdIndexer::<MyEvent>::new("index_exsting_id", pool);
///        PgEventListener::builder(event_store)
///            .register_listener(
///                id_indexer,
///                PgEventListenerConfig::poller(Duration::from_secs(5)).with_notifier()
///            )
///            .start_with_shutdown(shutdown())
///            .await
///            .expect("start event listener failed");
///    }
///
///    async fn shutdown() {
///        tokio::signal::ctrl_c().await.expect("ctrl_c signal failed");
///    }
///    ```
///
/// 2. **Deploy the application**: Start the application with the updated event
///    structure and the `PgIdIndexer` integration. Newly created events with the new
///    domain identifier will automatically have the identifier indexed.
///
/// Once the indexing process is complete, you can query the event store using the
/// new domain identifier to fetch events.
///
/// If indexing is done, you can remove the `PgIdIndexer` from the list of registered event listeners.
pub struct PgIdIndexer<E: Event + Clone> {
    id: &'static str,
    pool: PgPool,
    query: StreamQuery<PgEventId, E>,
    _event: PhantomData<E>,
}

impl<E: Event + Clone> PgIdIndexer<E> {
    /// Creates a new `PgIdIndexer` instance for indexing events.
    ///
    /// # Arguments
    ///
    /// * `id` - A unique identifier for the listener, used to store the last processed `event_id` in the database.
    /// * `pool` - A `PgPool` instance for Postgres.
    pub fn new(id: &'static str, pool: PgPool) -> Self {
        Self {
            id,
            pool,
            query: disintegrate::query!(E),
            _event: PhantomData,
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub struct Error(#[from] sqlx::Error);

#[async_trait]
impl<E: Event + Clone + Send + Sync> EventListener<PgEventId, E> for PgIdIndexer<E> {
    type Error = Error;

    fn id(&self) -> &'static str {
        self.id
    }

    fn query(&self) -> &StreamQuery<PgEventId, E> {
        &self.query
    }

    async fn handle(&self, event: PersistedEvent<PgEventId, E>) -> Result<(), Self::Error> {
        let mut query_builder = sql_builder(event.id(), event.domain_identifiers());
        query_builder.build().execute(&self.pool).await?;
        Ok(())
    }
}

fn sql_builder(
    event_id: PgEventId,
    domain_identifiers: DomainIdentifierSet,
) -> sqlx::QueryBuilder<'static, Postgres> {
    let domain_identifiers = <BTreeMap<_, _> as Clone>::clone(&domain_identifiers).into_iter();
    let mut sql_builder = sqlx::QueryBuilder::new("UPDATE event SET ");
    let mut separated = sql_builder.separated(",");
    for (id_name, id_value) in domain_identifiers {
        separated.push(format!("{id_name} = "));

        match id_value {
            disintegrate::IdentifierValue::String(value) => {
                separated.push_bind_unseparated(value.clone())
            }
            disintegrate::IdentifierValue::i64(value) => separated.push_bind_unseparated(value),
            disintegrate::IdentifierValue::Uuid(value) => separated.push_bind_unseparated(value),
        };
    }
    separated.push_unseparated(" WHERE event_id = ");
    separated.push_bind_unseparated(event_id);

    sql_builder
}

#[cfg(test)]
mod test {
    use disintegrate::domain_identifiers;
    use uuid::Uuid;

    use super::sql_builder;

    #[test]
    fn it_builds_event_update() {
        let ids =
            domain_identifiers! {cart_id: "cart1", product_id: 1, customer_id: Uuid::new_v4()};

        let builder = sql_builder(1, ids);

        assert_eq!(
            builder.sql(),
            "UPDATE event SET cart_id = $1,customer_id = $2,product_id = $3 WHERE event_id = $4"
        );
    }
}
