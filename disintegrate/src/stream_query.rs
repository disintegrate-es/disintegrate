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

use crate::identifier::Identifier;

/// Represents a query for filtering event streams.
///
/// A `StreamQuery` is used to define filters and constraints for querying event streams.
/// It allows you to specify a filter to narrow down the events of interest and an origin
/// to determine the starting point of the query within the event stream.
#[derive(Debug, Clone)]
pub struct StreamQuery<E: Clone> {
    /// An optional filter applied to the event stream. It determines which events are included
    /// in the query results based on certain criteria.
    filter: Option<StreamFilter>,
    /// The starting point of the event stream query. It represents the position in the
    /// event stream from where the query begins, and only events occurring after the origin will be considered.
    origin: i64,
    /// A marker indicating the event type associated with the stream query.
    event_type: PhantomData<E>,
}

impl<E: Clone> StreamQuery<E> {
    /// Returns the filter associated with the stream query, if any.
    pub fn filter(&self) -> Option<&StreamFilter> {
        self.filter.as_ref()
    }

    /// Returns the origin of the event stream query.
    pub fn origin(&self) -> i64 {
        self.origin
    }

    /// Changes the origin of the event stream query and returns the modified query.
    pub fn change_origin(mut self, origin: i64) -> Self {
        self.origin = origin;
        self
    }
}

/// Creates a new stream query with the given filter.
pub fn query<E: Clone>(filter: Option<StreamFilter>) -> StreamQuery<E> {
    StreamQuery {
        filter,
        origin: 0,
        event_type: PhantomData,
    }
}

/// Creates a filter that checks for equality between an identifier and a value.
pub fn eq(ident: Identifier, value: impl ToString) -> StreamFilter {
    StreamFilter::Eq {
        ident,
        value: value.to_string(),
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

/// Creates a stream query with a given event type and filter.
#[macro_export]
macro_rules! query {
    ($event_ty: ty) => {{
        $crate::query!(0; $event_ty)
    }};
    ($event_ty:ty,  $($filter:tt)+ ) => {{
        $crate::query!(0; $event_ty, $($filter)*)
    }};
    ($origin:expr; $event_ty:ty) => {{
        $crate::stream_query::query::<$event_ty>(None).change_origin($origin)
    }};
    ($origin:expr; $event_ty:ty,  $($filter:tt)+ ) => {{
        $crate::stream_query::query::<$event_ty>(Some($crate::filter!($($filter)*))).change_origin($origin)
    }};
}

#[macro_export]
macro_rules! filter {
    ($ident:ident == $value:expr) => {
       $crate::stream_query::eq($crate::ident!(#$ident), $value)
    };
    (($($h:tt)+) and ($($t:tt)+)) => {
       $crate::stream_query::and($crate::filter!($($h)+), $crate::filter!($($t)+))
    };
    (($($h:tt)+) or ($($t:tt)+)) => {
       $crate::stream_query::or($crate::filter!($($h)+), $crate::filter!($($t)+))
    };
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamFilter {
    /// Checks for equality between an identifier and a value.
    Eq {
        /// The identifier to compare.
        ident: Identifier,
        /// The value to compare against.
        value: String,
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
}

/// Represents a filter evaluator used to evaluate stream filters.
pub trait FilterEvaluator {
    /// The result type produced by evaluating a filter.
    type Result;
    /// Evaluates the given filter and returns the result.
    fn eval(&mut self, filter: &StreamFilter) -> Self::Result;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ident;

    #[test]
    fn it_can_create_stream_query_with_filter() {
        let filter = eq(ident!(#id), "123");
        let query_no_filter: StreamQuery<()> = query(None::<StreamFilter>);
        assert_eq!(query_no_filter.filter(), None);

        let query_with_filter: StreamQuery<()> = query(Some(filter.clone()));
        assert_eq!(query_with_filter.filter(), Some(&filter));
    }

    #[test]
    fn it_can_create_stream_query_macros() {
        let query_no_filter: StreamQuery<()> = query!(());
        assert_eq!(query_no_filter.filter(), None);

        let query_with_filter: StreamQuery<()> = query!((), id == "123");
        assert_eq!(query_with_filter.filter(), Some(&eq(ident!(#id), "123")));

        let query_with_origin: StreamQuery<()> = query!(42; (), id == "123");
        assert_eq!(query_with_origin.origin(), 42);
        assert_eq!(query_with_origin.filter(), Some(&eq(ident!(#id), "123")));

        let query_with_origin_no_filter: StreamQuery<()> = query!(42; ());
        assert_eq!(query_with_origin_no_filter.origin(), 42);
        assert_eq!(query_with_origin_no_filter.filter(), None);
    }

    #[test]
    fn it_can_create_filter_macros() {
        let filter = filter!(cart_id == "123");
        assert_eq!(filter, eq(ident!(#cart_id), "123"));

        let filter = filter!((cart_id == "123") and (product_id == "345"));
        assert_eq!(
            filter,
            and(eq(ident!(#cart_id), "123"), eq(ident!(#product_id), "345"))
        );

        let filter = filter!((cart_id == "123") or (product_id == "345"));
        assert_eq!(
            filter,
            or(eq(ident!(#cart_id), "123"), eq(ident!(#product_id), "345"))
        );

        let filter = filter!(((cart_id == "123") and (product_id == "345")) or
         ((cart_id == "678") and (product_id == "901")));
        assert_eq!(
            filter,
            or(
                and(eq(ident!(#cart_id), "123"), eq(ident!(#product_id), "345")),
                and(eq(ident!(#cart_id), "678"), eq(ident!(#product_id), "901"))
            )
        );
    }
}
