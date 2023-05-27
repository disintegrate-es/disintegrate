use crate::{domain::DomainEvent, proto, proto::Event as ProtoDomainEvent};

impl From<DomainEvent> for ProtoDomainEvent {
    fn from(event: DomainEvent) -> Self {
        Self {
            event: Some(match event {
                DomainEvent::CourseCreated {
                    course_id,
                    name,
                    seats,
                } => proto::event::Event::CourseCreated(proto::event::CourseCreated {
                    course_id,
                    name,
                    seats,
                }),
                DomainEvent::CourseClosed { course_id } => {
                    proto::event::Event::CourseClosed(proto::event::CourseClosed { course_id })
                }
                DomainEvent::CourseRenamed { course_id, name } => {
                    proto::event::Event::CourseRenamed(proto::event::CourseRenamed {
                        course_id,
                        name,
                    })
                }
                DomainEvent::StudentRegistered { student_id, name } => {
                    proto::event::Event::StudentRegistered(proto::event::StudentRegistered {
                        student_id,
                        name,
                    })
                }
                DomainEvent::StudentSubscribed {
                    student_id,
                    course_id,
                } => proto::event::Event::StudentSubscribed(proto::event::StudentSubscribed {
                    student_id,
                    course_id,
                }),
                DomainEvent::StudentUnsubscribed {
                    student_id,
                    course_id,
                } => proto::event::Event::StudentUnsubscribed(proto::event::StudentUnsubscribed {
                    student_id,
                    course_id,
                }),
            }),
        }
    }
}

impl From<ProtoDomainEvent> for DomainEvent {
    fn from(proto: ProtoDomainEvent) -> Self {
        match proto.event.expect("event is a required field") {
            proto::event::Event::CourseCreated(proto::event::CourseCreated {
                course_id,
                name,
                seats,
            }) => DomainEvent::CourseCreated {
                course_id,
                name,
                seats,
            },
            proto::event::Event::CourseClosed(proto::event::CourseClosed { course_id }) => {
                DomainEvent::CourseClosed { course_id }
            }
            proto::event::Event::CourseRenamed(proto::event::CourseRenamed { course_id, name }) => {
                DomainEvent::CourseRenamed { course_id, name }
            }
            proto::event::Event::StudentRegistered(proto::event::StudentRegistered {
                student_id,
                name,
            }) => DomainEvent::StudentRegistered { student_id, name },
            proto::event::Event::StudentSubscribed(proto::event::StudentSubscribed {
                student_id,
                course_id,
            }) => DomainEvent::StudentSubscribed {
                student_id,
                course_id,
            },
            proto::event::Event::StudentUnsubscribed(proto::event::StudentUnsubscribed {
                student_id,
                course_id,
            }) => DomainEvent::StudentUnsubscribed {
                student_id,
                course_id,
            },
        }
    }
}
