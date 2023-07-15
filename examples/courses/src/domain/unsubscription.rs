use disintegrate::{Decision, State, StreamQuery};

use super::{CourseId, DomainEvent, StudentId, UnsubscriptionEvent};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UnsubscriptionError {
    #[error("student not subscribed")]
    StudentNotSubscribed,
}

#[derive(Debug, Clone, Default)]
pub struct Unsubscription {
    course_id: CourseId,
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

impl State for Unsubscription {
    type Event = UnsubscriptionEvent;

    fn query(&self) -> StreamQuery<Self::Event> {
        disintegrate::query!(
            UnsubscriptionEvent,
                (course_id == self.course_id) and
                (student_id == self.student_id)
        )
    }

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

    type State = Unsubscription;

    type Error = UnsubscriptionError;

    fn default_state(&self) -> Self::State {
        Unsubscription::new(self.student_id.clone(), self.course_id.clone())
    }

    fn validation_query(&self) -> Option<StreamQuery<<Self::State as State>::Event>> {
        None
    }

    fn process(&self, state: &Self::State) -> Result<Vec<Self::Event>, Self::Error> {
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
        disintegrate::TestHarness::given([UnsubscriptionEvent::StudentSubscribed {
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
