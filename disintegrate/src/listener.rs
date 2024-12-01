//! Event listener handles events that are emitted.
use async_trait::async_trait;

use crate::{
    event::{Event, EventId, PersistedEvent},
    stream_query::StreamQuery,
};

/// Represents an event listener, which handles persisted events.
#[async_trait]
pub trait EventListener<ID: EventId, E: Event + Clone>: Send + Sync {
    /// The type of error that may occur during the handle of an event.
    type Error;

    /// Returns the unique identifier of the event listener.
    ///
    /// It is typically a string or identifier that helps identify and distinguish the event handler.
    fn id(&self) -> &'static str;

    /// Returns the stream query used by the event listener.
    ///
    /// The query specifies the criteria for the events that the event listener can handle.
    fn query(&self) -> &StreamQuery<ID, E>;

    /// Handles an event.
    ///
    /// This method handle the event coming from the event stream.
    /// The method returns a result indicating success or an error that may occur during the event handler.
    async fn handle(&self, event: PersistedEvent<ID, E>) -> Result<(), Self::Error>;
}
