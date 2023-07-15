mod commands;
mod queries;

use crate::{domain::DomainEvent, read_model};
use disintegrate::{DecisionMaker, EventStore};

#[derive(Clone)]
pub struct Application<ES> {
    decision_maker: DecisionMaker<ES>,
    read_model: read_model::Repository,
}

impl<ES> Application<ES>
where
    ES: EventStore<DomainEvent>,
{
    pub fn new(event_store: ES, read_model: read_model::Repository) -> Self {
        Self {
            decision_maker: DecisionMaker::new(event_store),
            read_model,
        }
    }
}
