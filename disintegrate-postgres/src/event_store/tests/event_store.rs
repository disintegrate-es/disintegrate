use super::*;
use crate::event_store::setup;
use crate::{Error, PgEventStore};
use disintegrate::EventStore;
use disintegrate::{domain_identifiers, query, DomainIdentifierSet, Event};
use disintegrate_serde::serde::json::Json;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
enum ShoppingCartEvent {
    Added {
        product_id: String,
        cart_id: String,
        quantity: i64,
    },
    Removed {
        product_id: String,
        cart_id: String,
        quantity: i64,
    },
}

impl Event for ShoppingCartEvent {
    const NAMES: &'static [&'static str] = &["ShoppingCartAdded", "ShoppingCartRemoved"];
    fn name(&self) -> &'static str {
        match self {
            ShoppingCartEvent::Added { .. } => "ShoppingCartAdded",
            ShoppingCartEvent::Removed { .. } => "ShoppingCartRemoved",
        }
    }
    fn domain_identifiers(&self) -> DomainIdentifierSet {
        match self {
            ShoppingCartEvent::Added {
                product_id,
                cart_id,
                ..
            } => domain_identifiers! {product_id: product_id.clone(), cart_id: cart_id.clone()},
            ShoppingCartEvent::Removed {
                product_id,
                cart_id,
                ..
            } => domain_identifiers! {product_id: product_id.clone(), cart_id: cart_id.clone()},
        }
    }
}

#[sqlx::test]
async fn it_queries_events(pool: PgPool) {
    setup(&pool).await.unwrap();
    let events = vec![
        ShoppingCartEvent::Added {
            product_id: "product_1".to_string(),
            cart_id: "cart_1".to_string(),
            quantity: 5,
        },
        ShoppingCartEvent::Removed {
            product_id: "product_1".to_string(),
            cart_id: "cart_1".to_string(),
            quantity: 1,
        },
        ShoppingCartEvent::Added {
            product_id: "product_2".to_string(),
            cart_id: "cart_1".to_string(),
            quantity: 5,
        },
        ShoppingCartEvent::Added {
            product_id: "product_2".to_string(),
            cart_id: "cart_1".to_string(),
            quantity: 1,
        },
    ];
    insert_events(&pool, &events).await;

    let event_store =
        PgEventStore::<ShoppingCartEvent, Json<ShoppingCartEvent>>::new(pool, Json::default());

    // Test the stream function
    let query = query!(ShoppingCartEvent, product_id == "product_1");
    let result = event_store
        .stream(&query)
        .unwrap()
        .collect::<Vec<_>>()
        .await;

    assert_eq!(result.len(), 2);
}

#[sqlx::test]
async fn it_appends_events(pool: PgPool) {
    setup(&pool).await.unwrap();
    let events: Vec<ShoppingCartEvent> = vec![ShoppingCartEvent::Added {
        product_id: "product_1".to_string(),
        cart_id: "cart_1".to_string(),
        quantity: 1,
    }];

    let query = query!(
        ShoppingCartEvent,
            (product_id == "product_1") or
            (cart_id == "cart_1")
    );
    let event_store =
        PgEventStore::<ShoppingCartEvent, Json<ShoppingCartEvent>>::new(pool, Json::default());

    let result = event_store.append(events, query, 0).await.unwrap();

    assert_eq!(result.len(), 1);

    let persisted_event = result.first().unwrap();
    assert_eq!(persisted_event.name(), "ShoppingCartAdded");
    assert_eq!(
        persisted_event.domain_identifiers(),
        domain_identifiers! {product_id: "product_1".to_string(), cart_id: "cart_1".to_string()},
    );
}

#[sqlx::test]
async fn it_returns_a_concurrency_error_when_it_appends_events_of_a_query_which_its_events_have_been_changed_and_event_store_is_empty(
    pool: PgPool,
) {
    setup(&pool).await.unwrap();
    let event_store =
        PgEventStore::<ShoppingCartEvent, Json<ShoppingCartEvent>>::new(pool, Json::default());

    let query = query!(
        ShoppingCartEvent,
            (product_id == "product_1") or
            (cart_id == "cart_1")
    );
    let _result = event_store
        .append(
            vec![ShoppingCartEvent::Added {
                product_id: "product_1".to_string(),
                cart_id: "cart_1".to_string(),
                quantity: 1,
            }],
            query,
            0,
        )
        .await
        .unwrap();
    let query = query!(
        ShoppingCartEvent,
            (product_id == "product_1") or
            (cart_id == "cart_1")
    );
    let result = event_store
        .append(
            vec![ShoppingCartEvent::Removed {
                product_id: "product_1".to_string(),
                cart_id: "cart_1".to_string(),
                quantity: 1,
            }],
            query,
            0,
        )
        .await;
    assert!(matches!(result, Err(Error::Concurrency)));
}

#[sqlx::test]
async fn it_returns_a_concurrency_error_when_it_appends_events_of_a_query_which_its_events_have_been_changed(
    pool: PgPool,
) {
    setup(&pool).await.unwrap();
    let events = vec![
        ShoppingCartEvent::Added {
            product_id: "product_1".to_string(),
            cart_id: "cart_1".to_string(),
            quantity: 2,
        },
        ShoppingCartEvent::Removed {
            product_id: "product_1".to_string(),
            cart_id: "cart_1".to_string(),
            quantity: 1,
        },
    ];
    insert_events(&pool, &events).await;

    let event_store =
        PgEventStore::<ShoppingCartEvent, Json<ShoppingCartEvent>>::new(pool, Json::default());

    let query_1 = query!(ShoppingCartEvent, cart_id == "cart_1");
    let query_1_result = event_store
        .stream(&query_1)
        .unwrap()
        .collect::<Vec<_>>()
        .await;

    let query_2 = query!(
        ShoppingCartEvent,
            (product_id == "product_1") or
            (cart_id == "cart_1")

    );
    let query_2_result = event_store
        .stream(&query_2)
        .unwrap()
        .collect::<Vec<_>>()
        .await;

    let _result = event_store
        .append(
            vec![ShoppingCartEvent::Removed {
                product_id: "product_1".to_string(),
                cart_id: "cart_1".to_string(),
                quantity: 1,
            }],
            query_1,
            query_1_result.last().unwrap().id(),
        )
        .await
        .unwrap();

    let result = event_store
        .append(
            vec![ShoppingCartEvent::Removed {
                product_id: "product_1".to_string(),
                cart_id: "cart_1".to_string(),
                quantity: 1,
            }],
            query_2,
            query_2_result.last().unwrap().id(),
        )
        .await;

    assert!(matches!(result, Err(Error::Concurrency)));
}
