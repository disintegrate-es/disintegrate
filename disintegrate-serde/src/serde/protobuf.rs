use std::marker::PhantomData;

use protobuf::Message;
use thiserror::Error;

use crate::serde::{Deserializer, Serializer};

/// A serialization and deserialization module using Protobuf.
#[derive(Debug, Clone, Copy)]
pub struct Protobuf<I, O>(PhantomData<I>, PhantomData<O>)
where
    O: Message;

impl<I, O> Default for Protobuf<I, O>
where
    O: Message,
{
    fn default() -> Self {
        Self(PhantomData, PhantomData)
    }
}

impl<I, O> Serializer<I> for Protobuf<I, O>
where
    O: From<I> + Message,
{
    /// Serializes the given value to a byte vector.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to be serialized.
    ///
    /// # Returns
    ///
    /// A byte vector containing the serialized data.
    ///
    /// # Panics
    ///
    /// Panics if the serialization from Rust type to Protobuf format fails.
    fn serialize(&self, value: I) -> Vec<u8> {
        let target = O::from(value);
        target
            .write_to_bytes()
            .expect("serialization from rust type to protobuf format should be successful")
    }
}

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("deserialization error")]
    Deserialize(#[from] protobuf::Error),
    #[error("failed to convert data to the target type")]
    Conversion,
}

impl<I, O> Deserializer<I> for Protobuf<I, O>
where
    I: TryFrom<O>,
    O: Message,
{
    type Error = DecodeError;

    /// Deserializes the given byte vector to a target type.
    ///
    /// # Arguments
    ///
    /// * `data` - The byte vector to be deserialized.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized value on success, or an error on failure.
    fn deserialize(&self, data: Vec<u8>) -> Result<I, Self::Error> {
        let target = O::parse_from_bytes(&data)?;
        I::try_from(target).map_err(|_| DecodeError::Conversion)
    }
}
