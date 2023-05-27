use super::Application;
use crate::domain::{
    Course, CourseId, DomainEvent, Student, StudentId, Subscription, Unsubscription,
};
use anyhow::Result;

impl<S> Application<S>
where
    S: disintegrate::StateStore<DomainEvent>,
{
    pub async fn create_course(&self, command: CreateCourse) -> Result<()> {
        println!("create course id {}", command.course_id);
        let mut course = self
            .state_store
            .hydrate(Course::new(command.course_id))
            .await?;
        course.create(&command.name, command.seats)?;

        self.state_store.save(&mut course).await?;

        Ok(())
    }

    pub async fn close_course(&self, command: CloseCourse) -> Result<()> {
        println!("close course id {}", command.course_id);
        let mut course = self
            .state_store
            .hydrate(Course::new(command.course_id))
            .await?;
        course.close()?;

        self.state_store.save(&mut course).await?;

        Ok(())
    }

    pub async fn rename_course(&self, command: RenameCourse) -> Result<()> {
        println!("rename course id {}", command.course_id);
        let mut course = self
            .state_store
            .hydrate(Course::new(command.course_id.clone()))
            .await?;
        course.rename(&command.name)?;

        self.state_store.save(&mut course).await?;

        Ok(())
    }

    pub async fn register_student(&self, command: RegisterStudent) -> Result<()> {
        println!("register student id {}", command.student_id);
        let mut student = self
            .state_store
            .hydrate(Student::new(command.student_id))
            .await?;
        student.register(&command.name)?;

        self.state_store.save(&mut student).await?;

        Ok(())
    }

    pub async fn subscribe_student(&self, command: SubscribeStudent) -> Result<()> {
        println!(
            "subscribe student id {} course id {}",
            command.student_id, command.course_id
        );
        let mut subscription = self
            .state_store
            .hydrate(Subscription::new(command.course_id, command.student_id))
            .await?;
        subscription.subscribe()?;

        self.state_store.save(&mut subscription).await?;

        Ok(())
    }

    pub async fn unsubscribe_student(&self, command: UnsubscribeStudent) -> Result<()> {
        println!(
            "unsubscribe student id {} course id {}",
            command.student_id, command.course_id
        );
        let mut unsubscription = self
            .state_store
            .hydrate(Unsubscription::new(command.course_id, command.student_id))
            .await?;
        unsubscription.unsubscribe()?;

        self.state_store.save(&mut unsubscription).await?;

        Ok(())
    }
}

pub struct CreateCourse {
    pub course_id: CourseId,
    pub name: String,
    pub seats: u32,
}

pub struct CloseCourse {
    pub course_id: CourseId,
}

pub struct RenameCourse {
    pub course_id: CourseId,
    pub name: String,
}

pub struct RegisterStudent {
    pub student_id: StudentId,
    pub name: String,
}

pub struct SubscribeStudent {
    pub student_id: StudentId,
    pub course_id: CourseId,
}
pub struct UnsubscribeStudent {
    pub student_id: StudentId,
    pub course_id: CourseId,
}
