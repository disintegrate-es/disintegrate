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

use crate::{identifier::IntoIdentifierValue, Event, Identifier, IdentifierValue};

/// Represents a query for filtering event streams.
///
/// A `StreamQuery` is used to define filters and constraints for querying event streams.
/// It allows you to specify a filter to narrow down the events of interest and an origin
/// to determine the starting point of the query within the event stream.
#[derive(Debug, Clone)]
pub struct StreamQuery<E: Clone> {
    /// An optional filter applied to the event stream. It determines which events are included
    /// in the query results based on certain criteria.
    filter: StreamFilter,
    /// A marker indicating the event type associated with the stream query.
    event_type: PhantomData<E>,
}

impl<E: Clone> StreamQuery<E> {
    /// Returns the filter associated with the stream query, if any.
    pub fn filter(&self) -> &StreamFilter {
        &self.filter
    }

    /// Changes the origin of the event stream query and returns the modified query.
    pub fn change_origin(mut self, id: i64) -> Self {
        self.filter = and(origin(id), self.filter);
        self
    }

    /// Sets the list of event types that will be excluded from the query result.
    pub fn exclude_events(mut self, types: &'static [&'static str]) -> Self {
        self.filter = and(exclude_events(types), self.filter);
        self
    }

    pub fn convert<U>(&self) -> StreamQuery<U>
    where
        E: Event + Into<U>,
        U: Event + Clone,
    {
        StreamQuery {
            filter: self.filter.clone(),
            event_type: PhantomData,
        }
    }

    pub fn union<U, O>(&self, other: &StreamQuery<O>) -> StreamQuery<U>
    where
        E: Event + Into<U>,
        U: Event + Clone,
        O: Event + Into<U> + Clone,
    {
        StreamQuery {
            filter: or(self.filter.clone(), other.filter.clone()),
            event_type: PhantomData,
        }
    }
}

impl<E: Clone> PartialEq for StreamQuery<E> {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
    }
}

impl<E: Clone> Eq for StreamQuery<E> {}

/// Creates a new stream query with the given filter.
pub fn query<E: Event + Clone>(filter: Option<StreamFilter>) -> StreamQuery<E> {
    let filter = if let Some(filter) = filter {
        and(events(E::SCHEMA.types), filter)
    } else {
        events(E::SCHEMA.types)
    };
    StreamQuery {
        filter,
        event_type: PhantomData,
    }
}

/// Creates a new filter that allows you to specify a subset of events to pass through.
pub fn events(names: &'static [&'static str]) -> StreamFilter {
    StreamFilter::Events { names }
}

/// Creates a new filter that allows you to specify a subset of events to filter out.
pub fn exclude_events(names: &'static [&'static str]) -> StreamFilter {
    StreamFilter::ExcludeEvents { names }
}

/// Creates a filter that checks for equality between an identifier and a value.
pub fn eq(ident: Identifier, value: impl IntoIdentifierValue) -> StreamFilter {
    StreamFilter::Eq {
        ident,
        value: value.into_identifier_value(),
    }
}

/// Creates a filter that performs a logical AND operation between two filters.
pub fn and(l: StreamFilter, r: StreamFilter) -> StreamFilter {
    StreamFilter::And {
        l: Box::new(l),
        r: Box::new(r),
    }
}

/// Creates a filter that performs a logical OR operation between two filters.
pub fn or(l: StreamFilter, r: StreamFilter) -> StreamFilter {
    StreamFilter::Or {
        l: Box::new(l),
        r: Box::new(r),
    }
}

/// Creates a origin filter that requires events after a specified id.
pub fn origin(id: i64) -> StreamFilter {
    StreamFilter::Origin { id }
}

/// Creates a stream query with a given event type and filter.
#[macro_export]
macro_rules! query {
    ($event_ty: ty) => {{
        $crate::stream_query::query::<$event_ty>(None)
    }};
    ($event_ty:ty,  $($filter:tt)+ ) => {{
        $crate::stream_query::query::<$event_ty>(Some($crate::filter!($event_ty, $($filter)*)))
    }};
    ($origin:expr; $event_ty:ty) => {{
        $crate::stream_query::query::<$event_ty>(Some($crate::stream_query::origin($origin)))
    }};
    ($origin:expr; $event_ty:ty,  $($filter:tt)+ ) => {{
        $crate::stream_query::query::<$event_ty>(Some($crate::filter!($event_ty, $($filter)*))).change_origin($origin)
    }};
}

/// A convenient macro to get the list of event types as a list of `&'static str`.
/// It performs compile-time checks to guarantee that the specified variants exist.  
#[macro_export]
macro_rules! event_types{
    ($event_ty:ty, [$($events:ty),+]) =>{
        {
            use $crate::Event;
            const TYPES: &[&str] = {
                const FILTER_ARG: &[&str] = &[$(stringify!($events),)+];
                   if !$crate::utils::include(<$event_ty>::SCHEMA.types, FILTER_ARG) {
                    panic!("one or more of the specified events do not exist");
                }
                FILTER_ARG
            };
            TYPES
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! filter {
    ($event_ty:ty, events[$($events:ty),+]) =>{
            $crate::stream_query::events($crate::event_types!($event_ty, [$($events),+]))
    };
    ($event_ty:ty, exclude_events[$($events:ty),+]) =>{

            $crate::stream_query::exclude_events($crate::event_types!($event_ty, [$($events),+]))

    };
    ($event_ty:ty, $ident:ident == $value:expr) => {
        {
            use $crate::Event;
            const _: &[&str] = {
                const FILTER_ARG: &[&str] = &[stringify!($ident)];
                const DOMAIN_IDENTIFIERS: &[&$crate::DomainIdentifierInfo] = <$event_ty>::SCHEMA.domain_identifiers;
                const DOMAIN_IDENTIFIERS_IDENTS: &[&str] = &$crate::const_slice_iter!(DOMAIN_IDENTIFIERS, const fn map(item: &$crate::DomainIdentifierInfo) -> &str {
                    item.ident.into_inner()
                });

                if !$crate::utils::include(DOMAIN_IDENTIFIERS_IDENTS, FILTER_ARG) {
                    panic!(concat!("Invalid eq filter: the domain identifier ", stringify!($ident), " does not exist"));
                }
                FILTER_ARG
            };
            $crate::stream_query::eq($crate::ident!(#$ident), $value.clone())
        }
    };
    ($event_ty:ty, ($($h:tt)+) and ($($t:tt)+)) => {
       $crate::stream_query::and($crate::filter!($event_ty, $($h)+), $crate::filter!($event_ty, $($t)+))
    };
    ($event_ty:ty, ($($h:tt)+) or ($($t:tt)+)) => {
       $crate::stream_query::or($crate::filter!($event_ty, $($h)+), $crate::filter!($event_ty, $($t)+))
    };
}

#[macro_export]
macro_rules! union {
    ($query:expr) =>{
        Into::<$crate::stream_query::StreamQuery<_>>::into($query).convert()
    };
    ($query1:expr, $query2: expr) =>{
        $crate::stream_query::StreamQuery::<_>::union(&Into::<$crate::stream_query::StreamQuery<_>>::into($query1),&Into::<$crate::stream_query::StreamQuery<_>>::into($query2))
    };
    ($query:expr, $($queries: expr),*) =>{
        {
                let mut result = $crate::union!($($queries),*);
                result = $crate::stream_query::StreamQuery::<_>::union(&Into::<$crate::stream_query::StreamQuery<_>>::into($query), &result);
                result
        }
    };
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamFilter {
    /// Includes only the specified events.
    Events {
        /// The list of events to include.
        names: &'static [&'static str],
    },
    /// Checks for equality between an identifier and a value.
    Eq {
        /// The identifier to compare.
        ident: Identifier,
        /// The value to compare against.
        value: IdentifierValue,
    },
    /// Performs a logical AND operation between two filters.
    And {
        /// The left operand of the AND operation.
        l: Box<StreamFilter>,
        /// The right operand of the AND operation.
        r: Box<StreamFilter>,
    },
    /// Performs a logical OR operation between two filters.
    Or {
        /// The left operand of the OR operation.
        l: Box<StreamFilter>,
        /// The right operand of the OR operation.
        r: Box<StreamFilter>,
    },

    /// Requires that the event id is greater than the specified origin.
    Origin { id: i64 },

    /// Exclude the specified events.
    ExcludeEvents {
        /// The list of events to exclude.
        names: &'static [&'static str],
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ident;
    use crate::utils::tests::*;

    #[test]
    fn it_can_create_stream_query_with_filter() {
        let filter = eq(ident!(#cart_id), "123");
        let query_with_filter: StreamQuery<ShoppingCartEvent> = query(Some(filter));
        assert_eq!(
            query_with_filter.filter(),
            &and(
                events(ShoppingCartEvent::SCHEMA.types),
                eq(ident!(#cart_id), "123")
            )
        );
    }

    #[test]
    fn it_can_create_stream_query_macros() {
        let query_with_filter: StreamQuery<ShoppingCartEvent> =
            query!(ShoppingCartEvent, cart_id == "123");
        assert_eq!(
            query_with_filter.filter(),
            &and(
                events(ShoppingCartEvent::SCHEMA.types),
                eq(ident!(#cart_id), "123")
            )
        );

        let query_with_origin: StreamQuery<ShoppingCartEvent> =
            query!(42; ShoppingCartEvent, cart_id == "123");
        assert_eq!(
            query_with_origin.filter(),
            &and(
                origin(42),
                and(
                    events(ShoppingCartEvent::SCHEMA.types),
                    eq(ident!(#cart_id), "123")
                )
            )
        );
    }

    #[test]
    fn it_can_create_filter_macros() {
        let filter = filter!(ShoppingCartEvent, cart_id == "123");
        assert_eq!(filter, eq(ident!(#cart_id), "123"));

        let filter = filter!(ShoppingCartEvent, (cart_id == "123") and (item_id == "345"));
        assert_eq!(
            filter,
            and(eq(ident!(#cart_id), "123"), eq(ident!(#item_id), "345"))
        );

        let filter = filter!(ShoppingCartEvent, (cart_id == "123") or (item_id == "345"));
        assert_eq!(
            filter,
            or(eq(ident!(#cart_id), "123"), eq(ident!(#item_id), "345"))
        );

        let filter = filter!(ShoppingCartEvent, ((cart_id == "123") and (item_id == "345")) or
         ((cart_id == "678") and (item_id == "901")));
        assert_eq!(
            filter,
            or(
                and(eq(ident!(#cart_id), "123"), eq(ident!(#item_id), "345")),
                and(eq(ident!(#cart_id), "678"), eq(ident!(#item_id), "901"))
            )
        );

        let filter = filter!(ShoppingCartEvent, (cart_id == "123") and ((item_id == "345") and (events[ItemAdded, ItemRemoved])));
        assert_eq!(
            filter,
            and(
                eq(ident!(#cart_id), "123"),
                and(
                    eq(ident!(#item_id), "345"),
                    events(ShoppingCartEvent::SCHEMA.types),
                )
            )
        );

        let filter = filter!(ShoppingCartEvent, (cart_id == "123") and ((item_id == "345") and (exclude_events[ItemAdded, ItemRemoved])));
        assert_eq!(
            filter,
            and(
                eq(ident!(#cart_id), "123"),
                and(
                    eq(ident!(#item_id), "345"),
                    exclude_events(&["ItemAdded", "ItemRemoved"])
                )
            )
        );
    }

    #[test]
    fn it_exclude_events() {
        let query_with_exceptions: StreamQuery<ShoppingCartEvent> =
            query(None).exclude_events(event_types!(ShoppingCartEvent, [ItemAdded]));
        assert_eq!(
            query_with_exceptions.filter(),
            &and(
                exclude_events(&["ItemAdded"]),
                events(ShoppingCartEvent::SCHEMA.types)
            )
        );
    }

    #[test]
    fn it_unions_two_queries() {
        let query1: StreamQuery<ShoppingCartEvent> = query(Some(origin(0)));
        let query2: StreamQuery<ShoppingCartEvent> = query(Some(origin(1)));
        let union: StreamQuery<ShoppingCartEvent> = query1.union(&query2);
        assert_eq!(
            union.filter(),
            &or(
                and(events(ShoppingCartEvent::SCHEMA.types), origin(0)),
                and(events(ShoppingCartEvent::SCHEMA.types), origin(1))
            )
        );
    }
}
