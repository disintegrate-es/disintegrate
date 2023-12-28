mod cart;
mod event;

use cart::AddItem;
use event::DomainEvent;

use anyhow::{Ok, Result};
use disintegrate::serde::json::Json;
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

    // Create a Postgres DecisionMaker
    let decision_maker = disintegrate_postgres::decision_maker(event_store);

    // Make the decision. This performs the business decision and persists the changes into the
    // event store
    decision_maker
        .make(AddItem::new("user-1".to_string(), "item-1".to_string(), 4))
        .await?;
    Ok(())
}
