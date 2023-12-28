#[cfg(feature = "avro")]
pub mod avro;
#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "prost")]
pub mod prost;
#[cfg(feature = "protobuf")]
pub mod protobuf;

/// Serialization and deserialization error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// an error occurred during the deserialization of the data
    #[error("deserialization error: {0}")]
    Deserialization(#[source] Box<dyn std::error::Error + Sync + Send>),
    /// an error occurred while converting the persisted data to the application data
    #[error("conversion error")]
    Conversion,
}

/// Defines the behavior for serializing values of type `T`.
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

/// Defines the behavior for deserializing values of type `T`.
pub trait Deserializer<T> {
    /// Deserializes a byte vector into a value of type `T`.
    ///
    /// # Arguments
    ///
    /// * `data` - The byte vector to be deserialized.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized value on success, or an error on failure.
    fn deserialize(&self, data: Vec<u8>) -> Result<T, Error>;
}

/// Combines the `Serializer` and `Deserializer` traits for convenience.
pub trait Serde<T>: Serializer<T> + Deserializer<T> {}

impl<K, T> Serde<T> for K where K: Serializer<T> + Deserializer<T> {}
