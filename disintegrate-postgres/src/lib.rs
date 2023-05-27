//! # PostgreSQL Disintegrate Backend Library
mod error;
mod event_store;
#[cfg(feature = "listener")]
mod listener;
mod state_store;

pub use crate::event_store::PgEventStore;
#[cfg(feature = "listener")]
pub use crate::listener::{PgEventListener, PgEventListenerConfig};
pub use error::Error;

/// Initializes the PostgreSQL DB
///
/// It creates all the tables, indexes, functions, and triggers used by the Postgres event store.
/// If the `listener` feature is enabled, it also initializes the DB for the Postgres event listener.
pub async fn setup(pool: &sqlx::PgPool) -> Result<(), Error> {
    crate::event_store::setup(pool).await?;
    #[cfg(feature = "listener")]
    crate::listener::setup(pool).await?;
    Ok(())
}
