use crate::PgEventId;
use disintegrate::Event;
use disintegrate::StreamQuery;
use sqlx::{Postgres, QueryBuilder};

/// Extension trait for `sqlx::QueryBuilder` that appends a [`StreamQuery`]
/// as a SQL criteria expression.
///
/// All values (origin, event type, identifier values) are bound as parameters.
/// Identifier column names come from a validated schema and are interpolated
/// directly as SQL identifiers.
pub trait QueryBuilderExt<'a> {
    fn push_stream_query<QE>(&mut self, query: &StreamQuery<PgEventId, QE>) -> &mut Self
    where
        QE: Event + Clone;
}

impl<'a> QueryBuilderExt<'a> for QueryBuilder<'a, Postgres> {
    fn push_stream_query<QE>(&mut self, query: &StreamQuery<PgEventId, QE>) -> &mut Self
    where
        QE: Event + Clone,
    {
        let mut filters = query.filters().iter().peekable();
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

            self.push("(");

            if filter.origin() > 0 {
                self.push("event_id > ");
                self.push_bind(filter.origin());

                if has_events {
                    self.push(" AND (");
                }
            }

            let mut events_iter = events.iter().peekable();
            while let Some(event) = events_iter.next() {
                self.push("(event_type = ");
                self.push_bind(event.to_string());

                let event_info = QE::SCHEMA.event_info(event).unwrap();
                let mut event_identifiers = filter
                    .identifiers()
                    .iter()
                    .filter(|(ident, _)| event_info.has_domain_id(ident))
                    .peekable();

                if event_identifiers.peek().is_some() {
                    self.push(" AND ");
                }

                while let Some((ident, value)) = event_identifiers.next() {
                    self.push(ident);
                    self.push(" = ");
                    match value {
                        disintegrate::IdentifierValue::String(v) => {
                            self.push_bind(v.clone());
                        }
                        disintegrate::IdentifierValue::i64(v) => {
                            self.push_bind(*v);
                        }
                        disintegrate::IdentifierValue::Uuid(v) => {
                            self.push_bind(*v);
                        }
                    };
                    if event_identifiers.peek().is_some() {
                        self.push(" AND ");
                    }
                }

                self.push(")");
                if events_iter.peek().is_some() {
                    self.push(" OR ");
                }
            }

            if filter.origin() > 0 && has_events {
                self.push(")");
            }

            self.push(")");
            if filters.peek().is_some() {
                self.push(" OR ");
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use disintegrate::{
        domain_ids, event_types, ident, query, DomainIdInfo, DomainIdSet, Event, EventInfo,
        EventSchema, IdentifierType,
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
                    domain_ids: &[&ident!(#bar_id)],
                },
                &EventInfo {
                    name: "Foo",
                    domain_ids: &[&ident!(#foo_id)],
                },
            ],
            domain_ids: &[
                &DomainIdInfo {
                    ident: ident!(#foo_id),
                    type_info: IdentifierType::String,
                },
                &DomainIdInfo {
                    ident: ident!(#bar_id),
                    type_info: IdentifierType::String,
                },
            ],
        };

        fn name(&self) -> &'static str {
            ""
        }
        fn domain_ids(&self) -> DomainIdSet {
            domain_ids! {}
        }
    }

    fn build_sql(query: &StreamQuery<PgEventId, TestEvent>) -> String {
        let mut qb = QueryBuilder::<Postgres>::new("");
        qb.push_stream_query(query);
        qb.sql().to_string()
    }

    #[test]
    fn it_builds_criteria() {
        let query = query!(TestEvent);
        assert_eq!(
            build_sql(&query),
            "((event_type = $1) OR (event_type = $2))"
        );
    }

    #[test]
    fn it_builds_criteria_with_an_id_filter() {
        let query = query!(TestEvent; foo_id == "value");
        assert_eq!(
            build_sql(&query),
            "((event_type = $1) OR (event_type = $2 AND foo_id = $3))"
        );
    }

    #[test]
    fn it_builds_criteria_with_two_ids() {
        let query = query!(TestEvent; foo_id == "value", bar_id == "value2");
        assert_eq!(
            build_sql(&query),
            "((event_type = $1 AND bar_id = $2) OR (event_type = $3 AND foo_id = $4))"
        );
    }

    #[test]
    fn it_builds_criteria_with_origin() {
        let query = query!(10 => TestEvent; foo_id == "value");
        assert_eq!(
            build_sql(&query),
            "(event_id > $1 AND ((event_type = $2) OR (event_type = $3 AND foo_id = $4)))"
        );
    }

    #[test]
    fn it_builds_criteria_with_union() {
        let query: StreamQuery<PgEventId, TestEvent> =
            query!(TestEvent; bar_id == "value1").union(&query!(TestEvent; foo_id == "value2"));
        assert_eq!(
            build_sql(&query),
            "((event_type = $1 AND bar_id = $2) OR (event_type = $3)) OR ((event_type = $4) OR (event_type = $5 AND foo_id = $6))"
        );
    }

    #[test]
    fn it_builds_criteria_with_excluded_events() {
        let query =
            query!(TestEvent; bar_id == "value1").exclude_events(event_types!(TestEvent, [Bar]));
        assert_eq!(build_sql(&query), "((event_type = $1))");
    }

    #[test]
    fn it_quotes_string_value_safely() {
        // A value with single quotes that would have caused SQL injection in the
        // previous string-interpolated implementation must now be bound.
        let query = query!(TestEvent; foo_id == "abc' OR '1'='1");
        let mut qb = QueryBuilder::<Postgres>::new("");
        qb.push_stream_query(&query);
        let sql = qb.sql();
        assert!(
            !sql.contains('\''),
            "value must not be inlined into the SQL, got: {sql}"
        );
        assert_eq!(
            sql,
            "((event_type = $1) OR (event_type = $2 AND foo_id = $3))"
        );
    }
}
