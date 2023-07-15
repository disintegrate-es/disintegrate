use super::Application;
use crate::domain::{
    CloseCourse, CreateCourse, DomainEvent, RegisterStudent, RenameCourse, SubscribeStudent,
    UnsubscribeStudent,
};
use anyhow::Result;
use disintegrate::EventStore;

impl<ES> Application<ES>
where
    ES: EventStore<DomainEvent>,
    <ES as disintegrate::EventStore<DomainEvent>>::Error: std::error::Error + 'static,
{
    pub async fn create_course(&self, command: CreateCourse) -> Result<()> {
        println!("create course id {}", command.course_id);
        self.decision_maker.make(command).await?;
        Ok(())
    }

    pub async fn close_course(&self, command: CloseCourse) -> Result<()> {
        println!("close course id {}", command.course_id);
        self.decision_maker.make(command).await?;
        Ok(())
    }

    pub async fn rename_course(&self, command: RenameCourse) -> Result<()> {
        println!("rename course id {}", command.course_id);
        self.decision_maker.make(command).await?;
        Ok(())
    }

    pub async fn register_student(&self, command: RegisterStudent) -> Result<()> {
        println!("register student id {}", command.student_id);
        self.decision_maker.make(command).await?;
        Ok(())
    }

    pub async fn subscribe_student(&self, command: SubscribeStudent) -> Result<()> {
        println!(
            "subscribe student id {} course id {}",
            command.student_id, command.course_id
        );
        self.decision_maker.make(command).await?;
        Ok(())
    }

    pub async fn unsubscribe_student(&self, command: UnsubscribeStudent) -> Result<()> {
        println!(
            "unsubscribe student id {} course id {}",
            command.student_id, command.course_id
        );
        self.decision_maker.make(command).await?;
        Ok(())
    }
}
