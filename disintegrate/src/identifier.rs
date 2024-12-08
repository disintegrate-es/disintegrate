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
use uuid::Uuid;

/// Represents a valid identifier.
#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Identifier(pub(crate) &'static str);

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
    pub fn new(s: &'static str) -> Result<Self, IdentifierError> {
        if !Self::is_valid_identifier(s) {
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
    pub const unsafe fn unsafe_new(s: &'static str) -> Self {
        Self(s)
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
    /// The inner string value of the identifier.
    ///
    /// # Returns
    ///
    /// A reference to the inner string value of the identifier.
    pub const fn into_inner(&self) -> &str {
        self.0
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
        unsafe { ident = $crate::Identifier::unsafe_new(stringify!($id)) }
        ident
    }};
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

impl TryFrom<&'static str> for Identifier {
    type Error = IdentifierError;

    fn try_from(value: &'static str) -> Result<Self, Self::Error> {
        Self::new(value)
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
    type Target = &'static str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Error for IdentifierError {}

macro_rules! impl_identifier_type {
    ($($type:ident),+) =>{
        #[derive(Debug, Eq, PartialEq, Copy, Clone)]
        #[allow(non_camel_case_types)]
        /// Represents the type of an identifier value.
        pub enum IdentifierType{
           $($type,)+
        }

        #[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
        #[allow(non_camel_case_types)]
        /// Represents the value of an identifier, allowing different types.
        pub enum IdentifierValue{
           $($type($type),)+
        }

        impl std::fmt::Display for IdentifierValue{
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match self{
                    $(Self::$type(value) => write!(f, "{}", value),)+
                }
            }
        }

        $(impl IntoIdentifierValue for $type{
            const TYPE: IdentifierType = IdentifierType::$type;
            fn into_identifier_value(self) -> IdentifierValue {
                IdentifierValue::$type(self)
            }
        })+

        $(impl IntoIdentifierValue for &$type{
            const TYPE: IdentifierType = IdentifierType::$type;
            fn into_identifier_value(self) -> IdentifierValue {
                IdentifierValue::$type(self.clone())
            }
        })+

        impl IntoIdentifierValue for &str{
            const TYPE: IdentifierType = IdentifierType::String;
            fn into_identifier_value(self) -> IdentifierValue {
                IdentifierValue::String(self.to_string())
            }
        }
    };
}

impl_identifier_type! {String, i64, Uuid}

/// Represents a value that can be used as an identifier value.
///
/// The `IntoIdentifierValue` trait allows converting values into `IdentifierValue` instances,
/// specifying the type of the value.
///
/// Implementations of this trait should provide the associated constant `TYPE` with the specific
/// `IdentifierType` variant corresponding to the type being converted.
pub trait IntoIdentifierValue {
    /// the type of the value
    const TYPE: IdentifierType;
    /// Converts the value into the corresponding `IdentifierValue` variant.
    ///
    /// # Returns
    ///
    /// An `IdentifierValue` variant containing the converted value.
    fn into_identifier_value(self) -> IdentifierValue;
}

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
        let identifier: Identifier = "hello_world".try_into().unwrap();
        assert_eq!(*identifier, "hello_world".to_string());
    }

    #[test]
    fn it_cannot_parse_empty_identifier() {
        let err = TryInto::<Identifier>::try_into("").unwrap_err();
        assert_eq!(err.to_string(), "Invalid identifier: ");
    }

    #[test]
    fn it_cannot_parse_identifier_with_spaces() {
        let err = TryInto::<Identifier>::try_into("hello world").unwrap_err();
        assert_eq!(err.to_string(), "Invalid identifier: hello world");
    }

    #[test]
    fn it_cannot_parse_non_ascii_identifier() {
        let err = TryInto::<Identifier>::try_into("héllo").unwrap_err();
        assert_eq!(err.to_string(), "Invalid identifier: héllo");
    }

    #[test]
    fn it_converts_string_into_identifier_value() {
        let string_value: &'static str = "my_string";
        let identifier_value = string_value.into_identifier_value();
        assert_eq!(
            identifier_value,
            IdentifierValue::String("my_string".to_string())
        );
    }

    #[test]
    fn it_converts_integer_into_identifier_value() {
        let number: i64 = 42;
        let identifier_value = number.into_identifier_value();
        assert_eq!(identifier_value, IdentifierValue::i64(42));
    }

    #[test]
    fn it_converts_uuid_into_identifier_value() {
        let uuid_value = uuid::Uuid::new_v4();
        let identifier_value = uuid_value.into_identifier_value();
        assert_eq!(identifier_value, IdentifierValue::Uuid(uuid_value));
    }
}
