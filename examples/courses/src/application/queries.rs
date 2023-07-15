use super::Application;
use crate::domain::CourseId;
use crate::read_model;
use anyhow::Result;

impl<ES> Application<ES> {
    pub async fn course_by_id(&self, course_id: CourseId) -> Result<Option<read_model::Course>> {
        println!("get course id {}", course_id);
        Ok(self.read_model.course_by_id(course_id).await?)
    }
}
