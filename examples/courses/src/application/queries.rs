use super::Application;
use crate::domain::{CourseId, DomainEvent};
use crate::read_model;
use anyhow::Result;

impl<S> Application<S>
where
    S: disintegrate::StateStore<DomainEvent>,
{
    pub async fn course_by_id(&self, course_id: CourseId) -> Result<Option<read_model::Course>> {
        println!("get course id {}", course_id);
        Ok(self.read_model.course_by_id(course_id).await?)
    }
}
