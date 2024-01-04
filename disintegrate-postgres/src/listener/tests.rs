use super::*;

use async_trait::async_trait;
use disintegrate::{
    domain_identifiers, ident, query, DomainIdentifierInfo, DomainIdentifierSet, EventSchema,
    EventStore, IdentifierType, PersistedEvent, StreamQuery,
};
use disintegrate_serde::serde::json::Json;

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
enum ShoppingCartEvent {
    Added(CartEventPayload),
    Removed(CartEventPayload),
}

impl Event for ShoppingCartEvent {
    const SCHEMA: EventSchema = EventSchema {
        types: &["ShoppingCartAdded", "ShoppingCartRemoved"],
        domain_identifiers: &[
            &DomainIdentifierInfo {
                ident: ident!(#cart_id),
                type_info: IdentifierType::String,
            },
            &DomainIdentifierInfo {
                ident: ident!(#product_id),
                type_info: IdentifierType::String,
            },
        ],
    };

    fn name(&self) -> &'static str {
        match self {
            ShoppingCartEvent::Added { .. } => "ShoppingCartAdded",
            ShoppingCartEvent::Removed { .. } => "ShoppingCartRemoved",
        }
    }
    fn domain_identifiers(&self) -> DomainIdentifierSet {
        match self {
            ShoppingCartEvent::Added(payload) | ShoppingCartEvent::Removed(payload) => {
                domain_identifiers! {product_id: payload.product_id, cart_id: payload.cart_id}
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CartEventPayload {
    cart_id: String,
    product_id: String,
    quantity: i64,
}

#[derive(FromRow)]
struct Cart {
    cart_id: String,
    product_id: String,
    quantity: i32,
}

impl Cart {
    async fn carts(pool: &PgPool) -> Result<Vec<Cart>, sqlx::Error> {
        sqlx::query_as::<_, Cart>("SELECT cart_id, product_id, quantity FROM carts")
            .fetch_all(pool)
            .await
    }
}

struct CartEventHandler {
    query: StreamQuery<ShoppingCartEvent>,
    pool: PgPool,
}

impl CartEventHandler {
    async fn new(pool: PgPool) -> Result<Self, sqlx::Error> {
        sqlx::query(
            r#"
        CREATE TABLE IF NOT EXISTS carts (
           product_id TEXT,
           cart_id TEXT,
           quantity INT 
        )"#,
        )
        .execute(&pool)
        .await?;
        Ok(Self {
            query: query!(ShoppingCartEvent),
            pool,
        })
    }
}

#[async_trait]
impl EventListener<ShoppingCartEvent> for CartEventHandler {
    type Error = sqlx::Error;
    fn id(&self) -> &'static str {
        "carts"
    }

    fn query(&self) -> &StreamQuery<ShoppingCartEvent> {
        &self.query
    }

    async fn handle(
        &self,
        persisted_event: PersistedEvent<ShoppingCartEvent>,
    ) -> Result<(), Self::Error> {
        match persisted_event.into_inner() {
            ShoppingCartEvent::Added(payload) => {
                sqlx::query("INSERT INTO carts (cart_id, product_id, quantity) VALUES($1, $2, $3)")
                    .bind(payload.cart_id.clone())
                    .bind(payload.product_id.clone())
                    .bind(payload.quantity)
                    .execute(&self.pool)
                    .await
                    .unwrap();
            }
            ShoppingCartEvent::Removed(_) => unimplemented!(),
        }
        Ok(())
    }
}

#[sqlx::test]
async fn it_handles_events(pool: PgPool) {
    let event_store = PgEventStore::<ShoppingCartEvent, Json<ShoppingCartEvent>>::new(
        pool.clone(),
        Json::default(),
    )
    .await
    .unwrap();

    let event_handler_executor = PgEventListerExecutor::new(
        event_store.clone(),
        CartEventHandler::new(pool.clone()).await.unwrap(),
        PgEventListenerConfig::poller(Duration::from_secs(1)),
    );

    let cart_id = "cart_1".to_string();
    let product_id = "product_1".to_string();
    let query = query!(
        ShoppingCartEvent,
        (cart_id == cart_id) or (product_id == product_id)
    );
    let _result = event_store
        .append(
            vec![ShoppingCartEvent::Added(CartEventPayload {
                cart_id,
                product_id,
                quantity: 1,
            })],
            query,
            0,
        )
        .await
        .unwrap();
    event_handler_executor.handle_events_from(0).await.unwrap();

    let carts = Cart::carts(&pool).await.unwrap();
    assert_eq!(carts.len(), 1);
    let first_row = carts.first().unwrap();
    assert_eq!("cart_1", &first_row.cart_id);
    assert_eq!("product_1", &first_row.product_id);
    assert_eq!(1, first_row.quantity);
}

#[sqlx::test]
async fn it_runs_event_listeners(pool: PgPool) {
    let event_store = PgEventStore::<ShoppingCartEvent, Json<ShoppingCartEvent>>::new(
        pool.clone(),
        Json::default(),
    )
    .await
    .unwrap();

    let event_listener = PgEventListener::builder(event_store.clone())
        .register_listener(
            CartEventHandler::new(pool.clone()).await.unwrap(),
            PgEventListenerConfig::poller(Duration::from_millis(50)),
        )
        .start_with_shutdown(async {
            tokio::time::sleep(Duration::from_millis(400)).await;
        });

    let cart_id = "cart_1".to_string();
    let product_id = "product_1".to_string();
    let query = query!(
        ShoppingCartEvent,
        (cart_id == cart_id) or (product_id == product_id)
    );
    let append_handle = event_store.append(
        vec![ShoppingCartEvent::Added(CartEventPayload {
            cart_id,
            product_id,
            quantity: 1,
        })],
        query,
        0,
    );

    let (_, append_result) = tokio::join!(event_listener, append_handle);

    assert!(append_result.is_ok());
    let carts = Cart::carts(&pool).await.unwrap();
    assert_eq!(carts.len(), 1);
    let first_row = carts.first().unwrap();
    assert_eq!("cart_1", &first_row.cart_id);
    assert_eq!("product_1", &first_row.product_id);
    assert_eq!(1, first_row.quantity);
}
