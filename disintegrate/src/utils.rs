#![doc(hidden)]

#[macro_export]
#[doc(hidden)]
macro_rules! const_slice_unique {
    ($a:expr) => {
        &{
            const A: &[&str] = $crate::const_slice_sort!($a);
            const LEN: usize = A.len() - disintegrate::utils::count_dup(A);

            let mut out: [_; LEN] = if LEN == 0 {
                unsafe { std::mem::transmute([0u8; std::mem::size_of::<&str>() * LEN]) }
            } else {
                [A[0]; LEN]
            };

            let mut r: usize = 1;
            let mut w: usize = 1;
            while r < A.len() {
                if !disintegrate::utils::eq(A[r], out[w - 1]) {
                    out[w] = A[r];
                    w += 1;
                }
                r += 1;
            }
            out
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! const_slices_concat {
    ($a:expr, $b:expr) => {
        &{
            const A: &[&str] = $a;
            const B: &[&str] = $b;
            let mut out: [_; { A.len() + B.len() }] = if A.len() == 0 && B.len() == 0 {
                unsafe {
                    std::mem::transmute([0u8; std::mem::size_of::<&str>() * (A.len() + B.len())])
                }
            } else if A.len() == 0 {
                [B[0]; { A.len() + B.len() }]
            } else {
                [A[0]; { A.len() + B.len() }]
            };
            let mut i = 0;
            while i < A.len() {
                out[i] = A[i];
                i += 1;
            }
            i = 0;
            while i < B.len() {
                out[i + A.len()] = B[i];
                i += 1;
            }
            out
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! const_slice_sort {
    ($a:expr) => {
        &{
            const A: &[&str] = $a;
            let mut out: [_; A.len()] = if A.len() == 0 {
                unsafe { std::mem::transmute([0u8; std::mem::size_of::<&str>() * A.len()]) }
            } else {
                [A[0]; A.len()]
            };

            let mut i = 1;
            while i < A.len() {
                out[i] = A[i];
                let mut j = i;
                while j > 0 && disintegrate::utils::compare(out[j], out[j - 1]) == -1 {
                    //swap
                    let tmp = out[j];
                    out[j] = out[j - 1];
                    out[j - 1] = tmp;

                    j -= 1;
                }
                i += 1;
            }
            out
        }
    };
}

pub const fn include(a: &[&str], b: &[&str]) -> bool {
    let mut i = 0;
    let mut j = 0;

    while i < a.len() && j < b.len() {
        if eq(a[i], b[j]) {
            j += 1;
            i = 0;
        } else {
            i += 1;
        }
    }

    j == b.len()
}

pub const fn count_dup(slice: &[&str]) -> usize {
    let mut count = 0;
    let mut i = 0;
    while i + 1 < slice.len() {
        if eq(slice[i], slice[i + 1]) {
            count += 1;
        }
        i += 1;
    }

    count
}

pub const fn compare(lhs: &str, rhs: &str) -> i8 {
    let lhs = lhs.as_bytes();
    let rhs = rhs.as_bytes();
    let lhs_len = lhs.len();
    let rhs_len = rhs.len();
    let min_len = if lhs_len < rhs_len { lhs_len } else { rhs_len };

    let mut i = 0;
    while i < min_len {
        if lhs[i] < rhs[i] {
            return -1;
        }
        if lhs[i] > rhs[i] {
            return 1;
        }
        i += 1;
    }

    if lhs_len < rhs_len {
        -1
    } else if lhs_len > rhs_len {
        1
    } else {
        0
    }
}

pub const fn eq(lhs: &str, rhs: &str) -> bool {
    let lhs = lhs.as_bytes();
    let rhs = rhs.as_bytes();
    let lhs_len = lhs.len();
    let rhs_len = rhs.len();

    if lhs_len != rhs_len {
        return false;
    }

    let mut i = 0;
    while i < lhs_len {
        if lhs[i] != rhs[i] {
            return false;
        }
        i += 1;
    }

    true
}

#[cfg(test)]
pub mod tests {
    use async_trait::async_trait;
    use futures::{
        stream::{self, BoxStream},
        StreamExt,
    };
    use mockall::mock;
    use serde::{de::DeserializeOwned, Deserialize, Serialize};
    use std::{error::Error as StdError, fmt};

    use crate::{
        domain_identifiers, query, BoxDynError, Decision, DomainIdentifierSet, Event, EventSchema,
        EventStore, PersistedEvent, StateMutate, StatePart, StateQuery, StateSnapshotter,
        StreamQuery,
    };

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "event_type", rename_all = "snake_case")]
    pub enum ShoppingCartEvent {
        ItemAdded { item_id: String, cart_id: String },
        ItemRemoved { item_id: String, cart_id: String },
    }

    pub fn item_added_event(item_id: &str, cart_id: &str) -> ShoppingCartEvent {
        ShoppingCartEvent::ItemAdded {
            item_id: item_id.to_string(),
            cart_id: cart_id.to_string(),
        }
    }

    pub fn item_removed_event(item_id: &str, cart_id: &str) -> ShoppingCartEvent {
        ShoppingCartEvent::ItemRemoved {
            item_id: item_id.to_string(),
            cart_id: cart_id.to_string(),
        }
    }

    pub fn event_stream<E: Event + Clone>(
        events: impl Into<Vec<E>>,
    ) -> Vec<Result<PersistedEvent<E>, Error>> {
        events
            .into()
            .into_iter()
            .enumerate()
            .map(|(id, event)| Ok(PersistedEvent::new((id + 1) as i64, event)))
            .collect()
    }

    impl Event for ShoppingCartEvent {
        const SCHEMA: EventSchema = EventSchema {
            types: &["ItemAdded", "ItemRemoved"],
            domain_identifiers: &["cart_id", "item_id"],
        };
        fn name(&self) -> &'static str {
            match self {
                ShoppingCartEvent::ItemAdded { .. } => "ItemAdded",
                ShoppingCartEvent::ItemRemoved { .. } => "ItemRemoved",
            }
        }
        fn domain_identifiers(&self) -> DomainIdentifierSet {
            match self {
                ShoppingCartEvent::ItemAdded {
                    item_id, cart_id, ..
                } => domain_identifiers! {item_id: item_id, cart_id: cart_id},
                ShoppingCartEvent::ItemRemoved {
                    item_id, cart_id, ..
                } => domain_identifiers! {item_id: item_id, cart_id: cart_id},
            }
        }
    }

    #[derive(Clone)]
    pub struct MockEventStore<D> {
        pub database: D,
    }
    impl<D> MockEventStore<D> {
        pub fn new(database: D) -> Self {
            Self { database }
        }
    }

    #[derive(Debug)]
    pub struct Error;
    impl StdError for Error {}
    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "test error")
        }
    }

    pub trait Database {
        fn stream<QE: Event + Clone + 'static + Send + Sync>(
            &self,
            query: &StreamQuery<QE>,
        ) -> Vec<Result<PersistedEvent<QE>, Error>>;

        fn append<QE: Event + Clone + 'static + Send + Sync>(
            &self,
            events: Vec<ShoppingCartEvent>,
            query: StreamQuery<QE>,
            last_event_id: i64,
        ) -> Vec<PersistedEvent<ShoppingCartEvent>>;
    }

    mock! {
        pub Database {}
        impl Database for Database {
        fn stream<QE: Event + Clone + 'static + Send + Sync>(
            &self,
            query: &StreamQuery<QE>,
        ) -> Vec<Result<PersistedEvent<QE>, Error>>;

        fn append<QE: Event + Clone + 'static + Send + Sync>(
            &self,
            events: Vec<ShoppingCartEvent>,
            query: StreamQuery<QE>,
            last_event_id: i64,
        ) -> Vec<PersistedEvent<ShoppingCartEvent>>;
        }
        impl Clone for Database {
            fn clone(&self) -> Self;
        }
    }

    #[async_trait]
    impl<D: Database + Sync> EventStore<ShoppingCartEvent> for MockEventStore<D> {
        type Error = Error;

        fn stream<'a, QE>(
            &'a self,
            query: &'a StreamQuery<QE>,
        ) -> BoxStream<Result<PersistedEvent<QE>, Self::Error>>
        where
            QE: TryFrom<ShoppingCartEvent> + Event + 'static + Clone + Send + Sync,
            <QE as TryFrom<ShoppingCartEvent>>::Error: StdError + 'static + Send + Sync,
        {
            stream::iter(self.database.stream(query)).boxed()
        }

        async fn append<QE>(
            &self,
            events: Vec<ShoppingCartEvent>,
            query: StreamQuery<QE>,
            last_event_id: i64,
        ) -> Result<Vec<PersistedEvent<ShoppingCartEvent>>, Self::Error>
        where
            QE: Event + 'static + Clone + Send + Sync,
        {
            Ok(self.database.append(events, query, last_event_id))
        }
    }

    #[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
    pub struct Cart {
        pub cart_id: String,
        pub items: Vec<String>,
    }

    impl Cart {
        pub fn new(cart_id: &str) -> Self {
            Self {
                cart_id: cart_id.into(),
                ..Default::default()
            }
        }
    }

    pub fn cart<const N: usize>(cart_id: &str, items: [String; N]) -> Cart {
        Cart {
            cart_id: cart_id.to_string(),
            items: Vec::from(items),
        }
    }

    impl StateQuery for Cart {
        const NAME: &'static str = "Cart";
        type Event = ShoppingCartEvent;

        fn query(&self) -> StreamQuery<Self::Event> {
            query!(ShoppingCartEvent, cart_id == self.cart_id.clone())
        }
    }

    impl StateMutate for Cart {
        fn mutate(&mut self, event: Self::Event) {
            match event {
                ShoppingCartEvent::ItemAdded { item_id, .. } => {
                    self.items.push(item_id);
                }
                ShoppingCartEvent::ItemRemoved { item_id, .. } => {
                    let index = self.items.iter().position(|i| i == &item_id).unwrap();
                    self.items.remove(index);
                }
            }
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    pub struct CartError(pub String);
    impl StdError for CartError {}

    impl fmt::Display for CartError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    mock! {
            pub Decision{}
            impl Decision for Decision {
                type Event = ShoppingCartEvent;
                type StateQuery = Cart;
                type Error = CartError;

            fn state_query(&self) -> <Self as Decision>::StateQuery;
            fn validation_query(&self) -> Option<StreamQuery<ShoppingCartEvent>>;
            fn process(&self, _state: &<Self as Decision>::StateQuery) -> Result<Vec<<Self as Decision>::Event>, <Self as Decision>::Error>;
        }
    }

    mock! {
            pub StateSnapshotter{}
            #[async_trait]
            impl StateSnapshotter for StateSnapshotter {
                async fn load_snapshot<S>(&self, default: StatePart<S>) -> StatePart<S>
                where
                    S: Send + Sync + DeserializeOwned + StateQuery + 'static;
                async fn store_snapshot<S>(&self, state: &StatePart<S>) -> Result<(), BoxDynError>
                where
                    S: Send + Sync + Serialize + StateQuery + 'static;
            }
            impl Clone for StateSnapshotter {
                fn clone(&self) -> Self;
            }
    }
}
