mod course;
mod student;
mod subscription;
mod unsubscription;
pub use course::{Course, CourseError, CourseId};
use disintegrate::macros::Event;
pub use student::{Student, StudentError, StudentId};
pub use subscription::{Subscription, SubscriptionError};
pub use unsubscription::{Unsubscription, UnsubscriptionError};

#[derive(Debug, Clone, PartialEq, Eq, Event)]
#[group(SubscriptionEvent, [CourseCreated, CourseClosed, StudentSubscribed, StudentUnsubscribed, StudentRegistered])]
#[group(UnsubscriptionEvent, [StudentSubscribed, StudentUnsubscribed])]
#[group(CourseEvent, [CourseCreated, CourseClosed, CourseRenamed])]
#[group(StudentEvent, [StudentRegistered])]
pub enum DomainEvent {
    CourseCreated {
        #[id]
        course_id: CourseId,
        name: String,
        seats: u32,
    },
    CourseClosed {
        #[id]
        course_id: CourseId,
    },
    CourseRenamed {
        #[id]
        course_id: CourseId,
        name: String,
    },
    StudentRegistered {
        #[id]
        student_id: StudentId,
        name: String,
    },
    StudentSubscribed {
        #[id]
        student_id: StudentId,
        #[id]
        course_id: CourseId,
    },
    StudentUnsubscribed {
        #[id]
        student_id: StudentId,
        #[id]
        course_id: CourseId,
    },
}
