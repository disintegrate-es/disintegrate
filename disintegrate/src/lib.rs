pub mod domain_identifier;
mod event;
mod event_store;
pub mod identifier;
mod listener;
pub mod state;
pub mod stream_query;
mod testing;

pub use crate::domain_identifier::{DomainIdentifier, DomainIdentifierSet};
pub use crate::event::{Event, PersistedEvent};
pub use crate::event_store::EventStore;
pub use crate::identifier::Identifier;
pub use crate::listener::EventListener;
pub use crate::state::{State, StateStore};
pub use crate::stream_query::{query, StreamQuery};
pub use crate::testing::TestHarness;

#[cfg(feature = "macros")]
pub mod macros {
    pub use disintegrate_macros::Event;
}

#[cfg(feature = "serde")]
pub mod serde {
    #[cfg(feature = "serde-avro")]
    pub use disintegrate_serde::serde::avro;
    #[cfg(feature = "serde-json")]
    pub use disintegrate_serde::serde::json;
    #[cfg(feature = "serde-prost")]
    pub use disintegrate_serde::serde::prost;
    #[cfg(feature = "serde-protobuf")]
    pub use disintegrate_serde::serde::protobuf;
    pub use disintegrate_serde::{Deserializer, Serde, Serializer};
}
