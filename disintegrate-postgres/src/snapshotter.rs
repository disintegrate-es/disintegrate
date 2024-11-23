//! # PostgreSQL Snapshotter
//!
//! This module provides an implementation of the `Snapshotter` trait using PostgreSQL as the underlying storage.
//! It allows storing and retrieving snapshots from a PostgreSQL database.
use async_trait::async_trait;
use disintegrate::{BoxDynError, Event, IntoState, StateSnapshotter, StreamQuery};
use disintegrate::{StatePart, StateQuery};
use md5::{Digest, Md5};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;

use crate::Error;

#[cfg(test)]
mod tests;

/// PostgreSQL implementation for the `Snapshotter` trait.
///
/// The `PgSnapshotter` struct implements the `Snapshotter` trait for PostgreSQL databases.
/// It allows for stroring and retrieving snapshots of `StateQuery` from PostgreSQL database.
#[derive(Clone)]
pub struct PgSnapshotter {
    pool: PgPool,
    every: u64,
}

impl PgSnapshotter {
    /// Creates a new instance of `PgSnapshotter` with the specified PostgreSQL connection pool and snapshot frequency.
    ///
    /// # Arguments
    ///
    /// - `pool`: A PostgreSQL connection pool (`PgPool`) representing the database connection.
    /// - `every`: The frequency of snapshot creation, specified as the number of events between consecutive snapshots.
    ///
    /// # Returns
    ///
    /// A new `PgSnapshotter` instance.
    pub async fn new(pool: PgPool, every: u64) -> Result<Self, Error> {
        setup(&pool).await?;
        Ok(Self { pool, every })
    }
}

#[async_trait]
impl StateSnapshotter for PgSnapshotter {
    async fn load_snapshot<S>(&self, default: StatePart<S>) -> StatePart<S>
    where
        S: Send + Sync + DeserializeOwned + StateQuery + 'static,
    {
        let query = query_key(&default.query());
        let stored_snapshot =
            sqlx::query("SELECT name, query, payload, version FROM snapshot where id = $1")
                .bind(snapshot_id(S::NAME, &query))
                .fetch_one(&self.pool)
                .await;
        if let Ok(row) = stored_snapshot {
            let snapshot_name: String = row.get(0);
            let snapshot_query: String = row.get(1);
            if S::NAME == snapshot_name && query == snapshot_query {
                let payload = serde_json::from_str(row.get(2)).unwrap_or(default.into_state());
                return StatePart::new(row.get(3), payload);
            }
        }

        default
    }

    async fn store_snapshot<S>(&self, state: &StatePart<S>) -> Result<(), BoxDynError>
    where
        S: Send + Sync + Serialize + StateQuery + 'static,
    {
        if state.applied_events() <= self.every {
            return Ok(());
        }
        let query = query_key(&state.query());
        let id = snapshot_id(S::NAME, &query);
        let version = state.version();
        let payload = serde_json::to_string(&state.clone().into_state())?;
        sqlx::query("INSERT INTO snapshot (id, name, query, payload, version) VALUES ($1,$2,$3,$4,$5) ON CONFLICT(id) DO UPDATE SET name = $2, query = $3, payload = $4, version = $5 WHERE snapshot.version < $5")
        .bind(id)
        .bind(S::NAME)
        .bind(query)
        .bind(payload)
        .bind(version)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

fn snapshot_id(state_name: &str, query: &str) -> Uuid {
    let mut hasher = Md5::new();
    hasher.update(state_name);

    uuid::Uuid::new_v3(
        &uuid::Uuid::from_bytes(hasher.finalize().into()),
        query.as_bytes(),
    )
}

fn query_key<E: Event + Clone>(query: &StreamQuery<E>) -> String {
    let mut result = String::new();
    for f in query.filters() {
        let excluded_events = if let Some(exclued_events) = f.excluded_events() {
            format!("-{}", exclued_events.join(","))
        } else {
            "".to_string()
        };
        result += &format!(
            "({}|{}{}|{})",
            f.origin(),
            f.events().join(","),
            excluded_events,
            f.identifiers()
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    result
}

pub async fn setup(pool: &PgPool) -> Result<(), Error> {
    sqlx::query(include_str!("snapshotter/sql/table_snapshot.sql"))
        .execute(pool)
        .await?;
    Ok(())
}
