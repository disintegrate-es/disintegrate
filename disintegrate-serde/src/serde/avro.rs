//! A module for serializing and deserializing data using Avro schema.
use std::marker::PhantomData;

use super::Error;
use apache_avro::{from_value, Codec, Reader, Schema, Writer};
use serde::{Deserialize, Serialize};

use crate::serde::{Deserializer, Serializer};

/// An Avro serialization and deserialization module.
#[derive(Debug, Clone)]
pub struct Avro<I, O> {
    schema: Schema,
    input: PhantomData<I>,
    output: PhantomData<O>,
}

impl<I, O> Avro<I, O> {
    /// Create a new instance of `Avro` with the specified Avro schema.
    ///
    /// # Arguments
    ///
    /// * `schema` - A string representing the Avro schema.
    ///
    /// # Returns
    ///
    /// A new `Avro` instance
    pub fn new(schema: &str) -> Self {
        let schema = Schema::parse_str(schema).unwrap();
        Self {
            schema,
            input: PhantomData,
            output: PhantomData,
        }
    }
}

impl<I, O> Serializer<I> for Avro<I, O>
where
    O: From<I> + Serialize,
{
    /// Serialize the given value to Avro format and return the serialized bytes.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to be serialized.
    ///
    /// # Returns
    ///
    /// Serialized bytes representing the value in Avro format.
    fn serialize(&self, value: I) -> Vec<u8> {
        let target = O::from(value);
        let mut writer = Writer::with_codec(&self.schema, Vec::new(), Codec::Deflate);
        writer
            .append_ser(target)
            .expect("avro serialization should not fail");
        writer.into_inner().expect("encoded avro should not fail")
    }
}

impl<I, O> Deserializer<I> for Avro<I, O>
where
    I: TryFrom<O>,
    for<'d> O: Deserialize<'d>,
{
    /// Deserialize the given Avro serialized bytes to produce a value of type `I`.
    ///
    /// # Arguments
    ///
    /// * `data` - The Avro serialized bytes to be deserialized.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized value on success, or an error on failure.
    fn deserialize(&self, data: Vec<u8>) -> Result<I, Error> {
        let mut reader = Reader::new(&data[..]).map_err(|e| Error::Deserialization(Box::new(e)))?;
        let value = reader
            .next()
            .expect("at least one value should be present")
            .map_err(|e| Error::Deserialization(Box::new(e)))?;
        let target: O = from_value(&value).map_err(|e| Error::Deserialization(Box::new(e)))?;
        I::try_from(target).map_err(|_| Error::Conversion)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryFrom;

    #[derive(Debug, PartialEq, Clone)]
    struct InputData {
        value: u32,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    struct SerializedData {
        value: String,
    }

    const TEST_SCHEMA: &str = r#"
        {
            "type": "record",
            "name": "TestRecord",
            "fields": [
                { "name": "value", "type": "string" }
            ]
        }
    "#;

    #[derive(Debug, PartialEq)]
    enum ConversionError {
        InvalidValue,
    }

    impl TryFrom<SerializedData> for InputData {
        type Error = ConversionError;

        fn try_from(data: SerializedData) -> Result<Self, Self::Error> {
            let input_value = data
                .value
                .parse::<u32>()
                .map_err(|_| ConversionError::InvalidValue)?;
            Ok(InputData { value: input_value })
        }
    }

    impl From<InputData> for SerializedData {
        fn from(data: InputData) -> Self {
            SerializedData {
                value: data.value.to_string(),
            }
        }
    }

    #[test]
    fn it_serializes_and_deserializes_avro_data() {
        // Create an instance of the Avro module with the test schema
        let avro = Avro::<InputData, SerializedData>::new(TEST_SCHEMA);

        let input = InputData { value: 42 };

        // Serialize the input data
        let serialized = avro.serialize(input.clone());

        // Deserialize the serialized data
        let deserialized: InputData = avro.deserialize(serialized).unwrap();

        // Ensure the deserialized data matches the original input
        assert_eq!(deserialized, input);
    }
}
