use disintegrate::Event;
use disintegrate_serde::Serializer;
use serde::Serialize;
use sqlx::{PgPool, Row};

use super::insert_builder::InsertBuilder;

mod event_store;
mod state_store;

pub async fn insert_events<E: Event + Clone + Serialize>(pool: &PgPool, events: &[E]) {
    for event in events {
        let mut sequence_insert = InsertBuilder::new(event, "event_sequence").returning("event_id");
        let row = sequence_insert.build().fetch_one(pool).await.unwrap();
        let payload =
            disintegrate_serde::serde::json::Json::<E>::default().serialize(event.clone());

        let mut event_insert = InsertBuilder::new(event, "event")
            .with_id(row.get(0))
            .with_payload(&payload);
        event_insert.build().execute(pool).await.unwrap();
    }
}
