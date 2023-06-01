use disintegrate::stream_query::{FilterEvaluator, StreamFilter, StreamQuery};
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
    query: &'a StreamQuery<QE>,
    builder: sqlx::QueryBuilder<'a, Postgres>,
    origin: i64,
    last_event_id: Option<i64>,
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
    pub fn new(query: &'a StreamQuery<QE>, init: &str) -> Self {
        Self {
            query,
            builder: sqlx::QueryBuilder::new(init),
            origin: query.origin(),
            last_event_id: None,
            end: None,
        }
    }

    /// Sets the origin value for the criteria.
    ///
    /// # Arguments
    ///
    /// * `origin` - The origin value.
    pub fn with_origin(mut self, origin: i64) -> Self {
        self.origin = origin;
        self
    }

    /// Sets the last event ID for the criteria.
    ///
    /// # Arguments
    ///
    /// * `last_event_id` - The last event ID value.
    pub fn with_last_event_id(mut self, last_event_id: i64) -> Self {
        self.last_event_id = Some(last_event_id);
        self
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
        self.builder.push(format!("event_id >= {}", self.origin));

        if let Some(last_event_id) = self.last_event_id {
            self.builder
                .push(format!(" AND event_id <= {last_event_id}"));
        };

        // Build the domain identifiers condition
        if let Some(condition) = self.query.filter() {
            self.builder.push(" AND (");
            self.eval(condition);
            self.builder.push(")");
        }

        // Build the event types condition
        if !QE::SCHEMA.types.is_empty() {
            let list = QE::SCHEMA
                .types
                .iter()
                .map(|t| format!("'{t}'"))
                .collect::<Vec<_>>()
                .join(",");
            self.builder.push(format!(" AND event_type IN ({list})"));
        }

        if let Some(end) = self.end {
            self.builder.push(format!(" {end}"));
        }
        self.builder.build()
    }
}

/// Filter Evaluator for SQL Events Criteria Builder
///
/// Implements the `FilterEvaluator` trait for evaluating stream filters and generating corresponding
/// SQL conditions for the SQL Events Criteria Builder.
impl<QE> FilterEvaluator for QueryBuilder<'_, QE>
where
    QE: Event + Clone,
{
    type Result = ();

    /// Evaluates the stream filter and generates the corresponding SQL condition.
    ///
    /// # Arguments
    ///
    /// * `filter` - The stream filter to evaluate.
    ///
    /// # Returns
    ///
    /// The generated SQL condition as a `String`.
    fn eval(&mut self, filter: &StreamFilter) -> Self::Result {
        match filter {
            StreamFilter::Eq { ident, value } => {
                self.builder.push(format!("{ident} = "));
                self.builder.push_bind(value.clone());
            }
            StreamFilter::And { l, r } => {
                self.builder.push("(");
                self.eval(l);
                self.builder.push(") AND (");
                self.eval(r);
                self.builder.push(")");
            }
            StreamFilter::Or { l, r } => {
                self.builder.push("(");
                self.eval(l);
                self.builder.push(") OR (");
                self.eval(r);
                self.builder.push(")");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use disintegrate::{domain_identifiers, query, DomainIdentifierSet, Event, EventSchema};
    use sqlx::Execute;

    #[derive(Clone)]
    enum TestEvent {}

    impl Event for TestEvent {
        const SCHEMA: EventSchema = EventSchema {
            types: &[],
            domain_identifiers: &[],
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
        let query = query!(TestEvent, id == "value");
        let mut sql_builder = QueryBuilder::new(&query, "SELECT * FROM event WHERE ");

        assert_eq!(
            sql_builder.build().sql(),
            r#"SELECT * FROM event WHERE event_id >= 0 AND (id = $1)"#
        );
    }

    #[test]
    fn it_builds_query_with_an_and_filter() {
        let query = query!(TestEvent, (id1 == "value") and (id2 == "value2"));
        let mut sql_builder = QueryBuilder::new(&query, "SELECT * FROM event WHERE ");

        assert_eq!(
            sql_builder.build().sql(),
            r#"SELECT * FROM event WHERE event_id >= 0 AND ((id1 = $1) AND (id2 = $2))"#
        );
    }

    #[test]
    fn it_builds_query_with_an_or_filter() {
        let query = query!(TestEvent, (id1 == "value") or (id2 == "value2"));
        let mut sql_builder = QueryBuilder::new(&query, "SELECT * FROM event WHERE ");

        assert_eq!(
            sql_builder.build().sql(),
            r#"SELECT * FROM event WHERE event_id >= 0 AND ((id1 = $1) OR (id2 = $2))"#
        );
    }

    #[test]
    fn it_builds_query_with_origin() {
        let query = query!(TestEvent, id == "value");
        let mut sql_builder =
            QueryBuilder::new(&query, "SELECT * FROM event WHERE ").with_origin(10);

        assert_eq!(
            sql_builder.build().sql(),
            r#"SELECT * FROM event WHERE event_id >= 10 AND (id = $1)"#
        );
    }

    #[test]
    fn it_builds_query_with_last_event_id() {
        let query = query!(TestEvent, id == "value");
        let mut sql_builder =
            QueryBuilder::new(&query, "SELECT * FROM event WHERE ").with_last_event_id(20);

        assert_eq!(
            sql_builder.build().sql(),
            r#"SELECT * FROM event WHERE event_id >= 0 AND event_id <= 20 AND (id = $1)"#
        );
    }
}
