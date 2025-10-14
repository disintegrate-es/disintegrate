#![doc = include_str!("../README.md")]

mod decision;
mod domain_identifier;
mod event;
mod event_store;
mod identifier;
mod listener;
mod state;
mod state_store;
mod stream_query;
mod testing;
pub mod utils;

#[doc(inline)]
pub use crate::decision::{Decision, DecisionMaker, Error as DecisionError, PersistDecision};
#[doc(inline)]
pub use crate::domain_identifier::{DomainIdentifier, DomainIdentifierSet};
#[doc(inline)]
pub use crate::event::{
    DomainIdentifierInfo, Event, EventId, EventInfo, EventSchema, PersistedEvent,
};
#[doc(inline)]
pub use crate::event_store::{EventStore, StreamItem};
#[doc(inline)]
pub use crate::identifier::{Identifier, IdentifierType, IdentifierValue, IntoIdentifierValue};
#[doc(inline)]
pub use crate::listener::EventListener;
#[doc(inline)]
pub use crate::state::{IntoState, IntoStatePart, MultiState, StateMutate, StatePart, StateQuery};
#[doc(inline)]
pub use crate::state_store::{
    EventSourcedStateStore, LoadState, LoadedState, NoSnapshot, SnapshotConfig, StateSnapshotter,
    WithSnapshot,
};
#[doc(inline)]
pub use crate::stream_query::{query, StreamFilter, StreamQuery};
#[doc(inline)]
pub use crate::testing::TestHarness;

pub type BoxDynError = Box<dyn std::error::Error + 'static + Send + Sync>;

#[cfg(feature = "macros")]
pub use disintegrate_macros::{Event, StateQuery};

#[cfg(feature = "serde")]
pub mod serde {
    //! # Event Store Serialization Deserializaion.
    #[cfg(feature = "serde-avro")]
    #[doc(inline)]
    pub use disintegrate_serde::serde::avro;
    #[cfg(feature = "serde-json")]
    #[doc(inline)]
    pub use disintegrate_serde::serde::json;
    #[cfg(feature = "serde-messagepack")]
    #[doc(inline)]
    pub use disintegrate_serde::serde::messagepack;
    #[cfg(feature = "serde-prost")]
    #[doc(inline)]
    pub use disintegrate_serde::serde::prost;
    #[cfg(feature = "serde-protobuf")]
    #[doc(inline)]
    pub use disintegrate_serde::serde::protobuf;
    #[doc(inline)]
    pub use disintegrate_serde::{Deserializer, Serde, Serializer};
}

#[doc(hidden)]
#[macro_export]
macro_rules! all_the_tuples {
    ($name:ident) => {
        $name!([], T1);
        $name!([T1], T2);
        $name!([T1, T2], T3);
        $name!([T1, T2, T3], T4);
        $name!([T1, T2, T3, T4], T5);
    };
}
