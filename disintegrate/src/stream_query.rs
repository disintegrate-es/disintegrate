//! A stream query represents a query for filtering event streams based on certain criteria.
//!
//! This module provides functionality for querying event streams using `StreamQuery`.
//! It allows you to define filters and constraints to narrow down the events of interest and specify
//! the starting point or origin within the event stream.
//!
//! The module also provides utility functions and macros for creating and combining stream filters,
//! such as `eq`, `and`, and `or`. These can be used to construct complex filter expressions.
//!
//! The `StreamFilter` enum defines different types of filters that can be applied to event streams,
//! including equality filters, logical AND filters, and logical OR filters. Filters are evaluated
//! using the `FilterEvaluator` trait, which provides an `eval` method for evaluating a filter against
//! an event.
use core::fmt::Debug;
use std::marker::PhantomData;

use crate::{domain_identifiers, event::EventId, DomainIdentifierSet, Event, PersistedEvent};

/// Represents a query for filtering event streams.
///
/// A `StreamQuery` is used to define filters and constraints for querying event streams.
/// It allows you to specify a filter to narrow down the events of interest and an origin
/// to determine the starting point of the query within the event stream.
#[derive(Debug, Clone)]
pub struct StreamQuery<ID: EventId, E: Event + Clone> {
    /// An optional filter applied to the event stream. It determines which events are included
    /// in the query results based on certain criteria.
    filters: Vec<StreamFilter<ID, E>>,
    /// A marker indicating the event type associated with the stream query.
    event_type: PhantomData<E>,
    /// A marker indicating the event id type associated with the stream query.
    event_id_type: PhantomData<ID>,
}

impl<ID: EventId, E: Event + Clone> StreamQuery<ID, E> {
    /// Returns the filter associated with the stream query, if any.
    pub fn filters(&self) -> &[StreamFilter<ID, E>] {
        &self.filters
    }

    /// Casts the stream query to a different event type.
    pub fn cast<U>(&self) -> StreamQuery<ID, U>
    where
        E: Event + Into<U>,
        U: Event + Clone,
    {
        StreamQuery {
            filters: self.filters.iter().map(|f| f.cast()).collect(),
            event_type: PhantomData,
            event_id_type: PhantomData,
        }
    }

    /// Unions two stream queries into a single query.
    pub fn union<U, O>(&self, other: &StreamQuery<ID, O>) -> StreamQuery<ID, U>
    where
        E: Event + Into<U>,
        U: Event + Clone,
        O: Event + Into<U> + Clone,
    {
        let filters = self
            .filters
            .iter()
            .map(|f| f.cast())
            .chain(other.filters.iter().map(|f| f.cast()))
            .collect();

        StreamQuery {
            filters,
            event_type: PhantomData,
            event_id_type: PhantomData,
        }
    }

    /// Changes the origin of the stream query.
    ///
    /// The origin determines the starting point of the query within the event stream.
    pub fn change_origin(self, origin: ID) -> Self {
        let filters = self
            .filters
            .iter()
            .map(|f| StreamFilter {
                origin,
                ..f.clone()
            })
            .collect();

        StreamQuery {
            filters,
            event_type: PhantomData,
            event_id_type: PhantomData,
        }
    }

    /// Excludes the specified events from the stream query.
    ///
    /// The excluded events are not included in the query results.
    pub fn exclude_events(self, excluded_events: &'static [&'static str]) -> Self {
        let filters = self
            .filters
            .iter()
            .map(|f| StreamFilter {
                excluded_events: Some(
                    excluded_events
                        .iter()
                        .filter(|e| f.events.contains(e))
                        .cloned()
                        .collect(),
                ),
                ..f.clone()
            })
            .collect();

        StreamQuery {
            filters,
            event_type: PhantomData,
            event_id_type: PhantomData,
        }
    }

    /// Checks if the stream query matches the given event.
    pub fn matches(&self, event: &PersistedEvent<ID, E>) -> bool {
        self.filters.iter().any(|filter| {
            if let Some(excluded_events) = &filter.excluded_events {
                if excluded_events.contains(&event.name()) {
                    return false;
                }
            }

            if !filter.events.contains(&event.name()) {
                return false;
            }

            if filter
                .identifiers
                .iter()
                .any(|(ident, value)| event.domain_identifiers().get(ident) != Some(value))
            {
                return false;
            }

            if event.id() <= filter.origin {
                return false;
            }

            true
        })
    }
}

impl<ID: EventId, E: Event + Clone + PartialEq> PartialEq for StreamQuery<ID, E> {
    fn eq(&self, other: &Self) -> bool {
        self.filters == other.filters
    }
}

/// Creates a new stream query with the given filter.
pub fn query<ID, E, O>(filter: Option<StreamFilter<ID, O>>) -> StreamQuery<ID, E>
where
    ID: EventId,
    E: Event + Clone,
    O: Event + Clone + Into<E>,
{
    if let Some(filter) = filter {
        StreamQuery {
            filters: vec![filter.cast()],
            event_type: PhantomData,
            event_id_type: PhantomData,
        }
    } else {
        StreamQuery {
            filters: vec![StreamFilter::new(domain_identifiers!())],
            event_type: PhantomData,
            event_id_type: PhantomData,
        }
    }
}

/// Creates a stream query with a given event type and filter.
#[macro_export]
macro_rules! query {
    ($event_ty: ty) => {{
        $crate::stream_query::query::<_, $event_ty, $event_ty>(None)
    }};
    ($event_ty:ty; $($filter:tt)+ ) => {{
        $crate::stream_query::query::<_, $event_ty, _>(Some($crate::filter!($event_ty; $($filter)*)))
    }};
    ($origin:expr => $event_ty:ty;  $($filter:tt)+ ) => {{
        $crate::query!($event_ty; $($filter)*).change_origin($origin)
    }};
}

/// A convenient macro to get the list of event types as a list of `&'static str`.
/// It performs compile-time checks to guarantee that the specified variants exist.  
#[macro_export]
macro_rules! event_types{
    ($event_ty:ty, [$($events:ty),+]) =>{
        {
            use $crate::Event;
            const EVENTS: &[&str] = {
                const FILTER_ARG: &[&str] = &[$(stringify!($events),)+];
                   if !$crate::utils::include(<$event_ty>::SCHEMA.events, FILTER_ARG) {
                        panic!("one or more of the specified events do not exist");
                   }
                FILTER_ARG
            };
            EVENTS
        }
    };
}

/// Creates stream filters for querying event streams.
#[macro_export]
#[doc(hidden)]
macro_rules! filter {
    ($origin:expr => $event_ty:ty; $($ident:ident == $value:expr),*) =>{
        $crate::filter!($event_ty; $($ident == $value),*).change_origin($origin)
    };
    ($event_ty:ty; $($ident:ident == $value:expr),*) =>{
        {
            #[allow(dead_code)]
            {
                use $crate::Event;
                // Check if the domain identifiers exist
                const DOMAIN_IDENTIFIERS: &[&$crate::DomainIdentifierInfo] = <$event_ty>::SCHEMA.domain_identifiers;
                const DOMAIN_IDENTIFIERS_INDENTS: &[&str] = &$crate::const_slice_iter!(DOMAIN_IDENTIFIERS, const fn map(item: &$crate::DomainIdentifierInfo) -> &str {
                    item.ident.into_inner()
                });

                $(
                   const _:&[&str] = {
                       const FILTER_ARG: &[&str] = &[stringify!($ident)];
                       if !$crate::utils::include(DOMAIN_IDENTIFIERS_INDENTS, FILTER_ARG) {
                           panic!(concat!("Invalid domain filter: the domain identifier ", stringify!($ident), " does not exist"));
                       }
                       FILTER_ARG
                   };

                )*
            }
            $crate::stream_query::StreamFilter::<_, $event_ty>::new($crate::domain_identifiers!($($ident: $value.clone()),*))
        }
    };
}

/// unions two or more stream queries into a single query.
#[macro_export]
macro_rules! union {
    ($query:expr) =>{
        Into::<$crate::stream_query::StreamQuery<_, _>>::into($query).cast()
    };
    ($query1:expr, $query2: expr) =>{
        $crate::stream_query::StreamQuery::<_, _>::union(&Into::<$crate::stream_query::StreamQuery<_, _>>::into($query1),&Into::<$crate::stream_query::StreamQuery<_, _>>::into($query2))
    };
    ($query:expr, $($queries: expr),*) =>{
        {
                let mut result = $crate::union!($($queries),*);
                result = $crate::stream_query::StreamQuery::<_, _>::union(&Into::<$crate::stream_query::StreamQuery<_, _>>::into($query), &result);
                result
        }
    };
}

/// Represents a filter applied to an event stream.
///
/// A `StreamFilter` is used to define filters and constraints for querying event streams.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamFilter<ID: EventId, E: Event + Clone> {
    /// The names of the events to include in the query results.
    events: &'static [&'static str],
    /// The domain identifiers and values used to filter the events.
    identifiers: DomainIdentifierSet,
    /// The starting point of the query within the event stream.
    origin: ID,
    /// The names of the events to exclude from the query results.
    excluded_events: Option<Vec<&'static str>>,
    /// A marker indicating the event type associated with the stream filter.
    event_type: PhantomData<E>,
}

impl<ID: EventId, E: Event + Clone> StreamFilter<ID, E> {
    /// Creates a new stream filter with the specified domain identifiers.
    pub fn new(identifiers: DomainIdentifierSet) -> Self {
        Self {
            events: E::SCHEMA.events,
            identifiers,
            origin: Default::default(),
            excluded_events: None,
            event_type: PhantomData,
        }
    }

    /// Changes the origin of the stream filter.
    pub fn change_origin(self, origin: ID) -> Self {
        Self { origin, ..self }
    }

    /// Excludes the specified events from the stream filter.
    pub fn exclude_events(self, excluded_events: &'static [&'static str]) -> Self {
        Self {
            excluded_events: Some(excluded_events.to_vec()),
            ..self
        }
    }

    /// Casts the stream filter to a different event type.
    pub fn cast<O>(&self) -> StreamFilter<ID, O>
    where
        E: Event + Into<O>,
        O: Event + Clone,
    {
        StreamFilter {
            events: self.events,
            identifiers: self.identifiers.clone(),
            origin: self.origin,
            excluded_events: self.excluded_events.clone(),
            event_type: PhantomData,
        }
    }

    /// Returns the names of the events to include in the query results.
    pub fn events(&self) -> &'static [&'static str] {
        self.events
    }

    /// Returns the domain identifiers used to filter the events.
    pub fn identifiers(&self) -> &DomainIdentifierSet {
        &self.identifiers
    }

    /// Returns the starting point of the query within the event stream.
    pub fn origin(&self) -> ID {
        self.origin
    }

    /// Returns the names of the events to exclude from the query results.
    pub fn excluded_events(&self) -> Option<&Vec<&'static str>> {
        self.excluded_events.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use crate::ident;
    use crate::stream_query::StreamFilter;
    use crate::utils::tests::*;
    use crate::IdentifierValue;

    #[test]
    fn test_filter_with_no_origin_and_no_exclude_events() {
        let filter: StreamFilter<i64, _> = filter! {
            ShoppingCartEvent;
            cart_id == 42
        };

        assert_eq!(filter.identifiers.len(), 1);
        assert_eq!(
            filter.identifiers[&ident!(#cart_id)],
            IdentifierValue::i64(42)
        );
    }

    #[test]
    fn test_filter_with_origin() {
        let filter = filter! {
            10 =>
            ShoppingCartEvent;
            cart_id == 42
        };

        assert_eq!(filter.origin, 10);
    }

    #[test]
    fn test_filter_with_all_parameters() {
        let filter = filter! {
            10 =>
            ShoppingCartEvent;
            cart_id == 42
        };

        assert_eq!(filter.origin, 10);
        assert_eq!(filter.identifiers.len(), 1);
        assert_eq!(
            filter.identifiers[&ident!(#cart_id)],
            IdentifierValue::i64(42)
        );
    }
}
