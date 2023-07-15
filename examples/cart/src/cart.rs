use crate::event::CartEvent;
use disintegrate::{query, Decision, State, StreamQuery};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Item {
    id: String,
    quantity: u32,
}

impl Item {
    fn new(id: String, quantity: u32) -> Self {
        Item { id, quantity }
    }
}

#[derive(Default, Clone)]
pub struct Cart {
    user_id: String,
    items: HashSet<Item>,
}

impl Cart {
    pub fn new(user_id: &str) -> Self {
        Self {
            user_id: user_id.into(),
            ..Default::default()
        }
    }
}

impl State for Cart {
    type Event = CartEvent;

    fn query(&self) -> StreamQuery<Self::Event> {
        query!(CartEvent, user_id == self.user_id)
    }

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
        }
    }
}

#[derive(Debug, Error)]
pub enum CartError {
    // cart errors
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
    type Event = CartEvent;
    type State = Cart;
    type Error = CartError;

    fn default_state(&self) -> Self::State {
        Cart::new(&self.user_id)
    }

    fn validation_query(&self) -> Option<StreamQuery<CartEvent>> {
        None
    }

    fn process(&self, _state: &Self::State) -> Result<Vec<Self::Event>, Self::Error> {
        // check your business constraints...
        Ok(vec![CartEvent::ItemAdded {
            user_id: self.user_id.clone(),
            item_id: self.item_id.to_string(),
            quantity: self.quantity,
        }])
    }
}
