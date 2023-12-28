mod commands;
mod queries;

use crate::{domain::DomainEvent, proto, read_model};
use disintegrate::serde::prost::Prost;
use disintegrate_postgres::{PgDecisionMaker, WithPgSnapshot};

pub type DecisionMaker =
    PgDecisionMaker<DomainEvent, Prost<DomainEvent, proto::Event>, WithPgSnapshot>;

#[derive(Clone)]
pub struct Application {
    decision_maker: DecisionMaker,
    read_model: read_model::Repository,
}

impl Application {
    pub fn new(decision_maker: DecisionMaker, read_model: read_model::Repository) -> Self {
        Self {
            decision_maker,
            read_model,
        }
    }
}
