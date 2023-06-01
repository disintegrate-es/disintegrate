//! Utility for testing a State implementation
//!
//! The test harness allows you to set up the initial state and a history of events, perform actions on the state,
//! and make assertions about the resulting state changes.
use std::fmt::Debug;

use crate::state::State;

/// Test harness for testing code that depends on a state.
pub struct TestHarness;

impl TestHarness {
    /// Sets up the initial state and applies a history of events.
    ///
    /// # Arguments
    ///
    /// * `state` - The initial state.
    /// * `history` - A history of events to apply to the state.
    ///
    /// # Returns
    ///
    /// A `TestHarnessStep` representing the "given" step.
    pub fn given<S: State>(
        mut state: S,
        history: impl Into<Vec<S::Event>>,
    ) -> TestHarnessStep<S, Given> {
        for event in history.into() {
            state.mutate(event);
        }
        TestHarnessStep {
            state,
            _step: Given,
        }
    }
}

/// Represents the initial state setup step in the test harness.
pub struct Given;

/// Represents the action execution step in the test harness.
pub struct When<R, E> {
    result: Result<R, E>,
}

pub struct TestHarnessStep<S: State, ST> {
    state: S,
    _step: ST,
}

impl<S: State> TestHarnessStep<S, Given> {
    /// Executes an action on the current state.
    ///
    /// # Arguments
    ///
    /// * `func` - The function representing the action to execute on the state.
    ///
    /// # Returns
    ///
    /// A `TestHarnessStep` representing the "when" step.
    pub fn when<F, R, E>(mut self, func: F) -> TestHarnessStep<S, When<R, E>>
    where
        F: FnOnce(&mut S) -> Result<R, E>,
    {
        let result = func(&mut self.state);
        TestHarnessStep {
            state: self.state,
            _step: When { result },
        }
    }
}

impl<S, R, E> TestHarnessStep<S, When<R, E>>
where
    S: State,
    <S as State>::Event: Debug + PartialEq,
    E: Debug + PartialEq,
{
    /// Makes assertions about the state changes.
    ///
    /// # Arguments
    ///
    /// * `expected` - The expected changes to the state.
    ///
    /// # Panics
    ///
    /// Panics if the action result is not `Ok` or if the state changes do not match the expected changes.
    ///
    /// # Examples
    pub fn then(mut self, expected: Vec<S::Event>) {
        assert!(self._step.result.is_ok());
        assert_eq!(expected.as_ref(), self.state.changes());
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
    pub fn then_err(self, expected: E) {
        let Err(err) = self._step.result else { panic!("expected error {:?}", expected) };
        assert!(err == expected);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Event, EventSchema};
    use crate::state::State;
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

        fn changes(&mut self) -> Vec<Self::Event> {
            std::mem::take(&mut self.changes)
        }
    }

    #[test]
    fn it_should_set_up_initial_state_and_apply_the_history() {
        let history = vec![SampleEvent::Created("x".into())];

        TestHarness::given(SampleState::new(), history)
            .when(|s| {
                s.mutate(SampleEvent::Deleted("x".into()));
                Ok::<(), String>(())
            })
            .then(vec![
                SampleEvent::Created("x".into()),
                SampleEvent::Deleted("x".into()),
            ]);
    }

    #[test]
    #[should_panic]
    fn it_should_panic_when_action_failed_and_events_were_expected() {
        TestHarness::given(SampleState::new(), [])
            .when(|_| Err::<(), &'static str>("Some error"))
            .then(vec![SampleEvent::Deleted("x".into())]);
    }

    #[test]
    fn it_should_assert_expected_error_with_then_err() {
        TestHarness::given(SampleState::new(), [])
            .when(|_| Err::<(), &'static str>("Some error"))
            .then_err("Some error");
    }

    #[test]
    #[should_panic(expected = "expected error \"Some error\"")]
    fn it_should_panic_when_an_error_is_expected() {
        TestHarness::given(SampleState::new(), [])
            .when(|_| Ok::<(), &'static str>(()))
            .then_err("Some error");
    }
}
