mod event;

extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Derives the `Event` trait for an enum, allowing it to be used as an event in Disintegrate.
///
/// The `Event` trait is used to mark an enum as an event type in Disintegrate. By deriving this
/// trait, the enum gains the necessary functionality to be used as an event.
///
/// The `Event` trait can be customized using attributes. The `id` attribute can be used to specify
/// the domain identifier of an event, while the `group` attribute can be used to group related
/// events together.
///
/// # Example
///
/// ```rust
/// use disintegrate_macros::Event;
///
/// #[derive(Event)]
/// #[group(UserEvent, [UserRegistered, UserUpdated])]
/// #[group(OrderEvent, [OrderCreated, OrderCancelled])]
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
/// `#[group]` attribute specifies the event group name and the list of variants to include in the group, while the `#[id]` attribute is used
/// to specify the domain identifiers of each variant.
#[proc_macro_derive(Event, attributes(id, group))]
pub fn event(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    event::event_inner(&ast)
}
