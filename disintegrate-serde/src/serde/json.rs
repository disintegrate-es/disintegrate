use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::serde::{Deserializer, Serializer};

/// A JSON serialization and deserialization module.
#[derive(Debug, Clone, Copy)]
pub struct Json<T>(PhantomData<T>);

impl<T> Default for Json<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T> Serializer<T> for Json<T>
where
    T: Serialize,
{
    /// Serializes the given value to JSON format and returns the serialized bytes.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to be serialized.
    ///
    /// # Returns
    ///
    /// Serialized bytes representing the value in JSON format.
    fn serialize(&self, value: T) -> Vec<u8> {
        serde_json::to_vec(&value).expect("json serialization should not fail")
    }
}

impl<T> Deserializer<T> for Json<T>
where
    for<'d> T: Deserialize<'d>,
{
    type Error = serde_json::Error;
    /// Deserializes the given JSON bytes to produce a value of type `T`.
    ///
    /// # Arguments
    ///
    /// * `data` - The JSON bytes to be deserialized.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized value on success, or an error on failure.
    fn deserialize(&self, data: Vec<u8>) -> Result<T, Self::Error> {
        serde_json::from_slice(&data)
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
    fn it_serialize_and_deserialize_json_data() {
        let json_serializer = Json::<Person>::default();
        let person = Person {
            name: String::from("Some Name"),
            age: 30,
        };

        let serialized_data = json_serializer.serialize(person.clone());
        let deserialized_person = json_serializer.deserialize(serialized_data).unwrap();

        assert_eq!(person, deserialized_person);
    }
}
