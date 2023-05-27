use crate::event::CartEvent;
use disintegrate::{query, State, StreamQuery};
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
    changes: Vec<CartEvent>,
}

impl State for Cart {
    type Event = CartEvent;

    fn query(&self) -> StreamQuery<Self::Event> {
        query!(Self::Event, user_id == self.user_id.clone())
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

    fn changes(&mut self) -> Vec<Self::Event> {
        std::mem::take(&mut self.changes)
    }
}

#[derive(Debug, Error)]
pub enum CartError {
    // cart errors
}

/// Implement your business logic using the state
impl Cart {
    pub fn new(user_id: &str) -> Self {
        Self {
            user_id: user_id.into(),
            items: HashSet::new(),
            changes: vec![],
        }
    }

    fn apply(&mut self, event: CartEvent) {
        self.mutate(event.clone());
        self.changes.push(event);
    }

    pub fn add_item(&mut self, item_id: &str, quantity: u32) -> Result<(), CartError> {
        // check your business constraints...

        self.apply(CartEvent::ItemAdded {
            user_id: self.user_id.clone(),
            item_id: item_id.to_string(),
            quantity,
        });
        Ok(())
    }
}
