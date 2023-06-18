use disintegrate::{State, StreamQuery};

use super::{CourseId, StudentId, SubscriptionEvent};

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

#[derive(Debug, Clone, Default)]
pub struct Course {
    id: CourseId,
    available_seats: u32,
    closed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Student {
    id: StudentId,
    subscribed_courses: Vec<CourseId>,
    registered: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Subscription {
    course: Course,
    student: Student,
}

impl Subscription {
    pub fn new(course_id: CourseId, student_id: StudentId) -> Self {
        Self {
            course: Course {
                id: course_id,
                ..Default::default()
            },
            student: Student {
                id: student_id,
                ..Default::default()
            },
        }
    }

    pub fn subscribe(&self) -> Result<Vec<SubscriptionEvent>, SubscriptionError> {
        const MAX_STUDENT_COURSES: usize = 2;

        if !self.student.registered {
            return Err(SubscriptionError::StudentNotRegistered);
        }

        if self.course.closed {
            return Err(SubscriptionError::CourseClosed);
        }

        if self.course.available_seats == 0 {
            return Err(SubscriptionError::NoSeatsAvailable);
        }

        if self
            .student
            .subscribed_courses
            .iter()
            .any(|c| c == &self.course.id)
        {
            return Err(SubscriptionError::StudentAlreadySubscribed);
        }

        if self.student.subscribed_courses.len() >= MAX_STUDENT_COURSES {
            return Err(SubscriptionError::StudentHasTooManyCourses);
        }

        Ok(vec![SubscriptionEvent::StudentSubscribed {
            course_id: self.course.id.clone(),
            student_id: self.student.id.clone(),
        }])
    }
}

impl State for Subscription {
    type Event = SubscriptionEvent;

    fn query(&self) -> StreamQuery<Self::Event> {
        disintegrate::query!(
            SubscriptionEvent,
                (course_id == self.course.id.clone()) or
                (student_id == self.student.id.clone())
        )
    }

    fn mutate(&mut self, event: Self::Event) {
        match event {
            SubscriptionEvent::CourseCreated { seats, .. } => {
                self.course.available_seats = seats;
            }
            SubscriptionEvent::CourseClosed { .. } => {
                self.course.closed = true;
            }
            SubscriptionEvent::StudentSubscribed {
                student_id,
                course_id,
                ..
            } => {
                if self.course.id == course_id {
                    self.course.available_seats -= 1;
                }
                if self.student.id == student_id {
                    self.student.subscribed_courses.push(course_id);
                }
            }
            SubscriptionEvent::StudentUnsubscribed {
                student_id,
                course_id,
                ..
            } => {
                if self.course.id == course_id {
                    self.course.available_seats += 1;
                }
                if self.student.id == student_id {
                    let index = self
                        .student
                        .subscribed_courses
                        .iter()
                        .position(|student_course_id| *student_course_id == course_id)
                        .unwrap();
                    self.student.subscribed_courses.remove(index);
                }
            }
            SubscriptionEvent::StudentRegistered { .. } => self.student.registered = true,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_subscribes_a_student() {
        disintegrate::TestHarness::given(
            Subscription::new("some course".to_string(), "some student".to_string()),
            [
                SubscriptionEvent::CourseCreated {
                    course_id: "some course".to_string(),
                    name: "some name".to_string(),
                    seats: 1,
                },
                SubscriptionEvent::StudentRegistered {
                    student_id: "some student".to_string(),
                    name: "some name".to_string(),
                },
            ],
        )
        .when(|s| s.subscribe())
        .then(vec![SubscriptionEvent::StudentSubscribed {
            course_id: "some course".into(),
            student_id: "some student".into(),
        }]);
    }

    #[test]
    fn it_should_not_subscribe_an_unregistered_student() {
        disintegrate::TestHarness::given(
            Subscription::new("some course".to_string(), "some student".to_string()),
            [SubscriptionEvent::CourseCreated {
                course_id: "some course".to_string(),
                name: "some name".to_string(),
                seats: 1,
            }],
        )
        .when(|s| s.subscribe())
        .then_err(SubscriptionError::StudentNotRegistered);
    }

    #[test]
    fn it_should_not_subscribe_a_student_to_a_closed_course() {
        disintegrate::TestHarness::given(
            Subscription::new("some course".to_string(), "some student".to_string()),
            [
                SubscriptionEvent::CourseCreated {
                    course_id: "some course".to_string(),
                    name: "some name".to_string(),
                    seats: 1,
                },
                SubscriptionEvent::StudentRegistered {
                    student_id: "some student".to_string(),
                    name: "some name".to_string(),
                },
                SubscriptionEvent::CourseClosed {
                    course_id: "some course".to_string(),
                },
            ],
        )
        .when(|s| s.subscribe())
        .then_err(SubscriptionError::CourseClosed);
    }

    #[test]
    fn it_should_not_subscribe_a_student_to_a_full_course() {
        disintegrate::TestHarness::given(
            Subscription::new("some course".to_string(), "some student".to_string()),
            [
                SubscriptionEvent::CourseCreated {
                    course_id: "some course".to_string(),
                    name: "some name".to_string(),
                    seats: 1,
                },
                SubscriptionEvent::StudentRegistered {
                    student_id: "some student".to_string(),
                    name: "some name".to_string(),
                },
                SubscriptionEvent::StudentSubscribed {
                    student_id: "another student".to_string(),
                    course_id: "some course".to_string(),
                },
            ],
        )
        .when(|s| s.subscribe())
        .then_err(SubscriptionError::NoSeatsAvailable);
    }

    #[test]
    fn it_should_not_subscribe_a_student_that_is_already_subscribed() {
        disintegrate::TestHarness::given(
            Subscription::new("some course".to_string(), "some student".to_string()),
            [
                SubscriptionEvent::CourseCreated {
                    course_id: "some course".to_string(),
                    name: "some name".to_string(),
                    seats: 2,
                },
                SubscriptionEvent::StudentRegistered {
                    student_id: "some student".to_string(),
                    name: "some name".to_string(),
                },
                SubscriptionEvent::StudentSubscribed {
                    student_id: "some student".to_string(),
                    course_id: "some course".to_string(),
                },
            ],
        )
        .when(|s| s.subscribe())
        .then_err(SubscriptionError::StudentAlreadySubscribed);
    }

    #[test]
    fn it_should_not_subscribe_a_student_that_attends_two_courses() {
        disintegrate::TestHarness::given(
            Subscription::new("some course".to_string(), "some student".to_string()),
            [
                SubscriptionEvent::CourseCreated {
                    course_id: "some course".to_string(),
                    name: "some name".to_string(),
                    seats: 1,
                },
                SubscriptionEvent::StudentRegistered {
                    student_id: "some student".to_string(),
                    name: "some name".to_string(),
                },
                SubscriptionEvent::StudentSubscribed {
                    student_id: "some student".to_string(),
                    course_id: "another course".to_string(),
                },
                SubscriptionEvent::StudentSubscribed {
                    student_id: "some student".to_string(),
                    course_id: "yet another course".to_string(),
                },
            ],
        )
        .when(|s| s.subscribe())
        .then_err(SubscriptionError::StudentHasTooManyCourses);
    }
}
