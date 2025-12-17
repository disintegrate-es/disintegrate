//! # PostgreSQL Disintegrate Backend Library
mod error;
mod event_store;
#[cfg(feature = "listener")]
mod listener;
mod migrator;
mod snapshotter;

pub use crate::event_store::PgEventStore;
#[cfg(feature = "listener")]
pub use crate::listener::{
    id_indexer::{Error as PgIdIndexerError, PgIdIndexer},
    PgEventListener, PgEventListenerConfig,
};
pub use crate::snapshotter::PgSnapshotter;
use disintegrate::{DecisionMaker, Event, EventSourcedStateStore, SnapshotConfig, WithSnapshot};
use disintegrate_serde::Serde;
pub use error::Error;
pub use migrator::Migrator;

pub type PgEventId = i64;

/// An alias for [`DecisionMaker`], specialized for Postgres.
pub type PgDecisionMaker<E, S, SN> =
    DecisionMaker<EventSourcedStateStore<PgEventId, E, PgEventStore<E, S>, SN>>;

/// An alias for [`WithSnapshot`], specialized for Postgres.
pub type WithPgSnapshot = WithSnapshot<PgEventId, PgSnapshotter>;

/// Creates a decision maker specialized for PostgreSQL.
///
/// # Arguments
///
/// - `event_store`: An instance of `PgEventStore`.
/// - `snapshot_config`: The `SnapshotConfig` to be used for the snapshotting.
///
/// # Returns
///
/// A `PgDecisionMaker` with snapshotting configured according to the provided `snapshot_config`.
pub fn decision_maker<
    E: Event + Send + Sync + Clone,
    S: Serde<E> + Clone + Sync + Send,
    SN: SnapshotConfig + Clone,
>(
    event_store: PgEventStore<E, S>,
    snapshot_config: SN,
) -> PgDecisionMaker<E, S, SN> {
    DecisionMaker::new(EventSourcedStateStore::new(event_store, snapshot_config))
}
