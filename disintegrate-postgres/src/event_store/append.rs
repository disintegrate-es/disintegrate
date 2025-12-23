use std::collections::BTreeSet;

use disintegrate::{Event, Identifier, PersistedEvent};
use disintegrate_serde::Serde;
use sqlx::postgres::PgArguments;
use sqlx::query::Query;
use sqlx::Postgres;

use crate::PgEventId;

/// SQL Insert Event Sequence Builder
///
/// A builder for constructing insert SQL queries for the `event_sequence` table.
pub struct InsertEventSequenceBuilder<'a, E>
where
    E: Event + Clone,
{
    builder: sqlx::QueryBuilder<'a, Postgres>,
    events: &'a [E],
    consumed: Option<bool>,
    committed: Option<bool>,
}

impl<'a, E> InsertEventSequenceBuilder<'a, E>
where
    E: Event + Clone,
{
    /// Creates a new instance of `InsertEventSequenceBuilder`.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to be inserted.
    pub fn new(events: &'a [E]) -> Self {
        Self {
            builder: sqlx::QueryBuilder::new("INSERT INTO event_sequence ("),
            events,
            consumed: None,
            committed: None,
        }
    }

    /// Sets the consumed flag for the event to be inserted.
    ///
    /// # Arguments
    ///
    /// * `consumed` - The value for the consumed flag.
    pub fn with_consumed(mut self, consumed: bool) -> Self {
        self.consumed = Some(consumed);
        self
    }

    /// Sets the committed flag for the event to be inserted.
    ///
    /// # Arguments
    ///
    /// * `committed` - The value for the consumed flag.
    #[cfg(test)]
    pub fn with_committed(mut self, committed: bool) -> Self {
        self.committed = Some(committed);
        self
    }

    /// Builds the SQL insert query.
    pub fn build(&'a mut self) -> Query<'a, Postgres, PgArguments> {
        let mut all_identifiers: BTreeSet<Identifier> = BTreeSet::new();
        for event in self.events.iter() {
            all_identifiers.extend(event.domain_identifiers().keys());
        }
        let mut separated_builder = self.builder.separated(",");

        separated_builder.push("event_type");

        if self.consumed.is_some() {
            separated_builder.push("consumed");
        }

        if self.committed.is_some() {
            separated_builder.push("committed");
        }

        for ident in &all_identifiers {
            separated_builder.push(ident);
        }

        separated_builder.push_unseparated(") ");

        self.builder.push_values(self.events, |mut b, event| {
            b.push_bind(event.name());
            if let Some(consumed) = self.consumed {
                b.push(if consumed { 1 } else { 0 });
            }
            if let Some(committed) = self.committed {
                b.push(committed);
            }
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

/// SQL Insert Event Builder
///
/// A builder for constructing insert SQL queries for the `event` table.
pub struct InsertEventsBuilder<'a, E, S>
where
    E: Event + Clone,
    S: Serde<E>,
{
    builder: sqlx::QueryBuilder<'a, Postgres>,
    events: &'a [PersistedEvent<PgEventId, E>],
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
    pub fn new(events: &'a [PersistedEvent<PgEventId, E>], serde: &'a S) -> Self {
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

        separated_builder.push("event_id");
        separated_builder.push("event_type");
        separated_builder.push("payload");
        for ident in &all_identifiers {
            separated_builder.push(ident);
        }

        separated_builder.push_unseparated(") ");

        self.builder.push_values(self.events, |mut b, event| {
            b.push_bind(event.id());
            b.push_bind(event.name());
            b.push_bind(self.serde.serialize(event.clone().into_inner()));
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
        self.builder.build()
    }
}

#[cfg(test)]
mod tests {
    use disintegrate::{
        domain_identifiers, ident, DomainIdentifierInfo, DomainIdentifierSet, EventInfo,
        EventSchema, IdentifierType,
    };
    use serde::{Deserialize, Serialize};
    use sqlx::Execute;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "event_type", rename_all = "snake_case")]
    enum ShoppingCartEvent {
        Added {
            product_id: String,
            cart_id: String,
            quantity: i64,
        },
        Removed {
            product_id: String,
            cart_id: String,
            quantity: i64,
        },
    }

    impl Event for ShoppingCartEvent {
        const SCHEMA: EventSchema = EventSchema {
            events: &["ShoppingCartAdded", "ShoppingCartRemoved"],
            events_info: &[
                &EventInfo {
                    name: "ShoppingCartAdded",
                    domain_identifiers: &[&ident!(#product_id), &ident!(#cart_id)],
                },
                &EventInfo {
                    name: "ShoppingCartRemoved",
                    domain_identifiers: &[&ident!(#product_id), &ident!(#cart_id)],
                },
            ],
            domain_identifiers: &[
                &DomainIdentifierInfo {
                    ident: ident!(#cart_id),
                    type_info: IdentifierType::String,
                },
                &DomainIdentifierInfo {
                    ident: ident!(#product_id),
                    type_info: IdentifierType::String,
                },
            ],
        };
        fn name(&self) -> &'static str {
            match self {
                ShoppingCartEvent::Added { .. } => "ShoppingCartAdded",
                ShoppingCartEvent::Removed { .. } => "ShoppingCartRemoved",
            }
        }
        fn domain_identifiers(&self) -> DomainIdentifierSet {
            match self {
                ShoppingCartEvent::Added {
                    product_id,
                    cart_id,
                    ..
                } => domain_identifiers! {product_id: product_id, cart_id: cart_id},
                ShoppingCartEvent::Removed {
                    product_id,
                    cart_id,
                    ..
                } => domain_identifiers! {product_id: product_id, cart_id: cart_id},
            }
        }
    }

    #[test]
    fn it_builds_event_sequence_insert() {
        let events = &[
            ShoppingCartEvent::Added {
                product_id: "product_1".into(),
                cart_id: "cart_1".into(),
                quantity: 10,
            },
            ShoppingCartEvent::Removed {
                product_id: "product_1".into(),
                cart_id: "cart_1".into(),
                quantity: 10,
            },
        ];
        let mut insert_query = InsertEventSequenceBuilder::new(events);
        assert_eq!(
            insert_query.build().sql(),
            "INSERT INTO event_sequence (event_type,cart_id,product_id) VALUES ($1, $2, $3), ($4, $5, $6) RETURNING (event_id)"
        );
    }

    #[test]
    fn it_builds_event_insert() {
        let events = &[
            PersistedEvent::new(
                0,
                ShoppingCartEvent::Added {
                    product_id: "product_1".into(),
                    cart_id: "cart_1".into(),
                    quantity: 10,
                },
            ),
            PersistedEvent::new(
                1,
                ShoppingCartEvent::Removed {
                    product_id: "product_1".into(),
                    cart_id: "cart_1".into(),
                    quantity: 10,
                },
            ),
        ];
        let serde = disintegrate_serde::serde::json::Json::default();
        let mut insert_query = InsertEventsBuilder::new(events, &serde);
        assert_eq!(
            insert_query.build().sql(),
            "INSERT INTO event (event_id,event_type,payload,cart_id,product_id) VALUES ($1, $2, $3, $4, $5), ($6, $7, $8, $9, $10)"
        );
    }
}
