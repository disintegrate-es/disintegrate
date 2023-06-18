use crate::domain::{CourseId, DomainEvent};
use async_trait::async_trait;
use disintegrate::{query, EventListener, PersistedEvent, StreamQuery};
use sqlx::{FromRow, PgPool};

#[derive(Clone)]
pub struct Repository {
    pool: PgPool,
}

impl Repository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    pub async fn course_by_id(&self, course_id: CourseId) -> Result<Option<Course>, sqlx::Error> {
        sqlx::query_as::<_, Course>(
            "SELECT course_id, name, available_seats, closed FROM course WHERE course_id = $1",
        )
        .bind(course_id)
        .fetch_optional(&self.pool)
        .await
    }
}

#[derive(FromRow)]
pub struct Course {
    pub course_id: String,
    pub name: String,
    pub available_seats: i32,
    pub closed: bool,
}

pub struct ReadModelProjection {
    query: StreamQuery<DomainEvent>,
    pool: PgPool,
}

impl ReadModelProjection {
    pub async fn new(pool: PgPool) -> Result<Self, sqlx::Error> {
        sqlx::query(
            r#"
        CREATE TABLE IF NOT EXISTS course (
           course_id TEXT PRIMARY KEY,
           name TEXT,
           available_seats INT,
           closed BOOL DEFAULT false
        )"#,
        )
        .execute(&pool)
        .await?;
        Ok(Self {
            query: query(None),
            pool,
        })
    }
}

#[async_trait]
impl EventListener<DomainEvent> for ReadModelProjection {
    type Error = sqlx::Error;
    fn id(&self) -> &'static str {
        "courses"
    }

    fn query(&self) -> &StreamQuery<DomainEvent> {
        &self.query
    }

    async fn handle(&self, event: PersistedEvent<DomainEvent>) -> Result<(), Self::Error> {
        match event.into_inner() {
            DomainEvent::CourseCreated {
                course_id,
                name,
                seats,
            } => {
                sqlx::query(
                    "INSERT INTO course (course_id, name, available_seats) VALUES($1, $2, $3)",
                )
                .bind(course_id)
                .bind(name)
                .bind(seats as i32)
                .execute(&self.pool)
                .await
                .unwrap();
            }
            DomainEvent::CourseClosed { course_id } => {
                sqlx::query("UPDATE course SET closed = true WHERE course_id = $1")
                    .bind(course_id)
                    .execute(&self.pool)
                    .await
                    .unwrap();
            }
            DomainEvent::StudentSubscribed { course_id, .. } => {
                sqlx::query(
                    "UPDATE course SET available_seats = available_seats - 1 WHERE course_id = $1",
                )
                .bind(course_id)
                .execute(&self.pool)
                .await
                .unwrap();
            }
            DomainEvent::StudentUnsubscribed { course_id, .. } => {
                sqlx::query(
                    "UPDATE course SET available_seats = available_seats + 1 WHERE course_id = $1",
                )
                .bind(course_id)
                .execute(&self.pool)
                .await
                .unwrap();
            }
            DomainEvent::CourseRenamed { course_id, name } => {
                sqlx::query("UPDATE course SET name = $2 WHERE course_id = $1")
                    .bind(course_id)
                    .bind(name)
                    .execute(&self.pool)
                    .await
                    .unwrap();
            }
            _ => {}
        }
        Ok(())
    }
}
