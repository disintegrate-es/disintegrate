use super::append::{InsertEventSequenceBuilder, InsertEventsBuilder};
use crate::{Error, PgEventId, PgEventStore};
use disintegrate::{
    domain_identifiers, ident, query, DomainIdentifierInfo, DomainIdentifierSet, Event, EventInfo,
    EventSchema, EventStore, IdentifierType, PersistedEvent,
};
use disintegrate_serde::serde::json::Json;
use disintegrate_serde::Deserializer;
use futures::StreamExt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
enum ShoppingCartEvent {
    Added { product_id: String, cart_id: String },
    Removed { product_id: String, cart_id: String },
}
fn added_event(product_id: &str, cart_id: &str) -> ShoppingCartEvent {
    ShoppingCartEvent::Added {
        product_id: product_id.to_string(),
        cart_id: cart_id.to_string(),
    }
}
fn removed_event(product_id: &str, cart_id: &str) -> ShoppingCartEvent {
    ShoppingCartEvent::Removed {
        product_id: product_id.to_string(),
        cart_id: cart_id.to_string(),
    }
}

impl Event for ShoppingCartEvent {
    const SCHEMA: EventSchema = EventSchema {
        events: &["ShoppingCartAdded", "ShoppingCartRemoved"],
        events_info: &[
            &EventInfo {
                name: "ShoppingCartAdded",
                domain_identifiers: &[&ident!(#product_id), &ident!(#cart_id)],
            },
            &EventInfo {
                name: "ShoppingCartRemoved",
                domain_identifiers: &[&ident!(#product_id), &ident!(#cart_id)],
            },
        ],
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
            ShoppingCartEvent::Added {
                product_id,
                cart_id,
                ..
            } => domain_identifiers! {product_id: product_id, cart_id: cart_id},
            ShoppingCartEvent::Removed {
                product_id,
                cart_id,
                ..
            } => domain_identifiers! {product_id: product_id, cart_id: cart_id},
        }
    }
}

#[sqlx::test]
async fn it_queries_events(pool: PgPool) {
    let event_store = PgEventStore::<ShoppingCartEvent, Json<ShoppingCartEvent>>::new(
        pool.clone(),
        Json::default(),
    )
    .await
    .unwrap();

    let events = vec![
        added_event("product_1", "cart_1"),
        removed_event("product_1", "cart_1"),
        added_event("product_2", "cart_1"),
        added_event("product_2", "cart_1"),
    ];
    insert_events(&pool, &events).await;

    // Test the stream function
    let query = query!(ShoppingCartEvent; product_id == "product_1");
    let result = event_store.stream(&query).collect::<Vec<_>>().await;

    assert_eq!(result.len(), 2);
}

#[sqlx::test]
async fn it_appends_events(pool: PgPool) {
    let event_store = PgEventStore::<ShoppingCartEvent, Json<ShoppingCartEvent>>::new(
        pool.clone(),
        Json::default(),
    )
    .await
    .unwrap();
    let events: Vec<ShoppingCartEvent> = vec![
        added_event("product_1", "cart_1"),
        removed_event("product_2", "cart_1"),
    ];

    let query = query!(ShoppingCartEvent; cart_id == "cart_1");

    event_store.append(events, query.clone(), 0).await.unwrap();

    let stored_events = sqlx::query("SELECT event_id, event_type, payload FROM event")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(stored_events.len(), 2);
    assert_event_row(
        stored_events.first().unwrap(),
        1,
        "ShoppingCartAdded",
        added_event("product_1", "cart_1"),
    );
    assert_event_row(
        stored_events.get(1).unwrap(),
        2,
        "ShoppingCartRemoved",
        removed_event("product_2", "cart_1"),
    );
}

#[sqlx::test]
async fn it_appends_unchecked(pool: PgPool) {
    let event_store = PgEventStore::<ShoppingCartEvent, Json<ShoppingCartEvent>>::new(
        pool.clone(),
        Json::default(),
    )
    .await
    .unwrap();
    let events: Vec<ShoppingCartEvent> = vec![
        added_event("product_1", "cart_1"),
        removed_event("product_2", "cart_1"),
    ];

    event_store.append_without_validation(events).await.unwrap();

    let stored_events = sqlx::query("SELECT event_id, event_type, payload FROM event")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(stored_events.len(), 2);
    assert_event_row(
        stored_events.first().unwrap(),
        1,
        "ShoppingCartAdded",
        added_event("product_1", "cart_1"),
    );
    assert_event_row(
        stored_events.get(1).unwrap(),
        2,
        "ShoppingCartRemoved",
        removed_event("product_2", "cart_1"),
    );
}

#[track_caller]
fn assert_event_row(
    row: &PgRow,
    event_id: PgEventId,
    event_type: &str,
    payload: ShoppingCartEvent,
) {
    let stored_event_id: PgEventId = row.get(0);
    assert_eq!(stored_event_id, event_id);
    let stored_event_type: String = row.get(1);
    assert_eq!(stored_event_type, event_type);
    let stored_payload: Vec<u8> = row.get(2);
    assert_eq!(
        Json::<ShoppingCartEvent>::default()
            .deserialize(stored_payload)
            .unwrap(),
        payload
    );
}

#[sqlx::test]
async fn it_returns_a_concurrency_error_when_it_appends_events_of_a_query_which_its_events_have_been_changed_and_event_store_is_empty(
    pool: PgPool,
) {
    let event_store =
        PgEventStore::<ShoppingCartEvent, Json<ShoppingCartEvent>>::new(pool, Json::default())
            .await
            .unwrap();

    let query = query!(ShoppingCartEvent; product_id == "product_1", cart_id == "cart_1");
    event_store
        .append(vec![added_event("product_1", "cart_1")], query, 0)
        .await
        .unwrap();
    let query = query!(ShoppingCartEvent; product_id == "product_1", cart_id == "cart_1");
    let result = event_store
        .append(vec![removed_event("product_1", "cart_1")], query, 0)
        .await;
    assert!(matches!(result, Err(Error::Concurrency)));
}

#[sqlx::test]
async fn it_returns_a_concurrency_error_when_it_appends_events_of_a_query_which_its_events_have_been_changed(
    pool: PgPool,
) {
    let event_store = PgEventStore::<ShoppingCartEvent, Json<ShoppingCartEvent>>::new(
        pool.clone(),
        Json::default(),
    )
    .await
    .unwrap();
    let events = vec![
        added_event("product_1", "cart_1"),
        removed_event("product_1", "cart_1"),
    ];
    insert_events(&pool, &events).await;

    let query_1 = query!(ShoppingCartEvent; cart_id == "cart_1");
    let query_1_result = event_store
        .stream(&query_1)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let query_2 = query!(ShoppingCartEvent; product_id == "product_1", cart_id == "cart_1");
    let query_2_result = event_store
        .stream(&query_2)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let _result = event_store
        .append(
            vec![removed_event("product_1", "cart_1")],
            query_1,
            query_1_result.last().unwrap().id(),
        )
        .await
        .unwrap();

    let result = event_store
        .append(
            vec![removed_event("product_1", "cart_1")],
            query_2,
            query_2_result.last().unwrap().id(),
        )
        .await;

    assert!(matches!(result, Err(Error::Concurrency)));
}

pub async fn insert_events<E: Event + Clone + Serialize + DeserializeOwned>(
    pool: &PgPool,
    events: &[E],
) {
    let mut persisted_events = Vec::with_capacity(events.len());
    for event in events {
        let mut event_sequence_insert = InsertEventSequenceBuilder::new(event)
            .with_consumed(true)
            .with_committed(true);
        let row = event_sequence_insert.build().fetch_one(pool).await.unwrap();
        persisted_events.push(PersistedEvent::new(row.get(0), event.clone()));
    }
    let serde = disintegrate_serde::serde::json::Json::default();
    InsertEventsBuilder::new(persisted_events.as_slice(), &serde)
        .build()
        .execute(pool)
        .await
        .unwrap();
}
