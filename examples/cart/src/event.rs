#![allow(clippy::enum_variant_names)]
use disintegrate::macros::Event;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Event, Serialize, Deserialize)]
#[group(UserEvent, [UserCreated])]
#[group(CartEvent, [ItemAdded, ItemRemoved, ItemUpdated])]
pub enum DomainEvent {
    UserCreated {
        #[id]
        user_id: String,
        name: String,
    },
    ItemAdded {
        #[id]
        user_id: String,
        #[id]
        item_id: String,
        quantity: u32,
    },
    ItemRemoved {
        #[id]
        user_id: String,
        #[id]
        item_id: String,
    },
    ItemUpdated {
        #[id]
        user_id: String,
        #[id]
        item_id: String,
        new_quantity: u32,
    },
}
