mod course;
mod student;
mod subscription;
mod unsubscription;
pub use course::{CloseCourse, CourseError, CourseId, CreateCourse, RenameCourse};
use disintegrate::macros::Event;
pub use student::{RegisterStudent, StudentError, StudentId};
pub use subscription::{SubscribeStudent, Subscription, SubscriptionError};
pub use unsubscription::{UnsubscribeStudent, UnsubscriptionError};

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
