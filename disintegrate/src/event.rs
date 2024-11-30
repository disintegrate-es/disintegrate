//! Event represents an occurrence or action of interest within the system.
//!
//! This module defines the Event trait, which provides methods for retrieving domain identifiers associated
//! with the event and getting the event's name.
//!
//! The PersistedEvent struct wraps an event and contains an ID assigned by the event store. It represents
//! an event that has been persisted in the event store.
use crate::{domain_identifier::DomainIdentifierSet, Identifier, IdentifierType};
use std::ops::Deref;

/// Represents the ID of an event.
pub trait EventId:
    Default + Copy + Clone + PartialEq + Eq + Ord + PartialOrd + Send + Sync + 'static
{
}

impl<Id> EventId for Id where
    Id: Default + Copy + Clone + PartialEq + Eq + Ord + PartialOrd + Send + Sync + 'static
{
}

/// Represents the schema of all supported events.
///
/// The event info contains the name of the event and the domain identifiers associated with it.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EventInfo {
    /// The name of the event.
    pub name: &'static str,
    /// The domain identifiers associated with the event.
    pub domain_identifiers: &'static [&'static Identifier],
}

impl EventInfo {
    /// Returns true if the event has the given domain identifier.
    pub fn has_domain_identifier(&self, ident: &Identifier) -> bool {
        self.domain_identifiers.iter().any(|id| *id == ident)
    }
}

/// Represents the domain identifier and its type.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct DomainIdentifierInfo {
    /// The domain identifier.
    pub ident: Identifier,
    /// The type of the domain identifier.
    pub type_info: IdentifierType,
}

/// Represents the schema of all supported events.
///
/// The schema contains the names of all supported events,
/// the domain identifiers associated with them, and the domain identifiers' types.
#[derive(Debug, Clone)]
pub struct EventSchema {
    pub events: &'static [&'static str],
    pub events_info: &'static [&'static EventInfo],
    pub domain_identifiers: &'static [&'static DomainIdentifierInfo],
}

impl EventSchema {
    pub fn event_info(&self, name: &str) -> Option<&EventInfo> {
        self.events_info
            .iter()
            .find(|info| info.name == name)
            .copied()
    }
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
pub struct PersistedEvent<ID: EventId, E: Event> {
    pub(crate) id: ID,
    pub(crate) event: E,
}

impl<ID: EventId, E: Event> PersistedEvent<ID, E> {
    /// Creates a new `PersistedEvent` instance with the given ID and event.
    pub fn new(id: ID, event: E) -> Self {
        Self { id, event }
    }

    /// Returns the inner event.
    pub fn into_inner(self) -> E {
        self.event
    }

    /// Retrieves the ID assigned by the event store for this persisted event.
    pub fn id(&self) -> ID {
        self.id
    }
}

impl<ID: EventId, E: Event> Deref for PersistedEvent<ID, E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}
