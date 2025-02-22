use crate::PgEventId;
use disintegrate::Event;
use disintegrate::StreamQuery;
use std::fmt::Write;

/// SQL Query Builder
///
/// A builder for constructing SQL query based on the stream query.
pub struct CriteriaBuilder<'a, QE>
where
    QE: Event + Clone,
{
    query: &'a StreamQuery<PgEventId, QE>,
    builder: String,
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
            builder: String::with_capacity(512),
        }
    }

    /// Builds the SQL criteria string.
    pub fn build(mut self) -> String {
        let mut filters = self.query.filters().iter().peekable();
        while let Some(filter) = filters.next() {
            let events: Vec<&str> = if let Some(excluded_events) = filter.excluded_events() {
                filter
                    .events()
                    .iter()
                    .filter(|e| !excluded_events.contains(e))
                    .cloned()
                    .collect()
            } else {
                filter.events().to_vec()
            };
            let has_events = !events.is_empty();

            // Start filter group
            self.builder.push('(');

            // Add event_id condition if needed
            if filter.origin() > 0 {
                write!(self.builder, "event_id > {}", filter.origin()).unwrap();

                if has_events {
                    write!(self.builder, " AND (").unwrap();
                }
            }

            // Process events
            let mut events = events.into_iter().peekable();
            while let Some(event) = events.next() {
                write!(self.builder, "(event_type = '{}'", event).unwrap();

                // Process identifiers
                let event_info = QE::SCHEMA.event_info(event).unwrap();
                let mut event_identifiers = filter
                    .identifiers()
                    .iter()
                    .filter(|(ident, _)| event_info.has_domain_identifier(ident))
                    .peekable();

                if event_identifiers.peek().is_some() {
                    write!(self.builder, " AND ").unwrap();
                }

                while let Some((ident, value)) = event_identifiers.next() {
                    write!(self.builder, "{} = ", ident).unwrap();
                    match value {
                        disintegrate::IdentifierValue::String(value) => {
                            write!(self.builder, "'{}'", value).unwrap();
                        }
                        disintegrate::IdentifierValue::i64(value) => {
                            write!(self.builder, "{}", value).unwrap();
                        }
                        disintegrate::IdentifierValue::Uuid(value) => {
                            write!(self.builder, "'{}'", value).unwrap();
                        }
                    };
                    if event_identifiers.peek().is_some() {
                        write!(self.builder, " AND ").unwrap();
                    }
                }

                self.builder.push(')');
                if events.peek().is_some() {
                    write!(self.builder, " OR ").unwrap();
                }
            }

            // Close events group if needed
            if filter.origin() > 0 && has_events {
                self.builder.push(')');
            }

            // Close filter group
            self.builder.push(')');
            if filters.peek().is_some() {
                write!(self.builder, " OR ").unwrap();
            }
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
