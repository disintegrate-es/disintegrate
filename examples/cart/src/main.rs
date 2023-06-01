mod cart;
mod event;

use cart::Cart;
use event::DomainEvent;

use anyhow::{Ok, Result};
use disintegrate::{serde::json::Json, StateStore};
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

    // Hydrate the `Cart` from the event store
    let user_id = "user-1";
    let mut cart = event_store.hydrate(Cart::new(user_id)).await?;

    // Invoke add item method on the hydrated cart
    cart.add_item("item-1", 4)?;

    // Save the new state
    event_store.save(&mut cart).await?;
    Ok(())
}
