use std::marker::PhantomData;

use prost::{bytes::Bytes, Message};

use crate::serde::{Deserializer, Serializer};
use thiserror::Error;

/// A serialization and deserialization module using Prost.
///
/// This module provides the capability to serialize and deserialize data using the Prost library.
#[derive(Debug, Clone, Copy)]
pub struct Prost<I, O>(PhantomData<I>, PhantomData<O>)
where
    O: Message;

impl<I, O> Prost<I, O>
where
    O: Message,
{
    /// Creates a new instance of the `ProstSerde` module.
    pub fn new() -> Self {
        Self(PhantomData, PhantomData)
    }
}

impl<I, O> Default for Prost<I, O>
where
    O: Message,
{
    fn default() -> Self {
        Prost::new()
    }
}

impl<I, O> Serializer<I> for Prost<I, O>
where
    O: From<I> + Message,
{
    /// Serializes the given value to Protobuf-encoded bytes.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to be serialized.
    ///
    /// # Returns
    ///
    /// Serialized bytes representing the value in Protobuf format.
    fn serialize(&self, value: I) -> Vec<u8> {
        let target = O::from(value);
        target.encode_to_vec()
    }
}

/// An error that occurs during decoding of encoded data.
#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("deserialization error")]
    Deserialize(#[from] prost::DecodeError),
    #[error("failed to convert data to the target type")]
    Conversion,
}

impl<I, O> Deserializer<I> for Prost<I, O>
where
    I: TryFrom<O>,
    O: Message + Default,
{
    type Error = DecodeError;

    /// Deserializes the given Protobuf-encoded bytes to produce a value of type `I`.
    ///
    /// # Arguments
    ///
    /// * `data` - The Protobuf-encoded bytes to be deserialized.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized value on success, or an error on failure.
    fn deserialize(&self, data: Vec<u8>) -> Result<I, Self::Error> {
        let buf = Bytes::from(data);

        let target = O::decode(buf)?;
        I::try_from(target).map_err(|_| DecodeError::Conversion)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;

    #[derive(PartialEq, Message, Clone)]
    struct Person {
        #[prost(string, tag = "1")]
        name: String,
        #[prost(uint32, tag = "2")]
        age: u32,
    }

    #[test]
    fn it_serialize_and_deserialize_prost_data() {
        let serde_module = Prost::<Person, Person>::new();

        let person = Person {
            name: String::from("Some name"),
            age: 30,
        };

        // Serialize the person to bytes
        let serialized_data = serde_module.serialize(person.clone());

        // Deserialize the bytes back to a person
        let deserialized_person = serde_module.deserialize(serialized_data).unwrap();

        // Verify that the deserialized person matches the original person
        assert_eq!(person, deserialized_person);
    }
}
