use disintegrate::{Decision, StateMutate, StateQuery};
use serde::{Deserialize, Serialize};

use super::{CourseEvent, DomainEvent};

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

#[derive(Debug, StateQuery, Clone, Serialize, Deserialize)]
#[state_query(CourseEvent)]
pub struct Course {
    #[id]
    course_id: CourseId,
    name: String,
    created: bool,
    closed: bool,
}

impl Course {
    pub fn new(course_id: CourseId) -> Self {
        Self {
            course_id,
            name: "".to_string(),
            created: false,
            closed: false,
        }
    }
}

impl StateMutate for Course {
    fn mutate(&mut self, event: Self::Event) {
        match event {
            CourseEvent::CourseCreated { name, .. } => {
                self.name = name;
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

pub struct CreateCourse {
    pub course_id: CourseId,
    pub name: String,
    pub seats: u32,
}

impl CreateCourse {
    pub fn new(course_id: CourseId, name: &str, seats: u32) -> Self {
        Self {
            course_id,
            name: name.into(),
            seats,
        }
    }
}

impl Decision for CreateCourse {
    type Event = DomainEvent;

    type StateQuery = Course;

    type Error = CourseError;

    fn state_query(&self) -> Self::StateQuery {
        Course::new(self.course_id.clone())
    }

    fn process(&self, state: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        if state.created {
            return Err(CourseError::AlreadyCreated);
        }

        if self.name.is_empty() {
            return Err(CourseError::NameEmpty);
        }

        Ok(vec![DomainEvent::CourseCreated {
            course_id: self.course_id.clone(),
            name: self.name.clone(),
            seats: self.seats,
        }])
    }
}

pub struct CloseCourse {
    pub course_id: CourseId,
}

impl CloseCourse {
    pub fn new(course_id: CourseId) -> Self {
        Self { course_id }
    }
}
impl Decision for CloseCourse {
    type Event = DomainEvent;

    type StateQuery = Course;

    type Error = CourseError;

    fn state_query(&self) -> Self::StateQuery {
        Course::new(self.course_id.clone())
    }

    fn process(&self, state: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        if !state.created {
            return Err(CourseError::NotFound);
        }

        if state.closed {
            return Err(CourseError::AlreadyClosed);
        }

        Ok(vec![DomainEvent::CourseClosed {
            course_id: self.course_id.clone(),
        }])
    }
}

pub struct RenameCourse {
    pub course_id: CourseId,
    pub name: String,
}

impl RenameCourse {
    pub fn new(course_id: CourseId, name: &str) -> Self {
        Self {
            course_id,
            name: name.into(),
        }
    }
}
impl Decision for RenameCourse {
    type Event = DomainEvent;

    type StateQuery = Course;

    type Error = CourseError;

    fn state_query(&self) -> Self::StateQuery {
        Course::new(self.course_id.clone())
    }

    fn process(&self, state: &Self::StateQuery) -> Result<Vec<Self::Event>, Self::Error> {
        if !state.created {
            return Err(CourseError::NotFound);
        }

        if self.name.is_empty() {
            return Err(CourseError::NameEmpty);
        }

        Ok(vec![DomainEvent::CourseRenamed {
            course_id: self.course_id.clone(),
            name: self.name.to_string(),
        }])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_creates_a_new_course() {
        disintegrate::TestHarness::given([])
            .when(CreateCourse::new("1".into(), "test course", 1))
            .then([DomainEvent::CourseCreated {
                course_id: "1".into(),
                name: "test course".into(),
                seats: 1,
            }]);
    }

    #[test]
    fn it_should_not_create_a_course_when_it_already_exists() {
        disintegrate::TestHarness::given([DomainEvent::CourseCreated {
            course_id: "1".into(),
            name: "test course".into(),
            seats: 3,
        }])
        .when(CreateCourse::new("1".into(), "some course", 1))
        .then_err(CourseError::AlreadyCreated);
    }

    #[test]
    fn it_should_not_create_a_course_when_the_provided_name_is_empty() {
        disintegrate::TestHarness::given([])
            .when(RenameCourse::new("1".into(), "new name"))
            .then_err(CourseError::NotFound);
    }

    #[test]
    fn it_renames_a_course() {
        disintegrate::TestHarness::given([DomainEvent::CourseCreated {
            course_id: "1".into(),
            name: "old name".into(),
            seats: 1,
        }])
        .when(RenameCourse::new("1".into(), "new name"))
        .then(vec![DomainEvent::CourseRenamed {
            course_id: "1".into(),
            name: "new name".into(),
        }]);
    }

    #[test]
    fn it_should_not_rename_a_course_when_it_does_not_exist() {
        disintegrate::TestHarness::given([])
            .when(RenameCourse::new("1".into(), "new name"))
            .then_err(CourseError::NotFound);
    }

    #[test]
    fn it_should_not_rename_a_course_when_the_new_name_is_empty() {
        disintegrate::TestHarness::given([DomainEvent::CourseCreated {
            course_id: "1".into(),
            name: "old name".into(),
            seats: 1,
        }])
        .when(RenameCourse::new("1".into(), ""))
        .then_err(CourseError::NameEmpty);
    }

    #[test]
    fn it_closes_a_course() {
        disintegrate::TestHarness::given([DomainEvent::CourseCreated {
            course_id: "1".into(),
            name: "old name".into(),
            seats: 1,
        }])
        .when(CloseCourse::new("1".into()))
        .then(vec![DomainEvent::CourseClosed {
            course_id: "1".into(),
        }])
    }

    #[test]
    fn it_should_not_close_a_course_when_it_does_not_exist() {
        disintegrate::TestHarness::given([])
            .when(CloseCourse::new("1".into()))
            .then_err(CourseError::NotFound)
    }

    #[test]
    fn it_should_not_close_a_course_when_it_is_already_closed() {
        disintegrate::TestHarness::given([
            DomainEvent::CourseCreated {
                course_id: "1".into(),
                name: "old name".into(),
                seats: 1,
            },
            DomainEvent::CourseClosed {
                course_id: "1".into(),
            },
        ])
        .when(CloseCourse::new("1".into()))
        .then_err(CourseError::AlreadyClosed)
    }
}
