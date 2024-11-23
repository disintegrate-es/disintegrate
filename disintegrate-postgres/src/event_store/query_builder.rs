use disintegrate::stream_query::StreamQuery;
use disintegrate::Event;
use sqlx::postgres::PgArguments;
use sqlx::query::Query;
use sqlx::Postgres;

/// SQL Query Builder
///
/// A builder for constructing SQL query based on the stream query.
pub struct QueryBuilder<'a, QE>
where
    QE: Event + Clone,
{
    query: StreamQuery<QE>,
    builder: sqlx::QueryBuilder<'a, Postgres>,
    end: Option<&'a str>,
}

impl<'a, QE> QueryBuilder<'a, QE>
where
    QE: Event + Clone,
{
    /// Creates a new instance of `QueryBuilder`.
    ///
    /// # Arguments
    ///
    /// * `query` - The stream query specifying the filtering and ordering options.
    /// * `init` - The initial SQL fragment.
    pub fn new(query: StreamQuery<QE>, init: &str) -> Self {
        Self {
            query,
            builder: sqlx::QueryBuilder::new(init),
            end: None,
        }
    }

    /// Sets the end SQL fragment of the query.
    ///
    /// # Arguments
    ///
    /// * `end` - The SQL fragment to be set as the end of the query.
    pub fn end_with(mut self, end: &'a str) -> Self {
        self.end = Some(end);
        self
    }

    /// Builds the SQL criteria string.
    pub fn build(&'a mut self) -> Query<'a, Postgres, PgArguments> {
        self.build_criteria(self.query.clone());

        if let Some(end) = self.end {
            self.builder.push(format!(" {end}"));
        }
        self.builder.build()
    }

    fn build_criteria(&mut self, query: StreamQuery<QE>) {
        let mut filters = query.filters().iter().peekable();
        while let Some(filter) = filters.next() {
            let events: Vec<&str> = if let Some(excluted_event) = filter.excluded_events() {
                filter
                    .events()
                    .iter()
                    .filter(|e| !excluted_event.contains(e))
                    .cloned()
                    .collect()
            } else {
                filter.events().to_vec()
            };
            self.builder.push("(");
            if filter.origin() > 0 {
                self.builder.push("event_id > ");
                self.builder.push(filter.origin());
                if !events.is_empty() {
                    self.builder.push(" AND ");
                }
            }

            let mut events = events.into_iter().peekable();
            while let Some(event) = events.next() {
                self.builder.push("(");
                self.builder.push(format!("event_type = '{event}'"));
                let event_info = QE::SCHEMA.event_info(event).unwrap();
                let mut event_identifiers = filter
                    .identifiers()
                    .iter()
                    .filter(|(ident, _)| event_info.has_domain_identifier(ident))
                    .peekable();

                event_identifiers.peek().map(|_| self.builder.push(" AND "));

                while let Some((ident, value)) = event_identifiers.next() {
                    self.builder.push(format!("{ident} = "));
                    match value {
                        disintegrate::IdentifierValue::String(value) => {
                            self.builder.push_bind(value.clone())
                        }
                        disintegrate::IdentifierValue::i64(value) => self.builder.push_bind(*value),
                        disintegrate::IdentifierValue::Uuid(value) => {
                            self.builder.push_bind(*value)
                        }
                    };
                    event_identifiers.peek().map(|_| self.builder.push(" AND "));
                }
                self.builder.push(")");
                events.peek().map(|_| self.builder.push(" OR "));
            }
            self.builder.push(")");
            filters.peek().map(|_| self.builder.push(" OR "));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use disintegrate::{
        domain_identifiers, event_types, ident, query, DomainIdentifierInfo, DomainIdentifierSet,
        Event, EventInfo, EventSchema, IdentifierType,
    };
    use sqlx::Execute;

    #[allow(dead_code)]
    #[derive(Clone)]
    enum TestEvent {
        Bar { bar_id: String },
        Foo { foo_id: String },
    }

    impl Event for TestEvent {
        const SCHEMA: EventSchema = EventSchema {
            events: &["Bar", "Foo"],
            events_info: &[
                &EventInfo {
                    name: "Bar",
                    domain_identifiers: &[&ident!(#bar_id)],
                },
                &EventInfo {
                    name: "Foo",
                    domain_identifiers: &[&ident!(#foo_id)],
                },
            ],
            domain_identifiers: &[
                &DomainIdentifierInfo {
                    ident: ident!(#foo_id),
                    type_info: IdentifierType::String,
                },
                &DomainIdentifierInfo {
                    ident: ident!(#bar_id),
                    type_info: IdentifierType::String,
                },
            ],
        };

        fn name(&self) -> &'static str {
            ""
        }
        fn domain_identifiers(&self) -> DomainIdentifierSet {
            domain_identifiers! {}
        }
    }
    #[test]
    fn it_builds_query() {
        let query = query!(TestEvent);
        let mut sql_builder = QueryBuilder::new(query, "SELECT * FROM event WHERE ");

        assert_eq!(
            sql_builder.build().sql(),
            "SELECT * FROM event WHERE ((event_type = 'Bar') OR (event_type = 'Foo'))"
        );
    }

    #[test]
    fn it_builds_query_with_an_id_filter() {
        let query = query!(TestEvent; foo_id == "value");
        let mut sql_builder = QueryBuilder::new(query, "SELECT * FROM event WHERE ");

        assert_eq!(
            sql_builder.build().sql(),
            "SELECT * FROM event WHERE ((event_type = 'Bar') OR (event_type = 'Foo' AND foo_id = $1))"
        );
    }

    #[test]
    fn it_builds_query_with_two_ids() {
        let query = query!(TestEvent; foo_id == "value", bar_id == "value2");
        let mut sql_builder = QueryBuilder::new(query, "SELECT * FROM event WHERE ");

        assert_eq!(
            sql_builder.build().sql(),
            "SELECT * FROM event WHERE ((event_type = 'Bar' AND bar_id = $1) OR (event_type = 'Foo' AND foo_id = $2))"
        );
    }

    #[test]
    fn it_builds_query_with_origin() {
        let query = query!(10 => TestEvent; foo_id == "value");
        let mut sql_builder = QueryBuilder::new(query, "SELECT * FROM event WHERE ");

        assert_eq!(
            sql_builder.build().sql(),
            "SELECT * FROM event WHERE (event_id > 10 AND (event_type = 'Bar') OR (event_type = 'Foo' AND foo_id = $1))"
        );
    }

    #[test]
    fn it_builds_query_with_union() {
        let query: StreamQuery<TestEvent> =
            query!(TestEvent; bar_id == "value1").union(&query!(TestEvent; foo_id == "value2"));
        let mut sql_builder = QueryBuilder::new(query, "SELECT * FROM event WHERE ");

        assert_eq!(
            sql_builder.build().sql(),
            "SELECT * FROM event WHERE ((event_type = 'Bar' AND bar_id = $1) OR (event_type = 'Foo')) OR ((event_type = 'Bar') OR (event_type = 'Foo' AND foo_id = $2))"
        );
    }

    #[test]
    fn it_builds_query_with_excluded_events() {
        let query =
            query!(TestEvent; bar_id == "value1").exclude_events(event_types!(TestEvent, [Bar]));
        let mut sql_builder = QueryBuilder::new(query, "SELECT * FROM event WHERE ");

        assert_eq!(
            sql_builder.build().sql(),
            r#"SELECT * FROM event WHERE ((event_type = 'Foo'))"#
        );
    }
}
