//! Event store is responsible for storing and retrieving events.
//!
//! It is designed to be implemented by different storage backends, such as databases
//! or distributed event sourcing systems. Implementations of this trait should handle event persistence, querying,
//! and conflict resolution in a way that aligns with the specific requirements and constraints of the underlying
//! storage system.
//!
//! For more details and specific implementations, refer to the trait documentation and individual implementations
//! of the `EventStore` trait.
use crate::{
    event::{Event, PersistedEvent},
    stream_query::StreamQuery,
};

use async_trait::async_trait;
use futures::stream::BoxStream;
use std::error::Error as StdError;

/// A trait representing an event store.
///
/// This trait provides methods for streaming events and appending events to the event store.
#[async_trait]
pub trait EventStore<E>
where
    E: Event + Send + Sync,
{
    type Error: Send + Sync;

    // Streams events based on the provided query.
    ///
    /// # Arguments
    ///
    /// * `query` - The stream query specifying the filtering conditions.
    ///
    /// # Returns
    ///
    /// A `Result` containing a boxed stream of `PersistedEvent` matching the query, or an error.
    fn stream<'a, QE>(
        &'a self,
        query: &'a StreamQuery<QE>,
    ) -> BoxStream<Result<PersistedEvent<QE>, Self::Error>>
    where
        QE: TryFrom<E> + Event + 'static + Clone + Send + Sync,
        <QE as TryFrom<E>>::Error: StdError + 'static + Send + Sync;

    /// Appends a batch of events to the event store.
    ///
    /// # Arguments
    ///
    /// * `events` - A vector of events to append to the event store.
    /// * `query` - The stream query associated with the appended events.
    /// * `last_event_id` - The ID of the last event in the event stream that was queried before appending.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `PersistedEvent` representing the appended events, or an error.
    ///
    /// # Notes
    ///
    /// The `append` method re-executes the `query` and checks if there are new events between the `last_event_id`
    /// queried and the appended events' IDs. If new events are found, a conflict has occurred, and the conflict
    /// handling mechanism should be implemented accordingly.
    async fn append<QE>(
        &self,
        events: Vec<E>,
        query: StreamQuery<QE>,
        last_event_id: i64,
    ) -> Result<Vec<PersistedEvent<E>>, Self::Error>
    where
        E: Clone + 'async_trait,
        QE: Event + 'static + Clone + Send + Sync;
}
