use disintegrate::{Decision, StateMutate, StateQuery};
use serde::{Deserialize, Serialize};

use super::{CourseId, CourseSubscriptionEvent, DomainEvent, StudentId, StudentSubscriptionEvent};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SubscriptionError {
    #[error("no seats available")]
    NoSeatsAvailable,
    #[error("course closed")]
    CourseClosed,
    #[error("student not registered")]
    StudentNotRegistered,
    #[error("student already subscribed")]
    StudentAlreadySubscribed,
    #[error("student has too many courses")]
    StudentHasTooManyCourses,
}

#[derive(Debug, Clone, StateQuery, Default, Deserialize, Serialize)]
#[state_query(CourseSubscriptionEvent)]
pub struct Course {
    #[id]
    course_id: CourseId,
    available_seats: u32,
    closed: bool,
}

impl Course {
    fn new(course_id: CourseId) -> Self {
        Self {
            course_id,
            ..Default::default()
        }
    }
}

impl StateMutate for Course {
    fn mutate(&mut self, event: Self::Event) {
        match event {
            CourseSubscriptionEvent::CourseCreated { seats, .. } => {
                self.available_seats = seats;
            }
            CourseSubscriptionEvent::CourseClosed { .. } => {
                self.closed = true;
            }
            CourseSubscriptionEvent::StudentSubscribed { .. } => {
                self.available_seats -= 1;
            }
            CourseSubscriptionEvent::StudentUnsubscribed { .. } => {
                self.available_seats += 1;
            }
        }
    }
}

#[derive(Debug, Clone, StateQuery, Default, Serialize, Deserialize)]
#[state_query(StudentSubscriptionEvent)]
pub struct Student {
    #[id]
    student_id: StudentId,
    subscribed_courses: Vec<CourseId>,
    registered: bool,
}

impl Student {
    fn new(student_id: StudentId) -> Self {
        Self {
            student_id,
            ..Default::default()
        }
    }
}
impl StateMutate for Student {
    fn mutate(&mut self, event: Self::Event) {
        match event {
            StudentSubscriptionEvent::StudentSubscribed { course_id, .. } => {
                self.subscribed_courses.push(course_id);
            }
            StudentSubscriptionEvent::StudentUnsubscribed { course_id, .. } => {
                let index = self
                    .subscribed_courses
                    .iter()
                    .position(|student_course_id| *student_course_id == course_id)
                    .unwrap();
                self.subscribed_courses.remove(index);
            }
            StudentSubscriptionEvent::StudentRegistered { .. } => self.registered = true,
        }
    }
}

#[derive(Debug)]
pub struct SubscribeStudent {
    pub student_id: StudentId,
    pub course_id: CourseId,
}

impl SubscribeStudent {
    pub fn new(student_id: StudentId, course_id: CourseId) -> Self {
        Self {
            student_id,
            course_id,
        }
    }
}

impl Decision for SubscribeStudent {
    type Event = DomainEvent;

    type StateQuery = (Course, Student);

    type Error = SubscriptionError;

    fn state_query(&self) -> Self::StateQuery {
        (
            Course::new(self.course_id.clone()),
            Student::new(self.student_id.clone()),
        )
    }

    fn process(
        &self,
        (course, student): &Self::StateQuery,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        const MAX_STUDENT_COURSES: usize = 2;

        if !student.registered {
            return Err(SubscriptionError::StudentNotRegistered);
        }

        if course.closed {
            return Err(SubscriptionError::CourseClosed);
        }

        if course.available_seats == 0 {
            return Err(SubscriptionError::NoSeatsAvailable);
        }

        if student
            .subscribed_courses
            .iter()
            .any(|c| c == &self.course_id)
        {
            return Err(SubscriptionError::StudentAlreadySubscribed);
        }

        if student.subscribed_courses.len() >= MAX_STUDENT_COURSES {
            return Err(SubscriptionError::StudentHasTooManyCourses);
        }

        Ok(vec![DomainEvent::StudentSubscribed {
            course_id: self.course_id.clone(),
            student_id: self.student_id.clone(),
        }])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_subscribes_a_student() {
        disintegrate::TestHarness::given([
            DomainEvent::CourseCreated {
                course_id: "some course".to_string(),
                name: "some name".to_string(),
                seats: 1,
            },
            DomainEvent::StudentRegistered {
                student_id: "some student".to_string(),
                name: "some name".to_string(),
            },
        ])
        .when(SubscribeStudent::new(
            "some student".into(),
            "some course".into(),
        ))
        .then([DomainEvent::StudentSubscribed {
            course_id: "some course".into(),
            student_id: "some student".into(),
        }]);
    }

    #[test]
    fn it_should_not_subscribe_an_unregistered_student() {
        disintegrate::TestHarness::given([DomainEvent::CourseCreated {
            course_id: "some course".to_string(),
            name: "some name".to_string(),
            seats: 1,
        }])
        .when(SubscribeStudent::new(
            "some student".into(),
            "some course".into(),
        ))
        .then_err(SubscriptionError::StudentNotRegistered);
    }

    #[test]
    fn it_should_not_subscribe_a_student_to_a_closed_course() {
        disintegrate::TestHarness::given([
            DomainEvent::CourseCreated {
                course_id: "some course".to_string(),
                name: "some name".to_string(),
                seats: 1,
            },
            DomainEvent::StudentRegistered {
                student_id: "some student".to_string(),
                name: "some name".to_string(),
            },
            DomainEvent::CourseClosed {
                course_id: "some course".to_string(),
            },
        ])
        .when(SubscribeStudent::new(
            "some student".into(),
            "some course".into(),
        ))
        .then_err(SubscriptionError::CourseClosed);
    }

    #[test]
    fn it_should_not_subscribe_a_student_to_a_full_course() {
        disintegrate::TestHarness::given([
            DomainEvent::CourseCreated {
                course_id: "some course".to_string(),
                name: "some name".to_string(),
                seats: 1,
            },
            DomainEvent::StudentRegistered {
                student_id: "some student".to_string(),
                name: "some name".to_string(),
            },
            DomainEvent::StudentSubscribed {
                student_id: "another student".to_string(),
                course_id: "some course".to_string(),
            },
        ])
        .when(SubscribeStudent::new(
            "some student".into(),
            "some course".into(),
        ))
        .then_err(SubscriptionError::NoSeatsAvailable);
    }

    #[test]
    fn it_should_not_subscribe_a_student_that_is_already_subscribed() {
        disintegrate::TestHarness::given([
            DomainEvent::CourseCreated {
                course_id: "some course".to_string(),
                name: "some name".to_string(),
                seats: 2,
            },
            DomainEvent::StudentRegistered {
                student_id: "some student".to_string(),
                name: "some name".to_string(),
            },
            DomainEvent::StudentSubscribed {
                student_id: "some student".to_string(),
                course_id: "some course".to_string(),
            },
        ])
        .when(SubscribeStudent::new(
            "some student".into(),
            "some course".into(),
        ))
        .then_err(SubscriptionError::StudentAlreadySubscribed);
    }

    #[test]
    fn it_should_not_subscribe_a_student_that_attends_two_courses() {
        disintegrate::TestHarness::given([
            DomainEvent::CourseCreated {
                course_id: "some course".to_string(),
                name: "some name".to_string(),
                seats: 1,
            },
            DomainEvent::StudentRegistered {
                student_id: "some student".to_string(),
                name: "some name".to_string(),
            },
            DomainEvent::StudentSubscribed {
                student_id: "some student".to_string(),
                course_id: "another course".to_string(),
            },
            DomainEvent::StudentSubscribed {
                student_id: "some student".to_string(),
                course_id: "yet another course".to_string(),
            },
        ])
        .when(SubscribeStudent::new(
            "some student".into(),
            "some course".into(),
        ))
        .then_err(SubscriptionError::StudentHasTooManyCourses);
    }
}
