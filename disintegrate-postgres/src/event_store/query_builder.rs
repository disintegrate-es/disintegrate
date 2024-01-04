use disintegrate::stream_query::{StreamFilter, StreamQuery};
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
        self.build_criteria(self.query.clone().filter());

        if let Some(end) = self.end {
            self.builder.push(format!(" {end}"));
        }
        self.builder.build()
    }

    fn build_criteria(&mut self, filter: &StreamFilter) {
        match filter {
            StreamFilter::Origin { id } => {
                self.builder.push("event_id > ");
                self.builder.push_bind(*id);
            }
            StreamFilter::Events { names } => {
                self.builder.push(event_types_in(names));
            }
            StreamFilter::ExcludeEvents { names } => {
                self.builder.push(event_types_not_in(names));
            }
            StreamFilter::Eq { ident, value } => {
                self.builder.push(format!("{ident} in ("));
                match value {
                    disintegrate::IdentifierValue::String(value) => {
                        self.builder.push_bind(value.clone())
                    }
                    disintegrate::IdentifierValue::i64(value) => self.builder.push_bind(*value),
                    disintegrate::IdentifierValue::Uuid(value) => self.builder.push_bind(*value),
                };
                self.builder.push(",NULL)");
            }
            StreamFilter::And { l, r } => {
                self.builder.push("(");
                self.build_criteria(l);
                self.builder.push(") AND (");
                self.build_criteria(r);
                self.builder.push(")");
            }
            StreamFilter::Or { l, r } => {
                self.builder.push("(");
                self.build_criteria(l);
                self.builder.push(") OR (");
                self.build_criteria(r);
                self.builder.push(")");
            }
        }
    }
}

fn event_types_in(types: &[&str]) -> String {
    format!(
        "event_type IN ({})",
        types
            .iter()
            .map(|t| format!("'{t}'"))
            .collect::<Vec<String>>()
            .join(",")
    )
}

fn event_types_not_in(types: &[&str]) -> String {
    format!(
        "event_type NOT IN ({})",
        types
            .iter()
            .map(|t| format!("'{t}'"))
            .collect::<Vec<String>>()
            .join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use disintegrate::{
        domain_identifiers, event_types, ident, query, DomainIdentifierInfo, DomainIdentifierSet,
        Event, EventSchema, IdentifierType,
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
            types: &["Bar", "Foo"],
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
    fn it_builds_query_with_an_eq_filter() {
        let query = query!(TestEvent, foo_id == "value");
        let mut sql_builder = QueryBuilder::new(query, "SELECT * FROM event WHERE ");

        assert_eq!(
            sql_builder.build().sql(),
            r#"SELECT * FROM event WHERE (event_type IN ('Bar','Foo')) AND (foo_id in ($1,NULL))"#
        );
    }

    #[test]
    fn it_builds_query_with_an_and_filter() {
        let query = query!(TestEvent, (foo_id == "value") and (bar_id == "value2"));
        let mut sql_builder = QueryBuilder::new(query, "SELECT * FROM event WHERE ");

        assert_eq!(
            sql_builder.build().sql(),
            r#"SELECT * FROM event WHERE (event_type IN ('Bar','Foo')) AND ((foo_id in ($1,NULL)) AND (bar_id in ($2,NULL)))"#
        );
    }

    #[test]
    fn it_builds_query_with_an_or_filter() {
        let query = query!(TestEvent, (foo_id == "value") or (bar_id == "value2"));
        let mut sql_builder = QueryBuilder::new(query, "SELECT * FROM event WHERE ");

        assert_eq!(
            sql_builder.build().sql(),
            r#"SELECT * FROM event WHERE (event_type IN ('Bar','Foo')) AND ((foo_id in ($1,NULL)) OR (bar_id in ($2,NULL)))"#
        );
    }

    #[test]
    fn it_builds_query_with_origin() {
        let query = query!(10; TestEvent, foo_id == "value");
        let mut sql_builder = QueryBuilder::new(query, "SELECT * FROM event WHERE ");

        assert_eq!(
            sql_builder.build().sql(),
            r#"SELECT * FROM event WHERE (event_id > $1) AND ((event_type IN ('Bar','Foo')) AND (foo_id in ($2,NULL)))"#
        );
    }

    #[test]
    fn it_builds_query_with_events_filter() {
        let query = query!(TestEvent, (bar_id == "value1") and (events[Foo]));
        let mut sql_builder = QueryBuilder::new(query, "SELECT * FROM event WHERE ");

        assert_eq!(
            sql_builder.build().sql(),
            r#"SELECT * FROM event WHERE (event_type IN ('Bar','Foo')) AND ((bar_id in ($1,NULL)) AND (event_type IN ('Foo')))"#
        );
    }

    #[test]
    fn it_builds_query_with_excluded_events() {
        let query = query!(TestEvent, (bar_id == "value1") and (events[Foo]))
            .exclude_events(event_types!(TestEvent, [Bar]));
        let mut sql_builder = QueryBuilder::new(query, "SELECT * FROM event WHERE ");

        assert_eq!(
            sql_builder.build().sql(),
            r#"SELECT * FROM event WHERE (event_type NOT IN ('Bar')) AND ((event_type IN ('Bar','Foo')) AND ((bar_id in ($1,NULL)) AND (event_type IN ('Foo'))))"#
        );
    }
}
