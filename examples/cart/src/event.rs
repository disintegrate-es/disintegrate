#![allow(clippy::enum_variant_names)]
use disintegrate::Event;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Event, Serialize, Deserialize)]
#[stream(UserEvent, [UserCreated])]
#[stream(CartEvent, [ItemAdded, ItemRemoved, ItemUpdated, CouponApplied])]
#[stream(CouponEvent, [CouponEmitted, CouponApplied])]
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
    CouponEmitted {
        #[id]
        coupon_id: String,
        quantity: u32,
    },
    CouponApplied {
        #[id]
        coupon_id: String,
        #[id]
        user_id: String,
    },
}
