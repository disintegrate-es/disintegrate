use super::Application;
use crate::domain::CourseId;
use crate::read_model;
use anyhow::Result;
use tracing::instrument;

impl Application {
    #[instrument(skip(self))]
    pub async fn course_by_id(&self, course_id: CourseId) -> Result<Option<read_model::Course>> {
        Ok(self.read_model.course_by_id(course_id).await?)
    }
}
