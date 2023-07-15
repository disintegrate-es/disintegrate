//! Utility for testing a Decision implementation
//!
//! The test harness allows you to set up a history of events, perform the given decision,
//! and make assertions about the resulting changes.
use std::fmt::Debug;

use crate::{
    decision::{Decision, State},
    Event,
};

/// Test harness for testing decisions.
pub struct TestHarness;

impl TestHarness {
    /// Sets up a history of events.
    ///
    /// # Arguments
    ///
    /// * `history` - A history of events to derive the current state.
    ///
    /// # Returns
    ///
    /// A `TestHarnessStep` representing the "given" step.
    pub fn given<E: Event + Clone>(history: impl Into<Vec<E>>) -> TestHarnessStep<E, Given> {
        TestHarnessStep {
            history: history.into(),
            _step: Given,
        }
    }
}

/// Represents the given step of the test harness.
pub struct Given;

/// Represents when step of the test harness.
pub struct When<R, ERR> {
    result: Result<Vec<R>, ERR>,
}

pub struct TestHarnessStep<E: Event + Clone, ST> {
    history: Vec<E>,
    _step: ST,
}

impl<E: Event + Clone> TestHarnessStep<E, Given> {
    /// Executes a decision on the state derived from the given history.
    ///
    /// # Arguments
    ///
    /// * `decision` - The decision to test.
    ///
    /// # Returns
    ///
    /// A `TestHarnessStep` representing the "when" step.
    pub fn when<D, S, R, ERR>(self, decision: D) -> TestHarnessStep<E, When<R, ERR>>
    where
        D: Decision<Event = R, Error = ERR, State = S>,
        S: State<Event = E>,
        R: Event,
    {
        let mut state = decision.default_state();
        for event in self.history.iter() {
            state.mutate(event.clone());
        }
        let result = decision.process(&state);
        TestHarnessStep {
            history: self.history,
            _step: When { result },
        }
    }
}

impl<R, E, ERR> TestHarnessStep<E, When<R, ERR>>
where
    E: Event + Clone + PartialEq,
    R: Debug + PartialEq,
    ERR: Debug + PartialEq,
{
    /// Makes assertions about the changes.
    ///
    /// # Arguments
    ///
    /// * `expected` - The expected changes.
    ///
    /// # Panics
    ///
    /// Panics if the action result is not `Ok` or if the changes do not match the expected changes.
    ///
    /// # Examples
    #[track_caller]
    pub fn then(self, expected: impl Into<Vec<R>>) {
        assert_eq!(Ok(expected.into()), self._step.result);
    }

    /// Makes assertions about the expected error result.
    ///
    /// # Arguments
    ///
    /// * `expected` - The expected error.
    ///
    /// # Panics
    ///
    /// Panics if the action result is not `Err` or if the error does not match the expected error.
    #[track_caller]
    pub fn then_err(self, expected: ERR) {
        let err = self._step.result.unwrap_err();
        assert_eq!(err, expected);
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    use crate::decision::State;
    use crate::event::{Event, EventSchema};
    use crate::stream_query::StreamQuery;

    #[derive(Clone, Debug, PartialEq)]
    enum SampleEvent {
        Created(String),
        Deleted(String),
    }

    impl Event for SampleEvent {
        const SCHEMA: EventSchema = EventSchema {
            types: &["Created", "Deleted"],
            domain_identifiers: &[],
        };

        fn domain_identifiers(&self) -> crate::domain_identifier::DomainIdentifierSet {
            todo!()
        }

        fn name(&self) -> &'static str {
            todo!()
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct SampleState {
        changes: Vec<SampleEvent>,
    }
    impl SampleState {
        fn new() -> Self {
            SampleState {
                changes: Vec::new(),
            }
        }
    }

    impl State for SampleState {
        type Event = SampleEvent;

        fn query(&self) -> StreamQuery<Self::Event> {
            todo!()
        }

        fn mutate(&mut self, event: Self::Event) {
            self.changes.push(event);
        }
    }

    struct SampleDecision {
        result: Result<Vec<SampleEvent>, &'static str>,
    }

    impl SampleDecision {
        fn new(result: Result<Vec<SampleEvent>, &'static str>) -> Self {
            Self { result }
        }
    }

    impl Decision for SampleDecision {
        type Event = SampleEvent;
        type State = SampleState;
        type Error = &'static str;

        fn default_state(&self) -> Self::State {
            SampleState::new()
        }

        fn process(&self, _state: &Self::State) -> Result<Vec<Self::Event>, Self::Error> {
            self.result.clone()
        }

        fn validation_query(&self) -> Option<StreamQuery<<Self::State as State>::Event>> {
            None
        }
    }

    #[test]
    fn it_should_set_up_initial_state_and_apply_the_history() {
        TestHarness::given(vec![SampleEvent::Created("x".into())])
            .when(SampleDecision::new(Ok(vec![
                SampleEvent::Created("x".into()),
                SampleEvent::Deleted("x".into()),
            ])))
            .then([
                SampleEvent::Created("x".into()),
                SampleEvent::Deleted("x".into()),
            ]);
    }

    #[test]
    #[should_panic]
    fn it_should_panic_when_action_failed_and_events_were_expected() {
        TestHarness::given([])
            .when(SampleDecision::new(Err("Some error")))
            .then([SampleEvent::Deleted("x".into())]);
    }

    #[test]
    fn it_should_assert_expected_error_with_then_err() {
        TestHarness::given([])
            .when(SampleDecision::new(Err("Some error")))
            .then_err("Some error");
    }

    #[test]
    #[should_panic]
    fn it_should_panic_when_an_error_is_expected() {
        TestHarness::given(vec![SampleEvent::Created("x".into())])
            .when(SampleDecision::new(Ok(vec![
                SampleEvent::Created("x".into()),
                SampleEvent::Deleted("x".into()),
            ])))
            .then_err("Some error");
    }
}
