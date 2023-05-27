//! Identifiers are string values that adhere to certain rules.
//!
//! This module provides the `Identifier` struct, which represents a valid identifier.
//! It also includes functions for creating, validating, and converting
//! identifiers, as well as an error type for identifier-related errors.
//!
//! # Examples
//!
//! Creating a new identifier:
//!
//! ```
//! use disintegrate::Identifier;
//!
//! let identifier = Identifier::new("my_identifier").unwrap();
//! println!("Identifier: {}", identifier);
//! ```
//!
//! Using the `ident!` macro to create identifiers in a safe manner:
//!
//! ```
//! use disintegrate::ident;
//!
//! let identifier = ident!(#my_identifier);
//! println!("Identifier: {}", identifier);
//! ```
//!
//! Parsing an identifier from a string:
//!
//! ```
//! use disintegrate::Identifier;
//! use std::str::FromStr;
//!
//! let identifier = Identifier::from_str("my_identifier").unwrap();
//! println!("Identifier: {}", identifier);
//! ```
//!
//! Handling identifier validation errors:
//!
//! ```
//! use disintegrate::Identifier;
//!
//! let identifier = Identifier::new("invalid identifier");
//! match identifier {
//!     Ok(id) => println!("Identifier: {}", id),
//!     Err(err) => println!("Error: {}", err),
//! }
//! ```
//!
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;

/// Represents a valid identifier.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Identifier(String);

impl Identifier {
    /// Creates a new identifier from a string.
    ///
    /// # Arguments
    ///
    /// * `s` - The string value representing the identifier.
    ///
    /// # Errors
    ///
    /// Returns an `IdentifierError` if the string value is not a valid identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use disintegrate::Identifier;
    ///
    /// let identifier = Identifier::new("my_identifier").unwrap();
    /// println!("Identifier: {}", identifier);
    /// ```
    pub fn new<S: Into<String>>(s: S) -> Result<Self, IdentifierError> {
        let s = s.into();
        if !Self::is_valid_identifier(&s) {
            return Err(IdentifierError::new(&format!("Invalid identifier: {s}")));
        }
        Ok(Self(s))
    }

    /// # Safety
    /// This method is marked as unsafe because it is intended to be used only in the `ident!` macro,
    /// where it is guaranteed that the `s` value passed to it is a valid identifier. The `ident!` macro
    /// is responsible for ensuring that the string passed to this method is a valid identifier, and
    /// the checks are intentionally skipped here to avoid redundant checks.
    ///
    /// It is important to note that if this method is called outside of the `ident!` macro, it may
    /// result in potentially invalid identifiers, as any object that implements `Into<String>` can be
    /// passed as `s`, and the result of the `into` method may be an invalid identifier.
    ///
    /// Note: If possible, it is generally preferred to use safe methods instead of unsafe ones,
    /// and to avoid using `unsafe` unless absolutely necessary. If there is a way to achieve the
    /// same functionality in a safe manner, that should be preferred.
    pub unsafe fn unsafe_new<S: Into<String>>(s: S) -> Self {
        Self(s.into())
    }

    /// Determines whether a string value is a valid identifier.
    ///
    /// # Arguments
    ///
    /// * `s` - The string value to check.
    ///
    /// # Returns
    ///
    /// Returns `true` if the string value is a valid identifier, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use disintegrate::Identifier;
    ///
    /// assert_eq!(Identifier::is_valid_identifier("my_identifier"), true);
    /// assert_eq!(Identifier::is_valid_identifier("123"), false);
    /// ```
    pub fn is_valid_identifier(s: &str) -> bool {
        lazy_static! {
            static ref RE: Regex = Regex::new("^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();
        }
        RE.is_match(s)
    }
}

/// Creates an identifier in a safe manner.
///
/// It takes an identifier name as an argument and constructs an `Identifier` instance
/// without performing runtime validation. This macro is used internally and ensures
/// that the identifier passed to it is valid.
///
/// # Example
///
/// ```
/// use disintegrate::ident;
///
/// let identifier = ident!(#my_identifier);
/// ```
///
/// # Safety
/// The `ident` macro is marked as unsafe because it bypasses the runtime validation of identifiers.
/// It should only be used in controlled scenarios where it is guaranteed that the identifier passed
/// to it is a valid identifier.
///
#[macro_export]
macro_rules! ident {
    (#$id:ident) => {{
        let ident;
        unsafe { ident = $crate::identifier::Identifier::unsafe_new(stringify!($id)) }
        ident
    }};
}

/// The `FromStr` trait implementation allows parsing of an `Identifier` from a string.
/// It enables converting a string representation of an identifier into an `Identifier` instance.
/// Parsing can fail if the string does not represent a valid identifier. The associated error type is `IdentifierError`.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use disintegrate::Identifier;
///
/// let identifier = Identifier::from_str("my_identifier").unwrap();
///
// ```
/// let identifier:Identifier = "my_identifier".parse().unwrap();
/// ```
///
impl FromStr for Identifier {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

/// The `Display` trait implementation enables formatting an `Identifier` for display purposes.
/// It allows converting an `Identifier` instance to a string representation.
///
/// # Example
///
/// ```
/// use std::fmt::Display;
/// use disintegrate::Identifier;
///
/// let identifier = Identifier::new("my_identifier").unwrap();
/// println!("Identifier: {}", identifier);
/// ```
///
impl Display for Identifier {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The `IdentifierError` struct represents an error that can occur when working with identifiers.
#[derive(Debug)]
pub struct IdentifierError(String);

impl IdentifierError {
    /// Creates a new `IdentifierError` instance with the provided error message.
    fn new(s: &str) -> Self {
        IdentifierError(s.to_string())
    }
}

/// The `Display` trait implementation enables formatting an `IdentifierError` for display purposes.
impl Display for IdentifierError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Implements the `Deref` trait for `Identifier`, allowing it to be dereferenced to a `String`.
/// This enables transparent access to the underlying `String` value of the identifier.
///
/// # Examples
///
/// ```
/// use disintegrate::Identifier;
///
/// let identifier = Identifier::new("my_identifier").unwrap();
/// assert_eq!(*identifier, "my_identifier");
/// ```
///
/// Dereferencing an `Identifier` instance returns a reference to the underlying `String` value,
/// allowing it to be used as if it were a `String` directly.
/// This can be useful when working with functions or methods that expect a `String` as input.
///
impl Deref for Identifier {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Error for IdentifierError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_create_valid_identifier() {
        let identifier = Identifier::new("hello_world").unwrap();
        assert_eq!(*identifier, "hello_world".to_string());
    }

    #[test]
    fn it_cannot_create_empty_identifier() {
        let err = Identifier::new("").unwrap_err();
        assert_eq!(err.to_string(), "Invalid identifier: ");
    }

    #[test]
    fn it_cannot_create_identifier_with_spaces() {
        let err = Identifier::new("hello world").unwrap_err();
        assert_eq!(err.to_string(), "Invalid identifier: hello world");
    }

    #[test]
    fn it_cannot_create_non_ascii_identifier() {
        let err = Identifier::new("héllo").unwrap_err();
        assert_eq!(err.to_string(), "Invalid identifier: héllo");
    }

    #[test]
    fn it_can_parse_identifier_from_string() {
        let identifier: Identifier = "hello_world".parse().unwrap();
        assert_eq!(*identifier, "hello_world".to_string());
    }

    #[test]
    fn it_cannot_parse_empty_identifier() {
        let err = "".parse::<Identifier>().unwrap_err();
        assert_eq!(err.to_string(), "Invalid identifier: ");
    }

    #[test]
    fn it_cannot_parse_identifier_with_spaces() {
        let err = "hello world".parse::<Identifier>().unwrap_err();
        assert_eq!(err.to_string(), "Invalid identifier: hello world");
    }

    #[test]
    fn it_cannot_parse_non_ascii_identifier() {
        let err = "héllo".parse::<Identifier>().unwrap_err();
        assert_eq!(err.to_string(), "Invalid identifier: héllo");
    }
}
