use disintegrate::{Decision, State, StreamQuery};

use super::{DomainEvent, StudentEvent};

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
        disintegrate::query!(StudentEvent, student_id == self.student_id)
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

pub struct RegisterStudent {
    pub student_id: StudentId,
    pub name: String,
}

impl RegisterStudent {
    pub fn new(student_id: StudentId, name: String) -> Self {
        Self { student_id, name }
    }
}

impl Decision for RegisterStudent {
    type Event = DomainEvent;

    type State = Student;

    type Error = StudentError;

    fn default_state(&self) -> Self::State {
        Student::new(self.student_id.clone())
    }

    fn validation_query(&self) -> Option<StreamQuery<<Self::State as State>::Event>> {
        None
    }

    fn process(&self, state: &Self::State) -> Result<Vec<Self::Event>, Self::Error> {
        if state.registered {
            return Err(StudentError::AlreadyRegistered);
        }
        if self.name.is_empty() {
            return Err(StudentError::NameEmpty);
        }

        Ok(vec![DomainEvent::StudentRegistered {
            student_id: self.student_id.clone(),
            name: self.name.clone(),
        }])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_registers_a_new_student() {
        disintegrate::TestHarness::given([])
            .when(RegisterStudent::new("1".into(), "some name".to_string()))
            .then([DomainEvent::StudentRegistered {
                student_id: "1".into(),
                name: "some name".into(),
            }]);
    }

    #[test]
    fn it_should_not_register_a_student_when_it_already_exists() {
        disintegrate::TestHarness::given([StudentEvent::StudentRegistered {
            student_id: "1".into(),
            name: "some name".into(),
        }])
        .when(RegisterStudent::new("1".into(), "some name".to_string()))
        .then_err(StudentError::AlreadyRegistered);
    }
}
