//! Domain identifier represents an ID associated to a domain entity
//!
//! This module provides types and utilities for managing and manipulating domain identifiers.
//!
//! # Examples
//!
//! Creating a `DomainIdentifierSet` with two domain identifiers:
//!
//! ```
//! use disintegrate::{DomainIdentifier, DomainIdentifierSet, Identifier, domain_identifiers, IntoIdentifierValue};
//!
//! // Create domain identifiers
//! let identifier1 = Identifier::new("id1").unwrap();
//! let identifier2 = Identifier::new("id2").unwrap();
//!
//! // Create a DomainIdentifierSet
//! let mut identifier_set = domain_identifiers! {
//!     id1: "value1", id2: "value2"
//! };
//!
//! // Insert a new domain identifier
//! let new_identifier = DomainIdentifier {
//!     key: Identifier::new("id3").unwrap(),
//!     value: "value3".into_identifier_value(),
//! };
//! identifier_set.insert(new_identifier);
//!
//! // Access domain identifiers
//! assert_eq!(identifier_set.len(), 3);
//! assert_eq!(identifier_set.get(&identifier1), Some("value1".into_identifier_value()).as_ref());
//! assert_eq!(identifier_set.get(&identifier2), Some("value2".into_identifier_value()).as_ref());
//!
//! // Iterate over domain identifiers
//! for (key, value) in &*identifier_set {
//!     println!("Identifier: {}, Value: {}", key, value);
//! }
//! ```
use crate::{Identifier, IdentifierValue};
use std::{collections::BTreeMap, ops::Deref};

/// Represents a key-value pair of domain identifiers.
///
/// The `DomainIdentifier` struct is used to associate a specific `Identifier` key with a corresponding value.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DomainIdentifier {
    pub key: Identifier,
    pub value: IdentifierValue,
}

/// A set of domain identifiers, represented as a map of `Identifier` keys and values.
///
/// The `DomainIdentifierSet` struct is used to store a collection of domain identifiers.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct DomainIdentifierSet(BTreeMap<Identifier, IdentifierValue>);

impl DomainIdentifierSet {
    /// Creates a new `DomainIdentifierSet` with the given `BTreeMap` of domain identifiers.
    pub fn new(domain_identifiers: BTreeMap<Identifier, IdentifierValue>) -> Self {
        Self(domain_identifiers)
    }

    /// Inserts a new `DomainIdentifier` into the set.
    pub fn insert(&mut self, DomainIdentifier { key, value }: DomainIdentifier) {
        self.0.insert(key, value);
    }
}

/// Implements the `Deref` trait for `DomainIdentifierSet`, allowing it to be dereferenced to a `HashMap<Identifier, IdentifierValue>`.
/// This enables transparent access to the underlying `BTreeMap` of domain identifiers.
impl Deref for DomainIdentifierSet {
    type Target = BTreeMap<Identifier, IdentifierValue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Creates a domain identifiers set.
#[macro_export]
macro_rules! domain_identifiers{
    {}=> {
        $crate::DomainIdentifierSet::default()
    };
    {$($key:ident: $value:expr),*} => {{
        #[allow(unused_mut)]
        let mut domain_identifiers = std::collections::BTreeMap::<$crate::Identifier, $crate::IdentifierValue>::new();
        $(domain_identifiers.insert($crate::ident!(#$key), $crate::IntoIdentifierValue::into_identifier_value($value.clone()));)*
        $crate::DomainIdentifierSet::new(domain_identifiers)
    }};
}
