mod commands;
mod queries;

use crate::domain::DomainEvent;
use crate::read_model;
pub use commands::{
    CloseCourse, CreateCourse, RegisterStudent, RenameCourse, SubscribeStudent, UnsubscribeStudent,
};

#[derive(Clone)]
pub struct Application<S>
where
    S: disintegrate::StateStore<DomainEvent>,
{
    state_store: S,
    read_model: read_model::Repository,
}

impl<S> Application<S>
where
    S: disintegrate::StateStore<DomainEvent>,
{
    pub fn new(state_store: S, read_model: read_model::Repository) -> Self {
        Self {
            state_store,
            read_model,
        }
    }
}
