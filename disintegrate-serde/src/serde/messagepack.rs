use rmp_serde;
use std::marker::PhantomData;

use serde::Deserialize;
use serde::Serialize;

use super::Error;
use crate::serde::Deserializer;
use crate::serde::Serializer;

/// A struct to serialize and deserialize MessagePack payloads.
#[derive(Debug, Clone, Copy)]
pub struct MessagePack<T>(PhantomData<T>);

impl<T> Default for crate::serde::messagepack::MessagePack<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
impl<T> Serializer<T> for crate::serde::messagepack::MessagePack<T>
where
    T: Serialize,
{
    /// Serializes the given value to MessagePack format and returns the serialized bytes.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to be serialized.
    ///
    /// # Returns
    ///
    /// Serialized bytes representing the value in MessagePack format.
    fn serialize(&self, value: T) -> Vec<u8> {
        rmp_serde::to_vec(&value).expect("MessagePack serialization failed")
    }
}

impl<T> Deserializer<T> for crate::serde::messagepack::MessagePack<T>
where
    for<'d> T: Deserialize<'d>,
{
    /// Deserializes the given MessagePack bytes to produce a value of type `T`.
    ///
    /// # Arguments
    ///
    /// * `data` - The MessagePack bytes to be deserialized.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized value on success, or an error on failure.
    fn deserialize(&self, data: Vec<u8>) -> Result<T, Error> {
        rmp_serde::from_slice(&data).map_err(|e| Error::Deserialization(Box::new(e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    struct Person {
        name: String,
        age: u32,
    }

    #[test]
    fn it_serialize_and_deserialize_messagepack_data() {
        let msgpack_serializer = crate::serde::messagepack::MessagePack::<Person>::default();
        let person = Person {
            name: String::from("Some Name"),
            age: 30,
        };

        let serialized_data = msgpack_serializer.serialize(person.clone());
        let deserialized_person = msgpack_serializer.deserialize(serialized_data).unwrap();

        assert_eq!(person, deserialized_person);
    }
}
