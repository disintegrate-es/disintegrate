use std::error::Error as StdError;
use thiserror::Error;

/// Represents all the ways a method can fail within Disintegrate Postgres.
#[derive(Error, Debug)]
pub enum Error {
    /// Error returned from the database.
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    /// An error occurred while deserializing an event payload.
    #[error(transparent)]
    Deserialization(#[from] disintegrate_serde::Error),
    /// An error occurred while acquiring an append permit.
    #[error(transparent)]
    AppendPermit(#[from] tokio::sync::AcquireError),
    /// An error occurred while mapping the event store event to the query event
    #[error("unable to map the event store event to the query event: {0}")]
    QueryEventMapping(#[source] Box<dyn StdError + 'static + Send + Sync>),
    // An error occurred while attempting to persist events using an outdated version of the event set.
    ///
    /// This error indicates that another process has inserted a new event that was not included in the event stream query
    /// used to make the current business decision. The event store's state has changed, potentially affecting the decision-making process.
    #[error("concurrent modification error")]
    Concurrency,
}
