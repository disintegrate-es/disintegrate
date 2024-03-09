use disintegrate::{Decision, StateMutate, StateQuery};
use serde::{Deserialize, Serialize};

use super::{CourseId, DomainEvent, StudentId, UnsubscriptionEvent};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UnsubscriptionError {
    #[error("student not subscribed")]
    StudentNotSubscribed,
}

#[derive(Debug, Clone, StateQuery, Default, Serialize, Deserialize)]
#[state_query(UnsubscriptionEvent)]
pub struct Unsubscription {
    #[id]
    course_id: CourseId,
    #[id]
    student_id: StudentId,
    student_subscribed: bool,
}
impl Unsubscription {
    pub fn new(student_id: StudentId, course_id: CourseId) -> Self {
        Self {
            student_id,
            course_id,
            student_subscribed: false,
        }
    }
}

impl StateMutate for Unsubscription {
    fn mutate(&mut self, event: Self::Event) {
        match event {
            UnsubscriptionEvent::StudentSubscribed { .. } => {
                self.student_subscribed = true;
            }
            UnsubscriptionEvent::StudentUnsubscribed { .. } => {
                self.student_subscribed = false;
            }
        }
    }
}

#[derive(Debug)]
pub struct UnsubscribeStudent {
    pub student_id: StudentId,
    pub course_id: CourseId,
}

impl UnsubscribeStudent {
    pub fn new(student_id: StudentId, course_id: CourseId) -> Self {
        Self {
            student_id,
            course_id,
        }
    }
}

impl Decision for UnsubscribeStudent {
    type Event = DomainEvent;

    type StateQuery = Unsubscription;

    type Error = UnsubscriptionError;

    fn state_query(&self) -> Self::StateQuery {
        Unsubscription::new(self.student_id.clone(), self.course_id.clone())
    }

    fn process(&self, state: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        if !state.student_subscribed {
            return Err(UnsubscriptionError::StudentNotSubscribed);
        }

        Ok(vec![DomainEvent::StudentUnsubscribed {
            course_id: self.course_id.clone(),
            student_id: self.student_id.clone(),
        }])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_unsubscribes_a_student() {
        disintegrate::TestHarness::given([DomainEvent::StudentSubscribed {
            student_id: "some student".to_string(),
            course_id: "some course".to_string(),
        }])
        .when(UnsubscribeStudent::new(
            "some student".into(),
            "some course".into(),
        ))
        .then([DomainEvent::StudentUnsubscribed {
            course_id: "some course".into(),
            student_id: "some student".into(),
        }]);
    }

    #[test]
    fn it_should_not_unsubscribe_a_student_not_subscribed() {
        disintegrate::TestHarness::given([])
            .when(UnsubscribeStudent::new(
                "some student".into(),
                "some course".into(),
            ))
            .then_err(UnsubscriptionError::StudentNotSubscribed);
    }
}
