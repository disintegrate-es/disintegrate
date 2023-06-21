use disintegrate::{State, StreamQuery};

use super::{CourseId, StudentId, UnsubscriptionEvent};

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
    pub fn new(course_id: CourseId, student_id: StudentId) -> Self {
        Self {
            course_id,
            student_id,
            ..Default::default()
        }
    }

    pub fn unsubscribe(&self) -> Result<Vec<UnsubscriptionEvent>, UnsubscriptionError> {
        if !self.student_subscribed {
            return Err(UnsubscriptionError::StudentNotSubscribed);
        }

        Ok(vec![UnsubscriptionEvent::StudentUnsubscribed {
            course_id: self.course_id.clone(),
            student_id: self.student_id.clone(),
        }])
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_unsubscribes_a_student() {
        disintegrate::TestHarness::given(
            Unsubscription::new("some course".to_string(), "some student".to_string()),
            [UnsubscriptionEvent::StudentSubscribed {
                student_id: "some student".to_string(),
                course_id: "some course".to_string(),
            }],
        )
        .when(|s| s.unsubscribe())
        .then(vec![UnsubscriptionEvent::StudentUnsubscribed {
            course_id: "some course".into(),
            student_id: "some student".into(),
        }]);
    }

    #[test]
    fn it_should_not_unsubscribe_a_student_not_subscribed() {
        disintegrate::TestHarness::given(
            Unsubscription::new("some course".to_string(), "some student".to_string()),
            [],
        )
        .when(|s| s.unsubscribe())
        .then_err(UnsubscriptionError::StudentNotSubscribed);
    }
}
