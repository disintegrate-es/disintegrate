<<<<<<< HEAD
use std::collections::BTreeSet;

use disintegrate::{Event, Identifier};
=======
use disintegrate::Event;
>>>>>>> ebf68a3 (add gin index)
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
<<<<<<< HEAD
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
    fn it_builds_event_insert() {
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
        let serde = disintegrate_serde::serde::json::Json::default();
        let mut insert_query = InsertEventsBuilder::new(events, &serde);
        assert_eq!(
            insert_query.build().sql(),
            "INSERT INTO event (event_type,payload,cart_id,product_id) VALUES ($1, $2, $3, $4), ($5, $6, $7, $8) RETURNING (event_id)"
        );
    }
}
=======
mod tests {}
>>>>>>> ebf68a3 (add gin index)
