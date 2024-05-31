mod event;
mod state_query;
mod symbol;

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};

use syn::{parse_macro_input, DeriveInput};

/// Derives the `Event` trait for an enum, allowing it to be used as an event in Disintegrate.
///
/// The `Event` trait is used to mark an enum as an event type in Disintegrate. By deriving this
/// trait, the enum gains the necessary functionality to be used as an event.
///
/// The `Event` trait can be customized using attributes. The `id` attribute can be used to specify
/// the domain identifier of an event, while the `stream` attribute can be used to stream related
/// events together.
///
/// # Example
///
/// ```rust
/// use disintegrate::Event;
///
/// #[derive(Event)]
/// #[stream(UserEvent, [UserRegistered, UserUpdated])]
/// #[stream(OrderEvent, [OrderCreated, OrderCancelled])]
/// enum DomainEvent{
///     UserCreated {
///         #[id]
///         user_id: String,
///         name: String,
///         email: String,
///     },
///     UserUpdated {
///         #[id]
///         user_id: String,
///         email: String,
///     },
///     OrderCreated {
///         #[id]
///         order_id: String,
///         amount: u32
///     },
///     OrderCancelled {
///         #[id]
///         order_id: String
///     },
/// }
/// ```
///
/// In this example, the `OrderEvent` enum is marked as an event by deriving the `Event` trait. The
/// `#[stream]` attribute specifies the event stream name and the list of variants to include in the stream, while the `#[id]` attribute is used
/// to specify the domain identifiers of each variant.
#[proc_macro_derive(Event, attributes(stream, id))]
pub fn event(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    event::event_inner(&ast)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derives the `StateQuery` trait for a struct, enabling its use as a state query in Disintegrate.
///
/// The `state_query` attribute is mandatory and must include the event type associated with the state query.
/// Additionally, the `id` attribute can be utilized to specify the domain identifier of a state query. It is employed
/// in generating a stream query for the state, querying for the event specified in the `state_query`
/// attribute, with the identifiers marked in `or`.
///
/// It is also possible to rename a state using the `rename` argument in the `state_query` attribute. This feature is beneficial
/// for snapshotting, and the name specified in `rename` is used to identify the snapshot.
///
/// # Example
///
/// ```rust
/// # use disintegrate::Event;
/// # #[derive(Event, Clone)]
/// # enum DomainEvent{
/// #    UserCreated {
/// #         #[id]
/// #         user_id: String,
/// #     },
/// # }
///
/// use disintegrate::StateQuery;
///
/// #[derive(StateQuery, Clone)]
/// #[state_query(DomainEvent, rename = "user-query-v1")] // Rename the state for snapshotting
/// struct UserStateQuery {
///     #[id]
///     user_id: String,
///     name: String
/// }
/// ```
///
/// In this example, the `UserStateQuery` struct is annotated with the `StateQuery` derive,
/// indicating its role as a state query. The `#[state_query]` attribute specifies the associated event type,
/// and the `#[id]` attribute is used to define the domain identifiers. The `#[state_query]` attribute with `rename`
/// renames the state to 'user-query-v1' for snapshotting purposes.
#[proc_macro_derive(StateQuery, attributes(state_query, id))]
pub fn state_query(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    state_query::state_query_inner(&ast)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn reserved_identifier_names(identifiers_fields: &[&Ident]) -> Option<TokenStream2> {
    const RESERVED_NAMES: &[&str] = &["event_id", "payload", "event_type", "inserted_at"];

    identifiers_fields
        .iter()
        .find(|id| RESERVED_NAMES.contains(&id.to_string().as_str()))
        .map(|id| {
            syn::Error::new(
                id.span(),
                "Reserved domain identifier name. Please use a different name",
            )
            .to_compile_error()
        })
}
