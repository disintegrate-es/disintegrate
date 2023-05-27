use thiserror::Error;

/// Represents all the ways a method can fail within Disintegrate Postgres.
#[derive(Error, Debug)]
pub enum Error {
    /// Error returned from the database.
    #[error("database error")]
    Database(#[from] sqlx::Error),
    // An error occurred while attempting to persist events using an outdated version of the event set.
    ///
    /// This error indicates that another process has inserted a new event that was not included in the event stream query
    /// used to make the current business decision. The event store's state has changed, potentially affecting the decision-making process.
    #[error("concurrent modification error")]
    Concurrency,
}
