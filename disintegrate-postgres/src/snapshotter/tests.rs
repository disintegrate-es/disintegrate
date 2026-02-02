use disintegrate::{
    domain_ids, ident, query, DomainIdInfo, DomainIdSet, Event, EventId,
    EventInfo, EventSchema, IdentifierType, IntoState, IntoStatePart, PersistedEvent, StateMutate,
};
use disintegrate_serde::{serde::json::Json, Deserializer};
use serde::Deserialize;
use sqlx::PgPool;

use super::*;

#[derive(Clone)]
enum CartEvent {
    #[allow(dead_code)]
    ItemAdded { cart_id: String, item_id: String },
}

impl Event for CartEvent {
    const SCHEMA: EventSchema = EventSchema {
        events: &["CartEventItemAdded"],
        events_info: &[&EventInfo {
            name: "CartProductAdded",
            domain_ids: &[&ident!(#cart_id), &ident!(#product_id)],
        }],
        domain_ids: &[
            &DomainIdInfo {
                ident: ident!(#cart_id),
                type_info: IdentifierType::String,
            },
            &DomainIdInfo {
                ident: ident!(#product_id),
                type_info: IdentifierType::String,
            },
        ],
    };
    fn name(&self) -> &'static str {
        match self {
            CartEvent::ItemAdded { .. } => "CartProductAdded",
        }
    }
    fn domain_ids(&self) -> DomainIdSet {
        match self {
            CartEvent::ItemAdded {
                item_id, cart_id, ..
            } => domain_ids! {item_id: item_id, cart_id: cart_id},
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CartState {
    cart_id: String,
    items: Vec<String>,
}

impl CartState {
    fn new<const N: usize>(cart_id: &str, items: [&str; N]) -> Self {
        Self {
            cart_id: cart_id.to_string(),
            items: items.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl StateQuery for CartState {
    const NAME: &'static str = "cart-state";
    type Event = CartEvent;

    fn query<ID: EventId>(&self) -> disintegrate::StreamQuery<ID, Self::Event> {
        query!(CartEvent; cart_id == self.cart_id)
    }
}

impl StateMutate for CartState {
    fn mutate(&mut self, event: Self::Event) {
        match event {
            CartEvent::ItemAdded { item_id, .. } => self.items.push(item_id),
        }
    }
}

#[derive(sqlx::FromRow)]
struct SnapshotRow {
    id: Uuid,
    name: String,
    query: String,
    version: PgEventId,
    payload: String,
}

#[sqlx::test]
async fn it_stores_snapshots(pool: PgPool) {
    let snapshotter = PgSnapshotter::try_new(pool.clone(), 0).await.unwrap();
    let mut state = CartState::new("c1", []).into_state_part();

    state.mutate_part(
        PersistedEvent::new(
            1,
            CartEvent::ItemAdded {
                cart_id: "c1".to_string(),
                item_id: "p1".to_string(),
            },
        )
        .into(),
    );

    snapshotter.store_snapshot(&state.clone()).await.unwrap();

    let stored_snapshot = sqlx::query_as::<_, SnapshotRow>("SELECT * FROM snapshot")
        .fetch_one(&pool)
        .await
        .unwrap();

    let query_key = query_key(&state.query());
    let snapshot_id = snapshot_id(CartState::NAME, &query_key);
    assert_eq!(stored_snapshot.id, snapshot_id);
    assert_eq!(stored_snapshot.name, CartState::NAME);
    assert_eq!(stored_snapshot.query, query_key);
    assert_eq!(
        Json::<CartState>::default()
            .deserialize(stored_snapshot.payload.into_bytes())
            .unwrap(),
        state.into_state()
    );
    assert_eq!(stored_snapshot.version, 1);
}

#[sqlx::test]
async fn it_loads_snapshots(pool: PgPool) {
    let snapshotter = PgSnapshotter::try_new(pool.clone(), 2).await.unwrap();
    let default_state = CartState::new("c1", []);
    let expected_state = CartState::new("c1", ["p1", "p2"]);
    let query_key = query_key(&default_state.query());
    let snapshot_id = snapshot_id(CartState::NAME, &query_key);
    sqlx::query("INSERT INTO snapshot (id, name, query, payload, version) VALUES ($1,$2,$3,$4,$5) ON CONFLICT(id) DO UPDATE SET name = $2, query = $3, payload = $4, version = $5 WHERE snapshot.version < $5")
        .bind(snapshot_id)
        .bind(CartState::NAME)
        .bind(query_key)
        .bind(serde_json::to_string(&expected_state).unwrap())
        .bind(3)
        .execute(&pool)
        .await.unwrap();

    let loaded_state = snapshotter
        .load_snapshot(default_state.into_state_part())
        .await;

    assert_eq!(loaded_state.version(), 3);
    assert_eq!(loaded_state.into_state(), expected_state);
}
