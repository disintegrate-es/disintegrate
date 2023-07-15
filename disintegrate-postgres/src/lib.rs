//! # PostgreSQL Disintegrate Backend Library
mod error;
mod event_store;
#[cfg(feature = "listener")]
mod listener;

pub use crate::event_store::PgEventStore;
#[cfg(feature = "listener")]
pub use crate::listener::{PgEventListener, PgEventListenerConfig};
pub use error::Error;
