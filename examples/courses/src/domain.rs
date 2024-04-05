mod course;
mod student;
mod subscription;
mod unsubscription;
pub use course::{CloseCourse, CourseError, CourseId, CreateCourse, RenameCourse};
use disintegrate::Event;
pub use student::{RegisterStudent, StudentError, StudentId};
pub use subscription::{SubscribeStudent, SubscriptionError};
pub use unsubscription::{UnsubscribeStudent, UnsubscriptionError};

#[derive(Debug, Clone, PartialEq, Eq, Event)]
#[stream(CourseSubscriptionEvent, [CourseCreated, CourseClosed, StudentSubscribed, StudentUnsubscribed])]
#[stream(StudentSubscriptionEvent, [StudentSubscribed, StudentUnsubscribed, StudentRegistered])]
#[stream(UnsubscriptionEvent, [StudentSubscribed, StudentUnsubscribed])]
#[stream(CourseEvent, [CourseCreated, CourseClosed, CourseRenamed])]
#[stream(StudentEvent, [StudentRegistered])]
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
