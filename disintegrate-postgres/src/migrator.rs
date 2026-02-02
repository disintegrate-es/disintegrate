//! Database initialization and migration utilities for the `PgEventStore`.
//!
//! This module provides helpers to:
//! - initialize the PostgreSQL schema for a fresh deployment
//! - apply schema migrations when upgrading versions
//!
//! The migrator is typically executed during application startup or via
//! dedicated administrative tooling.
use disintegrate::{DomainIdInfo, Event};
use disintegrate_serde::Serde;

use crate::PgEventStore;

/// PostgreSQL Migrator error.
#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub struct Error(#[from] sqlx::Error);

/// Helper for initializing and migrating the `PgEventStore` database schema.
///
/// `Migrator` encapsulates the logic required to:
/// - initialize the database for a fresh deployment
/// - apply schema migrations when upgrading between versions
///
/// It is intended to be used during application startup or in dedicated
/// maintenance / administration workflows.
pub struct Migrator<E, S>
where
    E: Event + Clone,
    S: Serde<E> + Send + Sync,
{
    event_store: PgEventStore<E, S>,
}

impl<E, S> Migrator<E, S>
where
    E: Event + Clone,
    S: Serde<E> + Send + Sync,
{
    pub fn new(event_store: PgEventStore<E, S>) -> Self {
        Self { event_store }
    }

    /// Init `PgEventStore` database
    pub async fn init_event_store(&self) -> Result<(), Error> {
        const RESERVED_NAMES: &[&str] = &["event_id", "payload", "event_type", "inserted_at"];

        sqlx::query(include_str!("event_store/sql/seq_event_event_id.sql"))
            .execute(&self.event_store.pool)
            .await?;
        sqlx::query(include_str!("event_store/sql/table_event.sql"))
            .execute(&self.event_store.pool)
            .await?;
        sqlx::query(include_str!("event_store/sql/idx_event_type.sql"))
            .execute(&self.event_store.pool)
            .await?;
        for domain_id in E::SCHEMA.domain_ids {
            if RESERVED_NAMES.contains(&domain_id.ident) {
                panic!(
                    "Domain id name {domain_id} is reserved. Please use a different name.",
                    domain_id = domain_id.ident
                );
            }
            self.add_domain_id_column("event", domain_id).await?;
        }
        Ok(())
    }

    /// Init `PgEventListener` database
    pub async fn init_listener(&self) -> Result<(), Error> {
        sqlx::query(include_str!("listener/sql/table_event_listener.sql"))
            .execute(&self.event_store.pool)
            .await?;
        sqlx::query(include_str!("listener/sql/fn_notify_event_listener.sql"))
            .execute(&self.event_store.pool)
            .await?;
        sqlx::query(include_str!(
            "listener/sql/trigger_notify_event_listener.sql"
        ))
        .execute(&self.event_store.pool)
        .await?;
        Ok(())
    }

    /// Migrate the database from version 2.1.0 to version 3.0.0
    ///
    /// Backward compatible migration replacing `HASH` indexes with `BTREE` indexes
    /// to improve performance.
    pub async fn migrate_v2_1_0_to_v3_0_0(&self) -> Result<(), Error> {
        self.migrate_hash_index_to_btree("idx_event_sequence_type", "event_sequence", "event_type")
            .await?;

        self.migrate_hash_index_to_btree("idx_events_type", "event", "event_type")
            .await?;

        for domain_id in E::SCHEMA.domain_ids {
            let column_name = domain_id.ident;

            self.migrate_hash_index_to_btree(
                &format!("idx_event_{column_name}"),
                "event",
                &column_name,
            )
            .await?;
            self.migrate_hash_index_to_btree(
                &format!("idx_event_sequence_{column_name}"),
                "event_sequence",
                &column_name,
            )
            .await?;
        }
        Ok(())
    }

    /// Migrate the database from version 3.x.x to version 4.0.0
    pub async fn migrate_v3_x_x_to_v4_0_0(&self) -> Result<(), Error> {
        sqlx::query(
            "SELECT setval('seq_event_event_id', COALESCE((SELECT MAX(event_id) FROM event), 0))",
        )
        .execute(&self.event_store.pool)
        .await?;
        sqlx::query(
            "ALTER TABLE event ALTER COLUMN event_id SET DEFAULT nextval('seq_event_event_id')",
        )
        .execute(&self.event_store.pool)
        .await?;
        Ok(())
    }

    async fn add_domain_id_column(
        &self,
        table: &str,
        domain_id: &DomainIdInfo,
    ) -> Result<(), Error> {
        let column_name = domain_id.ident;
        let sql_type = match domain_id.type_info {
            disintegrate::IdentifierType::String => "TEXT",
            disintegrate::IdentifierType::i64 => "BIGINT",
            disintegrate::IdentifierType::Uuid => "UUID",
        };
        sqlx::query(&format!(
            "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS {column_name} {sql_type}"
        ))
        .execute(&self.event_store.pool)
        .await?;

        sqlx::query(&format!(
            "CREATE INDEX IF NOT EXISTS idx_{table}_{column_name} ON {table} ({column_name}) WHERE {column_name} IS NOT NULL"
        ))
        .execute(&self.event_store.pool)
        .await?;
        Ok(())
    }

    async fn migrate_hash_index_to_btree(
        &self,
        index_name: &str,
        table: &str,
        column: &str,
    ) -> Result<(), Error> {
        let index_type: Option<String> = sqlx::query_scalar(&format!("SELECT am.amname AS index_type FROM pg_class c JOIN pg_am am ON am.oid = c.relam WHERE c.relname = '{index_name}'"))
        .fetch_optional(&self.event_store.pool)
        .await?;

        if index_type.is_some_and(|ty| ty == "hash") {
            sqlx::query(&format!(
                "CREATE INDEX IF NOT EXISTS {index_name}_tmp ON {table} ({column})"
            ))
            .execute(&self.event_store.pool)
            .await?;

            sqlx::query(&format!("DROP INDEX CONCURRENTLY {index_name}"))
                .execute(&self.event_store.pool)
                .await?;

            sqlx::query(&format!(
                "ALTER INDEX {index_name}_tmp RENAME TO  {index_name}"
            ))
            .execute(&self.event_store.pool)
            .await?;
        }

        Ok(())
    }
}
