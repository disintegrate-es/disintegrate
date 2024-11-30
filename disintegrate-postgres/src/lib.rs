//! # PostgreSQL Disintegrate Backend Library
mod error;
mod event_store;
#[cfg(feature = "listener")]
mod listener;
mod snapshotter;

pub use crate::event_store::PgEventStore;
#[cfg(feature = "listener")]
pub use crate::listener::{PgEventListener, PgEventListenerConfig};
pub use crate::snapshotter::PgSnapshotter;
use disintegrate::{
    DecisionMaker, Event, EventSourcedDecisionStateStore, NoSnapshot, WithSnapshot,
};
use disintegrate_serde::Serde;
pub use error::Error;

/// An alias for [`DecisionMaker`], specialized for Postgres.
pub type PgDecisionMaker<E, S, SN> =
    DecisionMaker<EventSourcedDecisionStateStore<i64, E, PgEventStore<E, S>, SN>>;

/// An alias for [`WithSnapshot`], specialized for Postgres.
pub type WithPgSnapshot = WithSnapshot<i64, PgSnapshotter>;

/// Creates a decision maker specialized for Postgres with snapshotting.
/// The `every` parameter determines the frequency of snapshot creation, indicating the number of events
/// between consecutive snapshots.
///
/// # Arguments
///
/// - `event_store`: An instance of `PgEventStore`.
/// - `every`: The frequency of snapshot creation, specified as the number of events between consecutive snapshots.
///
/// # Returns
///
/// A `PgDecisionMaker` with snapshotting configured using the provided snapshot frequency.
pub async fn decision_maker_with_snapshot<
    E: Event + Send + Sync + Clone,
    S: Serde<E> + Clone + Sync + Send,
>(
    event_store: PgEventStore<E, S>,
    every: u64,
) -> Result<PgDecisionMaker<E, S, WithPgSnapshot>, Error> {
    let pool = event_store.pool.clone();
    let snapshot = WithSnapshot::new(PgSnapshotter::new(pool, every).await?);
    Ok(DecisionMaker::new(EventSourcedDecisionStateStore::new(
        event_store,
        snapshot,
    )))
}

/// Creates a decision maker specialized for Postgres without snapshotting.
///
/// # Arguments
///
/// - `event_store`: An instance of `PgEventStore`.
///
/// # Returns
///
/// A `PgDecisionMaker` without snapshotting.
pub fn decision_maker<E: Event + Send + Sync + Clone, S: Serde<E> + Clone + Sync + Send>(
    event_store: PgEventStore<E, S>,
) -> PgDecisionMaker<E, S, NoSnapshot> {
    DecisionMaker::new(EventSourcedDecisionStateStore::new(event_store, NoSnapshot))
}
