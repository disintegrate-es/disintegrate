//! Domain identifier represents an ID associated to a domain entity
//!
//! This module provides types and utilities for managing and manipulating domain identifiers.
//!
//! # Examples
//!
//! Creating a `DomainIdentifierSet` with two domain identifiers:
//!
//! ```
//! use disintegrate::{DomainIdentifier, DomainIdentifierSet, Identifier, domain_identifiers};
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
//!     value: "value3".to_string(),
//! };
//! identifier_set.insert(new_identifier);
//!
//! // Access domain identifiers
//! assert_eq!(identifier_set.len(), 3);
//! assert_eq!(identifier_set.get(&identifier1), Some(&"value1".to_string()));
//! assert_eq!(identifier_set.get(&identifier2), Some(&"value2".to_string()));
//!
//! // Iterate over domain identifiers
//! for (key, value) in &*identifier_set {
//!     println!("Identifier: {}, Value: {}", key, value);
//! }
//! ```
use std::{collections::HashMap, ops::Deref};

use crate::identifier::Identifier;
use serde::{Deserialize, Serialize};

/// Represents a key-value pair of domain identifiers.
///
/// The `DomainIdentifier` struct is used to associate a specific `Identifier` key with a corresponding `String` value.
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct DomainIdentifier {
    pub key: Identifier,
    pub value: String,
}

/// A set of domain identifiers, represented as a hashmap of `Identifier` keys and `String` values.
///
/// The `DomainIdentifierSet` struct is used to store a collection of domain identifiers.
#[derive(Debug, Default, PartialEq, Clone, Deserialize, Serialize)]
pub struct DomainIdentifierSet(HashMap<Identifier, String>);

impl DomainIdentifierSet {
    /// Creates a new `DomainIdentifierSet` with the given `HashMap` of domain identifiers.
    pub fn new(domain_identifiers: HashMap<Identifier, String>) -> Self {
        Self(domain_identifiers)
    }

    /// Inserts a new `DomainIdentifier` into the set.
    pub fn insert(&mut self, DomainIdentifier { key, value }: DomainIdentifier) {
        self.0.insert(key, value);
    }
}

/// Implements the `Deref` trait for `DomainIdentifierSet`, allowing it to be dereferenced to a `HashMap<Identifier, String>`.
/// This enables transparent access to the underlying `HashMap` of domain identifiers.
impl Deref for DomainIdentifierSet {
    type Target = HashMap<Identifier, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Creates a domain identifiers set.
#[macro_export]
macro_rules! domain_identifiers{
    {}=> {
        $crate::domain_identifier::DomainIdentifierSet::default()
    };
    {$($key:ident: $value:expr),*} => {{

        const DOMAIN_IDENTIFIERS_COUNT: usize = $crate::count![@COUNT; $($key),*];

        #[allow(unused_mut)]
        let mut domain_identifiers = std::collections::HashMap::<$crate::identifier::Identifier, String>::with_capacity(DOMAIN_IDENTIFIERS_COUNT);
        $(domain_identifiers.insert($crate::ident!(#$key), $value.to_string());)*
        $crate::domain_identifier::DomainIdentifierSet::new(domain_identifiers)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! count {
    (@COUNT; $($element:expr),*) => {
        <[()]>::len(&[$($crate::count![@SUBST; $element]),*])
    };
    (@SUBST; $_element:expr) => { () };
}
