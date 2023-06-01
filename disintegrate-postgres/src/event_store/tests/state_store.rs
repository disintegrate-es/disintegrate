use crate::PgEventStore;

use super::*;
use disintegrate::{
    domain_identifiers, query,
    state::{Hydrated, State, StateStore},
    DomainIdentifierSet, Event, EventSchema, StreamQuery,
};
use disintegrate_serde::serde::json::Json;
use serde::{Deserialize, Serialize};

// Mock implementations for testing
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
struct SampleEvent {
    id: String,
    value: String,
}

impl Event for SampleEvent {
    const SCHEMA: EventSchema = EventSchema {
        types: &["SampleEvent"],
        domain_identifiers: &["id"],
    };

    fn name(&self) -> &'static str {
        "SampleEvent"
    }
    fn domain_identifiers(&self) -> DomainIdentifierSet {
        domain_identifiers! {id: self.id.clone()}
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct SampleState {
    id: String,
    value: String,
    _changes: Vec<SampleEvent>,
}

impl State for SampleState {
    type Event = SampleEvent;
    fn query(&self) -> StreamQuery<Self::Event> {
        query!(Self::Event, id == self.id.clone())
    }

    fn mutate(&mut self, event: Self::Event) {
        self.value = event.value;
    }

    fn changes(&mut self) -> Vec<Self::Event> {
        std::mem::take(&mut self._changes)
    }
}

#[sqlx::test]
async fn it_hydrates_state_from_empty_event_store(pool: sqlx::PgPool) {
    let event_store = PgEventStore::<SampleEvent, Json<SampleEvent>>::new(pool, Json::default())
        .await
        .unwrap();

    let default_state = SampleState {
        id: "some id".into(),
        value: "test".into(),
        _changes: vec![],
    };

    let hydrated_state = event_store.hydrate(default_state.clone()).await;
    assert!(hydrated_state.is_ok());

    let hydrated_state = hydrated_state.unwrap();
    let version = hydrated_state.version();

    assert_eq!(default_state, *hydrated_state);
    assert_eq!(version, 0);
}

#[sqlx::test]
async fn it_hydrates_state(pool: sqlx::PgPool) {
    let event_store =
        PgEventStore::<SampleEvent, Json<SampleEvent>>::new(pool.clone(), Json::default())
            .await
            .unwrap();

    insert_events(
        &pool,
        &[SampleEvent {
            id: "some id".into(),
            value: "test".into(),
        }],
    )
    .await;

    let default_state = SampleState {
        id: "some id".into(),
        value: "".into(),
        _changes: vec![],
    };

    let hydrated_state = event_store.hydrate(default_state.clone()).await;
    assert!(hydrated_state.is_ok());

    let hydrated_state = hydrated_state.unwrap();
    let version = hydrated_state.version();

    assert_eq!("test".to_string(), hydrated_state.value);
    assert_eq!("some id".to_string(), hydrated_state.id);
    assert_eq!(version, 1);
}

#[sqlx::test]
async fn it_saves_state(pool: sqlx::PgPool) {
    let event_store = PgEventStore::<SampleEvent, Json<SampleEvent>>::new(pool, Json::default())
        .await
        .unwrap();

    let default_state = SampleState {
        id: "some id".into(),
        value: "test".into(),
        _changes: vec![SampleEvent {
            id: "some id".into(),
            value: "test".into(),
        }],
    };
    let mut hydrated_state = Hydrated::new(default_state, 1);

    let save_result = event_store.save(&mut hydrated_state).await;
    assert!(save_result.is_ok());
    assert_eq!(hydrated_state._changes.len(), 0);
}
