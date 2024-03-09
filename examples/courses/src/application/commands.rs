use super::Application;
use crate::domain::{
    CloseCourse, CreateCourse, RegisterStudent, RenameCourse, SubscribeStudent, UnsubscribeStudent,
};
use anyhow::Result;
use tracing::instrument;

impl Application {
    #[instrument(skip(self))]
    pub async fn create_course(&self, command: CreateCourse) -> Result<()> {
        self.decision_maker.make(command).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn close_course(&self, command: CloseCourse) -> Result<()> {
        self.decision_maker.make(command).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn rename_course(&self, command: RenameCourse) -> Result<()> {
        self.decision_maker.make(command).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn register_student(&self, command: RegisterStudent) -> Result<()> {
        self.decision_maker.make(command).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn subscribe_student(&self, command: SubscribeStudent) -> Result<()> {
        self.decision_maker.make(command).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn unsubscribe_student(&self, command: UnsubscribeStudent) -> Result<()> {
        self.decision_maker.make(command).await?;
        Ok(())
    }
}
