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
        domain_identifiers! {id: self.id}
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct SampleState {
    id: String,
    value: String,
}

impl State for SampleState {
    type Event = SampleEvent;
    fn query(&self) -> StreamQuery<Self::Event> {
        query!(SampleEvent, id == self.id)
    }

    fn mutate(&mut self, event: Self::Event) {
        self.value = event.value;
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
    };
    let hydrated_state = Hydrated::new(default_state, 1);

    let change = SampleEvent {
        id: "some id".into(),
        value: "test".into(),
    };
    let save_result = event_store
        .save(&hydrated_state, vec![change.clone()])
        .await
        .unwrap();
    assert_eq!(change, save_result.first().unwrap().clone().into_inner());
}
