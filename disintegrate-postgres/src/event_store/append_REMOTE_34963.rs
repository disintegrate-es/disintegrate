use disintegrate::Event;
use disintegrate_serde::Serde;
use sqlx::postgres::PgArguments;
use sqlx::query::Query;
use sqlx::Postgres;
use sqlx::types::Json;

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
        let mut separated_builder = self.builder.separated(",");

        separated_builder.push("event_type");
        separated_builder.push("payload");
        separated_builder.push("domain_ids");

        separated_builder.push_unseparated(") ");

        self.builder.push_values(self.events, |mut b, event| {
            b.push_bind(event.name());
            b.push_bind(self.serde.serialize(event.clone()));
            b.push_bind(
                Json(event.domain_identifiers()),
            );
        });

        self.builder.push(" RETURNING (event_id)");
        self.builder.build()
    }
}

#[cfg(test)]
mod tests {}
