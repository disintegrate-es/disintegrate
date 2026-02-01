//! Domain identifier represents an ID associated to a domain entity
//!
//! This module provides types and utilities for managing and manipulating domain identifiers.
//!
//! # Examples
//!
//! Creating a `DomainIdSet` with two domain identifiers:
//!
//! ```
//! use disintegrate::{DomainId, DomainIdSet, Identifier, domain_ids, IntoIdentifierValue};
//!
//! // Create domain ids
//! let identifier1 = Identifier::new("id1").unwrap();
//! let identifier2 = Identifier::new("id2").unwrap();
//!
//! // Create a DomainIdSet
//! let mut domain_ids = domain_ids! {
//!     id1: "value1", id2: "value2"
//! };
//!
//! // Insert a new domain id
//! let new_domain_id = DomainId {
//!     key: Identifier::new("id3").unwrap(),
//!     value: "value3".into_identifier_value(),
//! };
//! domain_ids.insert(new_domain_id);
//!
//! // Access domain identifiers
//! assert_eq!(domain_ids.len(), 3);
//! assert_eq!(domain_ids.get(&identifier1), Some("value1".into_identifier_value()).as_ref());
//! assert_eq!(domain_ids.get(&identifier2), Some("value2".into_identifier_value()).as_ref());
//!
//! // Iterate over domain identifiers
//! for (key, value) in &*domain_ids {
//!     println!("Identifier: {}, Value: {}", key, value);
//! }
//! ```
use serde::Serialize;

use crate::{Identifier, IdentifierValue};
use std::{collections::BTreeMap, ops::Deref};

/// Represents a key-value pair of domain identifiers.
///
/// The `DomainId` struct is used to associate a specific `Identifier` key with a corresponding value.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DomainId {
    pub key: Identifier,
    pub value: IdentifierValue,
}

/// A set of domain identifiers, represented as a map of `Identifier` keys and values.
///
/// The `DomainIdSet` struct is used to store a collection of domain identifiers.
#[derive(Debug, Serialize, Default, PartialEq, Eq, Clone)]
pub struct DomainIdSet(BTreeMap<Identifier, IdentifierValue>);

impl DomainIdSet {
    /// Creates a new `DomainIdSet` with the given `BTreeMap` of domain identifiers.
    pub fn new(domain_ids: BTreeMap<Identifier, IdentifierValue>) -> Self {
        Self(domain_ids)
    }

    /// Inserts a new `DomainId` into the set.
    pub fn insert(&mut self, DomainId { key, value }: DomainId) {
        self.0.insert(key, value);
    }
}

/// Implements the `Deref` trait for `DomainIdSet`, allowing it to be dereferenced to a `HashMap<Identifier, IdentifierValue>`.
/// This enables transparent access to the underlying `BTreeMap` of domain identifiers.
impl Deref for DomainIdSet {
    type Target = BTreeMap<Identifier, IdentifierValue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Creates a domain identifiers set.
#[macro_export]
macro_rules! domain_ids{
    {}=> {
        $crate::DomainIdSet::default()
    };
    {$($key:ident: $value:expr),*} => {{
        #[allow(unused_mut)]
        let mut domain_ids = std::collections::BTreeMap::<$crate::Identifier, $crate::IdentifierValue>::new();
        $(domain_ids.insert($crate::ident!(#$key), $crate::IntoIdentifierValue::into_identifier_value($value.clone()));)*
        $crate::DomainIdSet::new(domain_ids)
    }};
}
