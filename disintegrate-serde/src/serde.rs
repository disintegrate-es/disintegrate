use std::fmt::Debug;

#[cfg(feature = "avro")]
pub mod avro;
#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "prost")]
pub mod prost;
#[cfg(feature = "protobuf")]
pub mod protobuf;

/// The `Serializer` trait defines the behavior for serializing values of type `T`.
pub trait Serializer<T> {
    /// Serializes a value of type `T` into a byte vector.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to be serialized.
    ///
    /// # Returns
    ///
    /// A byte vector containing the serialized representation of the value.
    fn serialize(&self, value: T) -> Vec<u8>;
}

/// The `Deserializer` trait defines the behavior for deserializing values of type `T`.
pub trait Deserializer<T> {
    /// The error type that can occur during deserialization.
    type Error: Debug;

    /// Deserializes a byte vector into a value of type `T`.
    ///
    /// # Arguments
    ///
    /// * `data` - The byte vector to be deserialized.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized value on success, or an error on failure.
    fn deserialize(&self, data: Vec<u8>) -> Result<T, Self::Error>;
}

impl<T, Err, F> Deserializer<T> for F
where
    F: Fn(Vec<u8>) -> Result<T, Err>,
    Err: Debug,
{
    type Error = Err;

    fn deserialize(&self, data: Vec<u8>) -> Result<T, Self::Error> {
        self(data)
    }
}

/// The `Serde` trait combines the `Serializer` and `Deserializer` traits for convenience.
pub trait Serde<T>: Serializer<T> + Deserializer<T> {}

impl<K, T> Serde<T> for K where K: Serializer<T> + Deserializer<T> {}
