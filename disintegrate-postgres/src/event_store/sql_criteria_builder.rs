use disintegrate::stream_query::{FilterEvaluator, StreamFilter, StreamQuery};
use disintegrate::Event;

/// SQL Events Criteria Builder
///
/// A builder for constructing SQL criteria for filtering events based on the stream query.
pub struct SqlEventsCriteriaBuilder<'a, QE>
where
    QE: Event + Clone,
{
    query: &'a StreamQuery<QE>,
    origin: i64,
    last_event_id: Option<i64>,
}

impl<'a, QE> SqlEventsCriteriaBuilder<'a, QE>
where
    QE: Event + Clone,
{
    /// Creates a new instance of `SqlEventsCriteriaBuilder`.
    ///
    /// # Arguments
    ///
    /// * `query` - The stream query specifying the filtering and ordering options.
    pub fn new(query: &'a StreamQuery<QE>) -> Self {
        Self {
            query,
            origin: query.origin(),
            last_event_id: None,
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

    /// Builds the SQL criteria string.
    pub fn build(&self) -> String {
        // Build the domain identifiers condition
        let domain_identifiers_condition = if let Some(condition) = self.query.filter() {
            self.eval(condition)
        } else {
            "true".to_string()
        };

        // Build the event types condition
        let event_names = QE::NAMES;
        let event_names_condition = if !event_names.is_empty() {
            let list = event_names
                .iter()
                .map(|t| format!("'{t}'"))
                .collect::<Vec<_>>()
                .join(",");
            format!("event_type IN ({list})")
        } else {
            "true".to_string()
        };

        let last_event_id_condition = match self.last_event_id {
            None => "true".to_string(),
            Some(last_event_id) => format!("id <= {last_event_id}"),
        };

        let origin = self.origin;
        format!(
            "(id >= {origin} AND {last_event_id_condition}) AND ({domain_identifiers_condition}) AND {event_names_condition}")
    }
}

/// Filter Evaluator for SQL Events Criteria Builder
///
/// Implements the `FilterEvaluator` trait for evaluating stream filters and generating corresponding
/// SQL conditions for the SQL Events Criteria Builder.
impl<QE> FilterEvaluator for SqlEventsCriteriaBuilder<'_, QE>
where
    QE: Event + Clone,
{
    type Result = String;

    /// Evaluates the stream filter and generates the corresponding SQL condition.
    ///
    /// # Arguments
    ///
    /// * `filter` - The stream filter to evaluate.
    ///
    /// # Returns
    ///
    /// The generated SQL condition as a `String`.
    fn eval(&self, filter: &StreamFilter) -> Self::Result {
        match filter {
            StreamFilter::Eq { ident, value } => {
                format!(r#"domain_identifiers @> '{{"{}":"{}"}}'"#, ident, value)
            }
            StreamFilter::And { l, r } => format!("({}) AND ({})", self.eval(l), self.eval(r)),
            StreamFilter::Or { l, r } => format!("({}) OR ({})", self.eval(l), self.eval(r)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use disintegrate::{domain_identifiers, query, DomainIdentifierSet, Event};

    #[derive(Clone)]
    enum TestEvent {}

    impl Event for TestEvent {
        const NAMES: &'static [&'static str] = &[];
        fn name(&self) -> &'static str {
            ""
        }
        fn domain_identifiers(&self) -> DomainIdentifierSet {
            domain_identifiers! {}
        }
    }

    #[test]
    fn it_build_eq_filter() {
        let query = query!(TestEvent, filed == "value");
        let result = SqlEventsCriteriaBuilder::new(&query).build();
        assert_eq!(
            result,
            r#"(id >= 0 AND true) AND (domain_identifiers @> '{"filed":"value"}') AND true"#
        );
    }

    #[test]
    fn it_build_and_filter() {
        let query = query!(TestEvent, (filed1 == "value") and (field2 == "value2"));
        let result = SqlEventsCriteriaBuilder::new(&query).build();

        assert_eq!(
            result,
            r#"(id >= 0 AND true) AND ((domain_identifiers @> '{"filed1":"value"}') AND (domain_identifiers @> '{"field2":"value2"}')) AND true"#
        );
    }

    #[test]
    fn it_build_or_filter() {
        let query = query!(TestEvent, (filed1 == "value") or (field2 == "value2"));
        let result = SqlEventsCriteriaBuilder::new(&query).build();

        assert_eq!(
            result,
            r#"(id >= 0 AND true) AND ((domain_identifiers @> '{"filed1":"value"}') OR (domain_identifiers @> '{"field2":"value2"}')) AND true"#
        );
    }

    #[test]
    fn it_build_with_origin() {
        let query = query!(TestEvent, filed == "value");
        let result = SqlEventsCriteriaBuilder::new(&query)
            .with_origin(10)
            .build();

        assert_eq!(
            result,
            r#"(id >= 10 AND true) AND (domain_identifiers @> '{"filed":"value"}') AND true"#
        );
    }

    #[test]
    fn it_build_with_last_event_id() {
        let query = query!(TestEvent, filed == "value");
        let result = SqlEventsCriteriaBuilder::new(&query)
            .with_last_event_id(20)
            .build();

        assert_eq!(
            result,
            r#"(id >= 0 AND id <= 20) AND (domain_identifiers @> '{"filed":"value"}') AND true"#
        );
    }
}
