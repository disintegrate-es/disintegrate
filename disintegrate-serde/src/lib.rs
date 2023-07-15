//! # Event Store Serialization Deserializaion Library
//!
//! This library provides traits and implementations for serializing and deserializing events for the Disintegrate Event Store.
//! It includes implementations for common formats such as Avro, JSON, Protocol Buffers (Prost).
pub mod serde;
pub use crate::serde::{Deserializer, Error, Serde, Serializer};
