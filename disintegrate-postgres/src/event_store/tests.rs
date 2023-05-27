use disintegrate::Event;
use disintegrate_serde::Serializer;
use serde::Serialize;
use serde_json::json;
use sqlx::{query, types::Json, PgPool};

mod event_store;
mod state_store;

pub async fn insert_events<E: Event + Clone + Serialize>(pool: &PgPool, events: &[E]) {
    for event in events {
        let (id,) = sqlx::query_as::<_, (i64,)>(
            "INSERT INTO event_sequence (event_type, domain_identifiers)
                VALUES ($1, $2)
                RETURNING id",
        )
        .bind(event.name())
        .bind(Json(event.domain_identifiers()))
        .fetch_one(pool)
        .await
        .unwrap();
        let domain_identifiers = json!(event.domain_identifiers());
        let payload =
            disintegrate_serde::serde::json::Json::<E>::default().serialize(event.clone());

        query("INSERT INTO event (id, event_type, domain_identifiers, payload) VALUES ($1, $2, $3, $4)")
            .bind(id)
            .bind(event.name())
            .bind(&domain_identifiers)
            .bind(&payload)
            .execute(pool)
            .await
            .unwrap();
    }
}
