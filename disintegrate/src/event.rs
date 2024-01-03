//! Event represents an occurrence or action of interest within the system.
//!
//! This module defines the Event trait, which provides methods for retrieving domain identifiers associated
//! with the event and getting the event's name.
//!
//! The PersistedEvent struct wraps an event and contains an ID assigned by the event store. It represents
//! an event that has been persisted in the event store.
use crate::{domain_identifier::DomainIdentifierSet, Identifier, IdentifierType};
use std::ops::Deref;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct DomainIdentifierInfo {
    pub ident: Identifier,
    pub type_info: IdentifierType,
}

#[derive(Debug, Clone)]
pub struct EventSchema {
    pub types: &'static [&'static str],
    pub domain_identifiers: &'static [&'static DomainIdentifierInfo],
}

/// Represents an event in the event store.
///
/// An event is an occurrence or action of interest within the system. It can be persisted and retrieved from
/// the event store. The `Event` trait provides methods for retrieving domain identifiers associated with the event
/// and getting the event's name. The constant `SCHEMA` holds the name and the domain identifiers of all supported events.
pub trait Event {
    /// Returns the schema of all supported events.
    const SCHEMA: EventSchema;
    /// Retrieves the domain identifiers associated with the event.
    fn domain_identifiers(&self) -> DomainIdentifierSet;
    /// Retrieves the name of the event.
    fn name(&self) -> &'static str;
}

/// Wrapper for a persisted event.
///
/// It contains an ID assigned by the event store and the event itself.
#[derive(Debug, Clone)]
pub struct PersistedEvent<E: Event> {
    pub(crate) id: i64,
    pub(crate) event: E,
}

impl<E: Event> PersistedEvent<E> {
    /// Creates a new `PersistedEvent` instance with the given ID and event.
    pub fn new(id: i64, event: E) -> Self {
        Self { id, event }
    }

    /// Returns the inner event.
    pub fn into_inner(self) -> E {
        self.event
    }

    /// Retrieves the ID assigned by the event store for this persisted event.
    pub fn id(&self) -> i64 {
        self.id
    }
}

impl<E: Event> Deref for PersistedEvent<E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}
