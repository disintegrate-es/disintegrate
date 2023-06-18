use disintegrate::{State, StreamQuery};

use super::StudentEvent;

pub type StudentId = String;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StudentError {
    #[error("not found")]
    NotFound,
    #[error("already registered")]
    AlreadyRegistered,
    #[error("name empty")]
    NameEmpty,
}

#[derive(Debug, Clone)]
pub struct Student {
    student_id: StudentId,
    name: String,
    registered: bool,
}

impl Student {
    pub fn new(student_id: StudentId) -> Self {
        Self {
            student_id,
            name: "".to_string(),
            registered: false,
        }
    }

    pub fn register(&self, name: &str) -> Result<Vec<StudentEvent>, StudentError> {
        if self.registered {
            return Err(StudentError::AlreadyRegistered);
        }
        if name.is_empty() {
            return Err(StudentError::NameEmpty);
        }

        Ok(vec![StudentEvent::StudentRegistered {
            student_id: self.student_id.clone(),
            name: name.into(),
        }])
    }
}

impl State for Student {
    type Event = StudentEvent;

    fn query(&self) -> StreamQuery<Self::Event> {
        disintegrate::query!(StudentEvent, student_id == self.student_id.clone())
    }

    fn mutate(&mut self, event: Self::Event) {
        match event {
            StudentEvent::StudentRegistered { name, .. } => {
                self.registered = true;
                self.name = name;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_register_a_new_student() {
        disintegrate::TestHarness::given(Student::new("1".to_string()), [])
            .when(|s| s.register("some name"))
            .then(vec![StudentEvent::StudentRegistered {
                student_id: "1".into(),
                name: "some name".into(),
            }]);
    }

    #[test]
    fn it_should_not_register_a_student_when_it_already_exists() {
        disintegrate::TestHarness::given(
            Student::new("1".into()),
            [StudentEvent::StudentRegistered {
                student_id: "1".into(),
                name: "some name".into(),
            }],
        )
        .when(|s| s.register("some name"))
        .then_err(StudentError::AlreadyRegistered);
    }
}
