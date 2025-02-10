use crate::PgEventId;
use disintegrate::Event;
use disintegrate::StreamQuery;

/// SQL Query Builder
///
/// A builder for constructing SQL query based on the stream query.
pub struct CriteriaBuilder<'a, QE>
where
    QE: Event + Clone,
{
    query: &'a StreamQuery<PgEventId, QE>,
    builder: String,
    alias: Option<&'a str>,
}

impl<'a, QE> CriteriaBuilder<'a, QE>
where
    QE: Event + Clone,
{
    /// Creates a new instance of `QueryBuilder`.
    ///
    /// # Arguments
    ///
    /// * `query` - The stream query specifying the filtering and ordering options.
    /// * `init` - The initial SQL fragment.
    pub fn new(query: &'a StreamQuery<PgEventId, QE>) -> Self {
        Self {
            query,
            builder: String::new(),
            alias: None,
        }
    }

    pub fn alias(mut self, alias: &'a str) -> Self {
        self.alias = Some(alias);
        self
    }

    /// Builds the SQL criteria string.
    pub fn build(mut self) -> String {
        let alias_prefix = self
            .alias
            .map(|alias| format!("{alias}."))
            .unwrap_or_default();
        let mut filters = self.query.filters().iter().peekable();
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
            let has_events = !events.is_empty();
            self.builder.push_str("(");
            if filter.origin() > 0 {
                self.builder.push_str(&format!("{alias_prefix}event_id > "));
                self.builder.push_str(&filter.origin().to_string());
                if has_events {
                    self.builder.push_str(" AND (");
                }
            }

            let mut events = events.into_iter().peekable();
            while let Some(event) = events.next() {
                self.builder.push_str("(");
                self.builder
                    .push_str(&format!("{alias_prefix}event_type = '{event}'"));
                let event_info = QE::SCHEMA.event_info(event).unwrap();
                let mut event_identifiers = filter
                    .identifiers()
                    .iter()
                    .filter(|(ident, _)| event_info.has_domain_identifier(ident))
                    .peekable();

                event_identifiers
                    .peek()
                    .map(|_| self.builder.push_str(" AND "));

                while let Some((ident, value)) = event_identifiers.next() {
                    self.builder.push_str(&format!("{alias_prefix}{ident} = "));
                    match value {
                        disintegrate::IdentifierValue::String(value) => {
                            self.builder.push_str(&format!("'{}'", value.clone()));
                        }
                        disintegrate::IdentifierValue::i64(value) => {
                            self.builder.push_str(&value.to_string())
                        }
                        disintegrate::IdentifierValue::Uuid(value) => {
                            self.builder.push_str(&format!("'{}'", value.clone()))
                        }
                    };
                    event_identifiers
                        .peek()
                        .map(|_| self.builder.push_str(" AND "));
                }
                self.builder.push_str(")");
                events.peek().map(|_| self.builder.push_str(" OR "));
            }
            if filter.origin() > 0 && has_events {
                self.builder.push_str(")");
            }
            self.builder.push_str(")");
            filters.peek().map(|_| self.builder.push_str(" OR "));
        }

        self.builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use disintegrate::{
        domain_identifiers, event_types, ident, query, DomainIdentifierInfo, DomainIdentifierSet,
        Event, EventInfo, EventSchema, IdentifierType,
    };

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
    fn it_builds_criteria() {
        let query = query!(TestEvent);
        let criteria_builder = CriteriaBuilder::new(&query);

        assert_eq!(
            criteria_builder.build(),
            "((event_type = 'Bar') OR (event_type = 'Foo'))"
        );
    }

    #[test]
    fn it_builds_criteria_with_an_id_filter() {
        let query = query!(TestEvent; foo_id == "value");
        let criteria_builder = CriteriaBuilder::new(&query);

        assert_eq!(
            criteria_builder.build(),
            "((event_type = 'Bar') OR (event_type = 'Foo' AND foo_id = 'value'))"
        );
    }

    #[test]
    fn it_builds_criteria_with_two_ids() {
        let query = query!(TestEvent; foo_id == "value", bar_id == "value2");
        let criteria_builder = CriteriaBuilder::new(&query);

        assert_eq!(
            criteria_builder.build(),
            "((event_type = 'Bar' AND bar_id = 'value2') OR (event_type = 'Foo' AND foo_id = 'value'))"
        );
    }

    #[test]
    fn it_builds_criteria_with_origin() {
        let query = query!(10 => TestEvent; foo_id == "value");
        let criteria_builder = CriteriaBuilder::new(&query);

        assert_eq!(
            criteria_builder.build(),
            "(event_id > 10 AND ((event_type = 'Bar') OR (event_type = 'Foo' AND foo_id = 'value')))"
        );
    }

    #[test]
    fn it_builds_criteria_with_union() {
        let query: StreamQuery<PgEventId, TestEvent> =
            query!(TestEvent; bar_id == "value1").union(&query!(TestEvent; foo_id == "value2"));
        let criteria_builder = CriteriaBuilder::new(&query);

        assert_eq!(
            criteria_builder.build(),
            "((event_type = 'Bar' AND bar_id = 'value1') OR (event_type = 'Foo')) OR ((event_type = 'Bar') OR (event_type = 'Foo' AND foo_id = 'value2'))"
        );
    }

    #[test]
    fn it_builds_criteria_with_excluded_events() {
        let query =
            query!(TestEvent; bar_id == "value1").exclude_events(event_types!(TestEvent, [Bar]));
        let criteria_builder = CriteriaBuilder::new(&query);

        assert_eq!(criteria_builder.build(), r#"((event_type = 'Foo'))"#);
    }
}
