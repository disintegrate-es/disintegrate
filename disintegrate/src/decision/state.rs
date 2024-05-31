//! A State contains the initial conditions that a `Decision` uses to make the changes.
use serde::Deserialize;
use serde::{de::DeserializeOwned, Serialize};

use crate::stream_query::{StreamFilter, StreamQuery};
use crate::{all_the_tuples, union, BoxDynError, StateSnapshotter};
use crate::{event::Event, PersistedEvent};
use async_trait::async_trait;
use paste::paste;
use std::error::Error as StdError;
use std::ops::Deref;

/// A mutable state that can be changed by events from the event store.
pub trait StateMutate: StateQuery {
    /// Mutates the state object based on the provided event.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to be applied to mutate the state.
    fn mutate(&mut self, event: Self::Event);
}

/// A group of states that can be queried and modified together.
///
/// The states can be mutated collectively based on an event
/// retrieved from the event store, and a unified query can be generated for all sub-states.
///
/// # Type Parameters
///
/// - `E`: The type of events that the multi-state object handles.
pub trait MultiState<E: Event + Clone> {
    /// Mutates all sub-states based on the provided event.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to be applied to mutate the sub-states.
    fn mutate_all(&mut self, event: PersistedEvent<E>);
    /// The unified query that represents the union of queries for all sub-states.
    ///
    /// This query can be used to retrieve a stream of events relevant to the entire multi-state
    /// object.
    ///
    /// # Returns
    ///
    /// A `StreamQuery` representing the combined query for all sub-states.
    fn query_all(&self) -> StreamQuery<E>;

    /// Returns the version of the multi-state.
    ///
    /// The multi-state version is determined as the maximum of the versions
    /// of its sub-states.
    ///
    /// # Returns
    ///
    /// The method returns an `i64` representing the version of the multi-state.
    fn version(&self) -> i64;
}

macro_rules! impl_multi_state {
    (
        [$($ty:ident),*], $last:ident
    ) => {
        #[allow(unused_parens)]
        impl<E, $($ty,)* $last> MultiState<E> for ($(StatePart<$ty>,)* StatePart<$last>)
        where
            E: Event + Clone,
            $($ty: StateQuery + StateMutate,)*
            $last: StateQuery + StateMutate,
            <$last as StateQuery>::Event: TryFrom<E> + Into<E>,
            $(<$ty as StateQuery>::Event: TryFrom<E> + Into<E>,)*
            <$last as StateQuery>::Event: TryFrom<E> + Into<E>,
            $(<<$ty as StateQuery>::Event as TryFrom<E>>::Error:
                StdError + 'static + Send + Sync,)*
            <<$last as StateQuery>::Event as TryFrom<E>>::Error:
                StdError + 'static + Send + Sync,
        {
            fn mutate_all(&mut self, event: PersistedEvent<E>) {
                paste! {
                    let ($([<state_ $ty:lower>],)* [<state_ $last:lower>])= self;
                    $(
                        if [<state_ $ty:lower>].matches_event(&event) {
                            [<state_ $ty:lower>].mutate_part(event.clone());
                        }
                    )*
                    if [<state_ $last:lower>].matches_event(&event) {
                         [<state_ $last:lower>].mutate_part(event.clone());
                    }
                }
            }

            fn query_all(&self) -> StreamQuery<E> {
                paste!{
                    let ($([<state_ $ty:lower>],)* [<state_ $last:lower>])= self;
                    union!($([<state_ $ty:lower>].query_part(),)* [<state_ $last:lower>].query_part())
                }
            }
            fn version(&self) -> i64 {
                paste!{
                    let ($([<state_ $ty:lower>],)* [<state_ $last:lower>])= self;
                    let version = [<state_ $last:lower>].version();
                    $(
                        let version = version.max([<state_ $ty:lower>].version());
                    )*
                    version
                }
            }
        }
    }
}

all_the_tuples!(impl_multi_state);

/// A multi-state snapshot.
///
/// A trait necessary to handle the snapshot of its sub-states' load and store.
///
/// # Type Parameters
///
/// - `T`: The type of snapshotter used for loading and storing snapshots.
/// - `E`: The type of events that the multi-state object handles.
#[async_trait]
pub trait MultiStateSnapshot<T: StateSnapshotter> {
    // Loads the state of all sub-states using the provided snapshotter
    /// and returns the version of the multi-state object.
    ///
    /// # Arguments
    ///
    /// * `backend` - The snapshotter used to load the state of sub-states.
    ///
    /// # Returns
    ///
    /// Returns the version of the multi-state object after loading its state.
    async fn load_all(&mut self, backend: &T) -> i64;
    /// Stores the snapshot of all sub-states using the provided snapshotter.
    ///
    /// # Arguments
    ///
    /// * `backend` - The snapshotter used to store the snapshot of sub-states.
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating the success or failure of the storage operation.
    async fn store_all(&self, backend: &T) -> Result<(), BoxDynError>;
}

macro_rules! impl_multi_state_snapshot {
    (
        [$($ty:ident),*], $last:ident
    ) => {
        #[async_trait]
        #[allow(unused_parens)]
        impl<B, $($ty,)* $last> MultiStateSnapshot<B> for ($(StatePart<$ty>,)* StatePart<$last>)
        where
            B: StateSnapshotter + Send + Sync,
            $($ty: StateQuery + Serialize + DeserializeOwned + 'static,)*
            $last: StateQuery + Serialize + DeserializeOwned + 'static,
        {
            async fn load_all(&mut self, backend: &B) -> i64 {
                paste! {

                    let ($([<state_ $ty:lower>],)* [<state_ $last:lower>]) = self;
                    *[<state_ $last:lower>] = backend.load_snapshot([<state_ $last:lower>].clone()).await;
                    let last_event_id = [<state_ $last:lower>].version;
                    $(
                        *[<state_ $ty:lower>] = backend.load_snapshot([<state_ $ty:lower>].clone()).await;
                        let last_event_id = last_event_id.max([<state_ $ty:lower>].version);
                    )*
                }
                last_event_id
            }

            async fn store_all(&self, backend: &B) -> Result<(), BoxDynError>{
                paste!{

                    let ($([<state_ $ty:lower>],)* [<state_ $last:lower>]) = self;
                    $(
                    backend.store_snapshot(&[<state_ $ty:lower>]).await?;
                    )*
                    backend.store_snapshot(&[<state_ $last:lower>]).await?;
                }
                Ok(())
            }
        }
    }
}
all_the_tuples!(impl_multi_state_snapshot);

/// Represents a state query used to retrieve events from the event store to build a state.
///
/// The query method returns a `StreamQuery` to be used for querying the event store.
pub trait StateQuery: Clone + Send + Sync {
    /// the unique name of the state query.
    const NAME: &'static str;
    /// The type of events queried by this state query.
    type Event: Event + Clone + Send + Sync;

    /// Returns the stream query used to retrieve relevant events for building the state.
    fn query(&self) -> StreamQuery<Self::Event>;
}

impl<S, E: Clone> From<&S> for StreamQuery<E>
where
    S: StateQuery<Event = E>,
{
    fn from(state: &S) -> Self {
        state.query()
    }
}

/// A structure representing a sub-state in a multi-state object. It encapsulates
/// the version, applied events count, and the payload of a sub-state.
///
/// # Type Parameters
///
/// - `S`: The type implementing the `StateMutate` trait, representing the sub-state.
#[derive(Clone, Serialize, Deserialize)]
pub struct StatePart<S: StateQuery> {
    /// The version of the sub-state.
    version: i64,
    /// The count of events applied to the sub-state.
    applied_events: u64,
    /// The payload of the sub-state.
    inner: S,
}

impl<S: StateQuery> StatePart<S> {
    pub fn new(version: i64, payload: S) -> Self {
        Self {
            version,
            applied_events: 0,
            inner: payload,
        }
    }
    pub fn version(&self) -> i64 {
        self.version
    }
    pub fn applied_events(&self) -> u64 {
        self.applied_events
    }
    pub fn query_part(&self) -> StreamQuery<<S as StateQuery>::Event> {
        self.inner.query().change_origin(self.version)
    }

    pub fn matches_event<U>(&self, event: &PersistedEvent<U>) -> bool
    where
        U: Event + Clone,
        <S as StateQuery>::Event: Into<U>,
    {
        matches_filter(event, self.query_part().convert().filter())
    }
    pub fn mutate_part<E>(&mut self, event: PersistedEvent<E>)
    where
        E: Event,
        S: StateMutate,
        <S as StateQuery>::Event: TryFrom<E>,
        <<S as StateQuery>::Event as TryFrom<E>>::Error: StdError + 'static + Send + Sync,
    {
        self.version = event.id;
        self.applied_events += 1;
        self.inner.mutate(event.event.try_into().unwrap());
    }
}

impl<S: StateQuery> Deref for StatePart<S> {
    type Target = S;

    fn deref(&self) -> &S {
        &self.inner
    }
}

/// Converts an state into `StatePart`s.
///
/// This trait is used to initialize a multi-state object by converting a state into a state part
/// with version and event information.
///
/// # Type Parameters
///
/// - `T`: The type of the object that can be converted into a `StatePart`.
///
/// # Associated Types
///
/// - `Target`: The resulting type after conversion, representing a `StatePart`.
pub trait IntoStatePart<T>: Sized {
    type Target;
    /// Converts the object into a `StatePart`.
    ///
    /// # Returns
    ///
    /// Returns the resulting `StatePart` after the conversion.
    fn into_state_part(self) -> Self::Target;
}

/// Extracts the state payload from a `StatePart`.
///
/// # Type Parameters
///
/// - `T`: The type representing the concrete state to be obtained from the `StatePart`.
pub trait IntoState<T>: Sized {
    /// Converts the `StatePart` into a concrete state type.
    ///
    /// # Returns
    ///
    /// Returns the concrete state obtained from the `StatePart`.
    fn into_state(self) -> T;
}

fn matches_filter<E: Event>(event: &PersistedEvent<E>, filter: &StreamFilter) -> bool {
    match filter {
        StreamFilter::Events { names } => names.contains(&event.name()),
        StreamFilter::ExcludeEvents { names } => !names.contains(&event.name()),
        StreamFilter::Eq { ident, value } => event
            .domain_identifiers()
            .get(ident)
            .map(|v| v == value)
            .unwrap_or(true),
        StreamFilter::And { l, r } => matches_filter(event, l) && matches_filter(event, r),
        StreamFilter::Or { l, r } => matches_filter(event, l) || matches_filter(event, r),
        StreamFilter::Origin { id } => event.id() > *id,
    }
}

macro_rules! impl_from_state {
    (
        [$($ty:ident),*], $last:ident
    ) => {
        #[allow(unused_parens)]
        impl<$($ty,)* $last> IntoStatePart<($($ty,)* $last)> for ($($ty,)* $last) where
            $($ty: StateQuery,)*
            $last: StateQuery,
        {
            type Target = ($(StatePart<$ty>,)* StatePart<$last>);
            paste::paste! {
                fn into_state_part(self) -> ($(StatePart<$ty>,)*StatePart<$last>){
                    let ($([<state_ $ty:lower>],)* [<state_ $last:lower>])= self;
                    ($(StatePart{ inner: [<state_ $ty:lower>], version: 0, applied_events: 0},)* StatePart{inner: [<state_ $last:lower>], version: 0, applied_events: 0})
                }
            }
        }

        #[allow(unused_parens)]
        impl<$($ty,)* $last> IntoState<($($ty,)* $last)> for ($(StatePart<$ty>,)* StatePart<$last>) where
            $($ty: StateQuery,)*
            $last: StateQuery,
        {
            paste::paste! {
                fn into_state(self) -> ($($ty,)* $last){
                    let ($([<state_ $ty:lower>],)* [<state_ $last:lower>])= self;
                    ($( [<state_ $ty:lower>].inner,)* [<state_ $last:lower>].inner)
                }
            }
        }
    }
}

all_the_tuples!(impl_from_state);

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::tests::*;

    #[test]
    fn it_mutates_all() {
        let mut state = (Cart::new("c1"), Cart::new("c2")).into_state_part();
        state.mutate_all(PersistedEvent::new(1, item_added_event("p1", "c1")));
        state.mutate_all(PersistedEvent::new(2, item_added_event("p2", "c2")));
        let (cart1, cart2) = state;
        assert_eq!(cart1.version, 1);
        assert_eq!(cart1.applied_events, 1);
        assert_eq!(cart1.into_state(), cart("c1", ["p1".to_string()]));
        assert_eq!(cart2.version, 2);
        assert_eq!(cart2.applied_events, 1);
        assert_eq!(cart2.into_state(), cart("c2", ["p2".to_string()]));
    }

    #[test]
    fn it_queries_all() {
        let cart1 = Cart::new("c1");
        let cart2 = Cart::new("c2");
        let state = (cart1.clone(), cart2.clone()).into_state_part();
        let query: StreamQuery<ShoppingCartEvent> = state.query_all();
        assert_eq!(
            query,
            union!(
                cart1.query().change_origin(0),
                cart2.query().change_origin(0)
            )
        );
    }

    #[tokio::test]
    async fn it_stores_all() {
        let multi_state = (cart("c1", []), cart("c2", [])).into_state_part();
        let mut snapshotter = MockStateSnapshotter::new();
        snapshotter
            .expect_store_snapshot()
            .once()
            .withf(|s: &StatePart<Cart>| s.inner == cart("c1", []))
            .return_once(|_| Ok(()));
        snapshotter
            .expect_store_snapshot()
            .once()
            .withf(|s: &StatePart<Cart>| s.inner == cart("c2", []))
            .return_once(|_| Ok(()));
        multi_state.store_all(&snapshotter).await.unwrap();
    }

    #[tokio::test]
    async fn it_loads_all() {
        let mut multi_state = (cart("c1", []), cart("c2", [])).into_state_part();
        let mut snapshotter = MockStateSnapshotter::new();
        snapshotter
            .expect_load_snapshot()
            .once()
            .withf(|q| q.inner == cart("c1", []))
            .returning(|_| cart("c1", ["p1".to_owned()]).into_state_part());
        snapshotter
            .expect_load_snapshot()
            .once()
            .withf(|q| q.inner == cart("c2", []))
            .returning(|_| cart("c2", ["p2".to_owned()]).into_state_part());
        multi_state.load_all(&snapshotter).await;
        let (cart1, cart2) = multi_state;
        assert_eq!(cart1.inner, cart("c1", ["p1".to_owned()]));
        assert_eq!(cart2.inner, cart("c2", ["p2".to_owned()]));
    }
}
