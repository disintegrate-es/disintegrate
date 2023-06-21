use disintegrate::Event;
use sqlx::postgres::PgArguments;
use sqlx::query::Query;
use sqlx::Postgres;

/// SQL Insert Builder
///
/// A builder for constructing insert SQL queries.
pub struct InsertBuilder<'a, E>
where
    E: Event + Clone,
{
    builder: sqlx::QueryBuilder<'a, Postgres>,
    event: &'a E,
    id: Option<i64>,
    payload: Option<&'a [u8]>,
    returning: Option<&'a str>,
}

impl<'a, E> InsertBuilder<'a, E>
where
    E: Event + Clone,
{
    /// Creates a new instance of `InsertBuilder`.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to be inserted.
    /// * `table` - The table name.
    pub fn new(event: &'a E, table: &str) -> Self {
        Self {
            builder: sqlx::QueryBuilder::new(format!("INSERT INTO {table} (")),
            event,
            id: None,
            payload: None,
            returning: None,
        }
    }

    /// Sets the ID for the event to be inserted.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the event.
    pub fn with_id(mut self, id: i64) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the payload for the event to be inserted.
    ///
    /// # Arguments
    ///
    /// * `payload` - The payload of the event.
    pub fn with_payload(mut self, payload: &'a [u8]) -> Self {
        self.payload = Some(payload);
        self
    }

    /// Sets the end SQL fragment of the query.
    ///
    /// # Arguments
    ///
    /// * `end` - The SQL fragment to be set as the end of the query.
    pub fn returning(mut self, returning: &'a str) -> Self {
        self.returning = Some(returning);
        self
    }

    /// Builds the SQL insert query.
    pub fn build(&'a mut self) -> Query<'a, Postgres, PgArguments> {
        let domain_identifiers = self.event.domain_identifiers();
        let mut separated_builder = self.builder.separated(",");

        separated_builder.push("event_type");

        for ident in domain_identifiers.keys() {
            separated_builder.push(ident);
        }

        if self.id.is_some() {
            separated_builder.push("event_id");
        }

        if self.payload.is_some() {
            separated_builder.push("payload");
        }

        separated_builder.push_unseparated(") VALUES (");

        separated_builder.push_bind_unseparated(self.event.name());

        for value in domain_identifiers.values() {
            separated_builder.push_bind(value.clone());
        }

        if let Some(id) = self.id {
            separated_builder.push_bind(id);
        }

        if let Some(payload) = self.payload {
            separated_builder.push_bind(payload);
        }

        separated_builder.push_unseparated(")");

        if let Some(returning) = self.returning {
            separated_builder.push_unseparated(format!(" RETURNING ({returning})"));
        }

        self.builder.build()
    }
}

#[cfg(test)]
mod tests {
    use disintegrate::{domain_identifiers, DomainIdentifierSet, EventSchema};
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
            types: &["ShoppingCartAdded", "ShoppingCartRemoved"],
            domain_identifiers: &["cart_id", "product_id"],
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
    fn it_builds_insert() {
        let event = ShoppingCartEvent::Added {
            product_id: "product_1".into(),
            cart_id: "cart_1".into(),
            quantity: 10,
        };
        let mut insert_query = InsertBuilder::new(&event, "event_sequence");

        assert_eq!(
            insert_query.build().sql(),
            "INSERT INTO event_sequence (event_type,cart_id,product_id) VALUES ($1,$2,$3)"
        );
    }

    #[test]
    fn it_builds_insert_with_id_and_payload() {
        let event = ShoppingCartEvent::Added {
            product_id: "product_1".into(),
            cart_id: "cart_1".into(),
            quantity: 10,
        };
        let payload: Vec<u8> = vec![];
        let mut insert_query = InsertBuilder::new(&event, "event")
            .with_id(1)
            .with_payload(&payload);

        assert_eq!(
            insert_query.build().sql(),
            "INSERT INTO event (event_type,cart_id,product_id,event_id,payload) VALUES ($1,$2,$3,$4,$5)"
        );
    }
}
