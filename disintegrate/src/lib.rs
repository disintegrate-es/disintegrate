#![doc = include_str!("../README.md")]

pub mod decision;
pub mod domain_identifier;
mod event;
mod event_store;
pub mod identifier;
mod listener;
pub mod stream_query;
mod testing;
pub mod utils;

#[doc(inline)]
pub use crate::decision::{
    Decision, DecisionMaker, DecisionStateStore, EventSourcedDecisionStateStore, IntoState,
    IntoStatePart, MultiState, NoSnapshot, SnapshotConfig, StateMutate, StatePart, StateQuery,
    StateSnapshotter, WithSnapshot,
};
#[doc(inline)]
pub use crate::domain_identifier::{DomainIdentifier, DomainIdentifierSet};
#[doc(inline)]
pub use crate::event::{DomainIdentifierInfo, Event, EventInfo, EventSchema, PersistedEvent};
#[doc(inline)]
pub use crate::event_store::EventStore;
#[doc(inline)]
pub use crate::identifier::{Identifier, IdentifierType, IdentifierValue, IntoIdentifierValue};
#[doc(inline)]
pub use crate::listener::EventListener;
#[doc(inline)]
pub use crate::stream_query::{query, StreamQuery};
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
