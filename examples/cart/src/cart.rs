use crate::event::{CartEvent, CouponEvent, DomainEvent};
use disintegrate::StateQuery;
use disintegrate::{Decision, StateMutate};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Item {
    id: String,
    quantity: u32,
}

impl Item {
    fn new(id: String, quantity: u32) -> Self {
        Item { id, quantity }
    }
}

#[derive(Default, StateQuery, Clone, Serialize, Deserialize)]
#[state_query(CartEvent)]
pub struct Cart {
    #[id]
    user_id: String,
    items: HashSet<Item>,
    applied_coupon: Option<String>,
}

impl Cart {
    pub fn new(user_id: &str) -> Self {
        Self {
            user_id: user_id.into(),
            ..Default::default()
        }
    }
}

impl StateMutate for Cart {
    fn mutate(&mut self, event: Self::Event) {
        match event {
            CartEvent::ItemAdded {
                item_id, quantity, ..
            } => {
                self.items.insert(Item::new(item_id, quantity));
            }
            CartEvent::ItemRemoved { item_id, .. } => {
                self.items.retain(|item| item.id != *item_id);
            }
            CartEvent::ItemUpdated {
                item_id,
                new_quantity,
                ..
            } => {
                self.items.replace(Item::new(item_id, new_quantity));
            }
            CartEvent::CouponApplied { coupon_id, .. } => {
                self.applied_coupon = Some(coupon_id);
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum CartError {
    // cart errors
    #[error("coupon already applied")]
    CouponAlreadyApplied,
    #[error("coupon not available")]
    CouponNotAvailable,
}

pub struct AddItem {
    user_id: String,
    item_id: String,
    quantity: u32,
}

impl AddItem {
    pub fn new(user_id: String, item_id: String, quantity: u32) -> Self {
        Self {
            user_id,
            item_id,
            quantity,
        }
    }
}

/// Implement your business logic
impl Decision for AddItem {
    type Event = DomainEvent;
    type StateQuery = Cart;
    type Error = CartError;

    fn state_query(&self) -> Self::StateQuery {
        Cart::new(&self.user_id)
    }

    fn process(&self, _state: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        // check your business constraints...
        Ok(vec![DomainEvent::ItemAdded {
            user_id: self.user_id.clone(),
            item_id: self.item_id.to_string(),
            quantity: self.quantity,
        }])
    }
}

#[derive(Default, StateQuery, Clone, Serialize, Deserialize)]
#[state_query(CouponEvent)]
pub struct Coupon {
    #[id]
    coupon_id: String,
    quantity: u32,
}

impl Coupon {
    pub fn new(coupon_id: &str) -> Self {
        Self {
            coupon_id: coupon_id.to_string(),
            ..Default::default()
        }
    }
}

impl StateMutate for Coupon {
    fn mutate(&mut self, event: Self::Event) {
        match event {
            CouponEvent::CouponEmitted { quantity, .. } => self.quantity += quantity,
            CouponEvent::CouponApplied { .. } => self.quantity -= 1,
        }
    }
}

pub struct ApplyCoupon {
    user_id: String,
    coupon_id: String,
}

impl ApplyCoupon {
    #[allow(dead_code)]
    pub fn new(user_id: String, coupon_id: String) -> Self {
        Self { user_id, coupon_id }
    }
}

/// Implement your business logic
impl Decision for ApplyCoupon {
    type Event = DomainEvent;
    type StateQuery = (Cart, Coupon);
    type Error = CartError;

    fn state_query(&self) -> Self::StateQuery {
        (Cart::new(&self.user_id), Coupon::new(&self.coupon_id))
    }

    fn process(&self, (cart, coupon): &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        // check your business constraints...
        if cart.applied_coupon.is_some() {
            return Err(CartError::CouponAlreadyApplied);
        }
        if coupon.quantity == 0 {
            return Err(CartError::CouponNotAvailable);
        }
        Ok(vec![DomainEvent::CouponApplied {
            coupon_id: self.coupon_id.clone(),
            user_id: self.user_id.clone(),
        }])
    }
}
