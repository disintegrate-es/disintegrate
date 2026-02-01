use std::collections::BTreeSet;

use disintegrate::{Event, Identifier, PersistedEvent};
use disintegrate_serde::Serde;
use sqlx::postgres::PgArguments;
use sqlx::query::Query;
use sqlx::Postgres;

/// SQL Insert Event Builder
///
/// A builder for constructing insert SQL queries for the `event` table.
pub struct InsertEventsBuilder<'a, E, S>
where
    E: Event + Clone,
    S: Serde<E>,
{
    builder: sqlx::QueryBuilder<'a, Postgres>,
    events: &'a [E],
    serde: &'a S,
}

impl<'a, E, S> InsertEventsBuilder<'a, E, S>
where
    E: Event + Clone,
    S: Serde<E>,
{
    /// Creates a new instance of `InsertEventBuilder` for batch inserts.
    ///
    /// # Arguments
    ///
    /// * `events` - The events to be inserted.
    /// * `serde` - The serialization implementation for the event payload.
    pub fn new(events: &'a [E], serde: &'a S) -> Self {
        Self {
            builder: sqlx::QueryBuilder::new("INSERT INTO event ("),
            events,
            serde,
        }
    }

    /// Builds the SQL batch insert query.
    pub fn build(&'a mut self) -> Query<'a, Postgres, PgArguments> {
        if self.events.is_empty() {
            panic!("Cannot build an insert query with no events");
        }

        let mut all_identifiers: BTreeSet<Identifier> = BTreeSet::new();
        for event in self.events.iter() {
            all_identifiers.extend(event.domain_identifiers().keys());
        }

        let mut separated_builder = self.builder.separated(",");

        separated_builder.push("event_type");
        separated_builder.push("payload");
        for ident in &all_identifiers {
            separated_builder.push(ident);
        }

        separated_builder.push_unseparated(") ");

        self.builder.push_values(self.events, |mut b, event| {
            b.push_bind(event.name());
            b.push_bind(self.serde.serialize(event.clone()));
            let event_identifiers = event.domain_identifiers();
            for ident in &all_identifiers {
                if let Some(value) = event_identifiers.get(ident) {
                    match value {
                        disintegrate::IdentifierValue::String(value) => b.push_bind(value.clone()),
                        disintegrate::IdentifierValue::i64(value) => b.push_bind(*value),
                        disintegrate::IdentifierValue::Uuid(value) => b.push_bind(*value),
                    };
                } else {
                    b.push("NULL");
                }
            }
        });

        self.builder.push(" RETURNING (event_id)");
        self.builder.build()
    }
}

#[cfg(test)]
mod tests {
}
