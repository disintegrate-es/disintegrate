use disintegrate::{State, StreamQuery};

use super::CourseEvent;

pub type CourseId = String;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CourseError {
    #[error("not found")]
    NotFound,
    #[error("already created")]
    AlreadyCreated,
    #[error("course alread closed")]
    AlreadyClosed,
    #[error("invalid seats")]
    InvalidSeats,
    #[error("name empty")]
    NameEmpty,
}

#[derive(Debug, Clone)]
pub struct Course {
    course_id: CourseId,
    name: String,
    seats: u32,
    created: bool,
    closed: bool,
}

impl Course {
    pub fn new(course_id: CourseId) -> Self {
        Self {
            course_id,
            name: "".to_string(),
            seats: 0,
            created: false,
            closed: false,
        }
    }

    pub fn create(&self, name: &str, seats: u32) -> Result<Vec<CourseEvent>, CourseError> {
        if self.created {
            return Err(CourseError::AlreadyCreated);
        }

        if name.is_empty() {
            return Err(CourseError::NameEmpty);
        }

        Ok(vec![CourseEvent::CourseCreated {
            course_id: self.course_id.clone(),
            name: name.to_string(),
            seats,
        }])
    }

    pub fn rename(&self, name: &str) -> Result<Vec<CourseEvent>, CourseError> {
        if !self.created {
            return Err(CourseError::NotFound);
        }

        if name.is_empty() {
            return Err(CourseError::NameEmpty);
        }

        Ok(vec![CourseEvent::CourseRenamed {
            course_id: self.course_id.clone(),
            name: name.to_string(),
        }])
    }

    pub fn close(&self) -> Result<Vec<CourseEvent>, CourseError> {
        if !self.created {
            return Err(CourseError::NotFound);
        }

        if self.closed {
            return Err(CourseError::AlreadyClosed);
        }

        Ok(vec![CourseEvent::CourseClosed {
            course_id: self.course_id.clone(),
        }])
    }
}

impl State for Course {
    type Event = CourseEvent;

    fn query(&self) -> StreamQuery<Self::Event> {
        disintegrate::query!(CourseEvent, course_id == self.course_id.clone())
    }

    fn mutate(&mut self, event: Self::Event) {
        match event {
            CourseEvent::CourseCreated { seats, name, .. } => {
                self.name = name;
                self.seats = seats;
                self.created = true;
            }
            CourseEvent::CourseClosed { .. } => {
                self.closed = true;
            }
            CourseEvent::CourseRenamed { name, .. } => {
                self.name = name;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_creates_a_new_course() {
        disintegrate::TestHarness::given(Course::new("1".to_string()), [])
            .when(|s| s.create("test course", 1))
            .then(vec![CourseEvent::CourseCreated {
                course_id: "1".into(),
                name: "test course".into(),
                seats: 1,
            }]);
    }

    #[test]
    fn it_should_not_create_a_course_when_it_already_exists() {
        disintegrate::TestHarness::given(
            Course::new("1".into()),
            [CourseEvent::CourseCreated {
                course_id: "1".into(),
                name: "test course".into(),
                seats: 3,
            }],
        )
        .when(|s| s.create("test course", 1))
        .then_err(CourseError::AlreadyCreated);
    }

    #[test]
    fn it_should_not_create_a_course_when_the_provided_name_is_empty() {
        disintegrate::TestHarness::given(Course::new("1".into()), [])
            .when(|s| s.rename("new name"))
            .then_err(CourseError::NotFound);
    }

    #[test]
    fn it_should_rename_a_course() {
        disintegrate::TestHarness::given(
            Course::new("1".into()),
            [CourseEvent::CourseCreated {
                course_id: "1".into(),
                name: "old name".into(),
                seats: 1,
            }],
        )
        .when(|s| s.rename("new name"))
        .then(vec![CourseEvent::CourseRenamed {
            course_id: "1".into(),
            name: "new name".into(),
        }]);
    }

    #[test]
    fn it_should_not_rename_a_course_when_it_does_not_exist() {
        disintegrate::TestHarness::given(Course::new("1".into()), [])
            .when(|s| s.create("", 1))
            .then_err(CourseError::NameEmpty);
    }

    #[test]
    fn it_should_not_rename_a_course_when_the_new_name_is_empty() {
        disintegrate::TestHarness::given(
            Course::new("1".into()),
            [CourseEvent::CourseCreated {
                course_id: "1".into(),
                name: "old name".into(),
                seats: 1,
            }],
        )
        .when(|s| s.rename(""))
        .then_err(CourseError::NameEmpty);
    }

    #[test]
    fn it_should_close_a_course() {
        disintegrate::TestHarness::given(
            Course::new("1".into()),
            [CourseEvent::CourseCreated {
                course_id: "1".into(),
                name: "old name".into(),
                seats: 1,
            }],
        )
        .when(|s| s.close())
        .then(vec![CourseEvent::CourseClosed {
            course_id: "1".into(),
        }])
    }

    #[test]
    fn it_should_not_close_a_course_when_it_does_not_exist() {
        disintegrate::TestHarness::given(Course::new("1".into()), [])
            .when(|s| s.close())
            .then_err(CourseError::NotFound)
    }

    #[test]
    fn it_should_not_close_a_course_when_it_is_already_closed() {
        disintegrate::TestHarness::given(
            Course::new("1".into()),
            [
                CourseEvent::CourseCreated {
                    course_id: "1".into(),
                    name: "old name".into(),
                    seats: 1,
                },
                CourseEvent::CourseClosed {
                    course_id: "1".into(),
                },
            ],
        )
        .when(|s| s.close())
        .then_err(CourseError::AlreadyClosed)
    }
}
