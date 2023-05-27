use async_trait::async_trait;

use crate::{
    application::{self, Application},
    domain::DomainEvent,
    proto,
};

#[derive(Clone)]
pub struct CourseApi<R>
where
    R: disintegrate::StateStore<DomainEvent>,
{
    app: Application<R>,
}

impl<R> CourseApi<R>
where
    R: disintegrate::StateStore<DomainEvent>,
{
    pub fn new(app: Application<R>) -> Self {
        Self { app }
    }
}

#[async_trait]
impl<R> proto::course_server::Course for CourseApi<R>
where
    R: disintegrate::StateStore<DomainEvent> + Send + Sync + 'static,
{
    async fn create(
        &self,
        request: tonic::Request<proto::CreateCourseRequest>,
    ) -> Result<tonic::Response<proto::CreateCourseResponse>, tonic::Status> {
        let request = request.into_inner();

        self.app
            .create_course(application::CreateCourse {
                course_id: request.course_id,
                name: request.name,
                seats: request.seats,
            })
            .await
            .map(|_| tonic::Response::new(proto::CreateCourseResponse {}))
            .map_err(|e| tonic::Status::internal(e.to_string()))
    }

    async fn close(
        &self,
        request: tonic::Request<proto::CloseCourseRequest>,
    ) -> Result<tonic::Response<proto::CloseCourseResponse>, tonic::Status> {
        let request = request.into_inner();
        self.app
            .close_course(application::CloseCourse {
                course_id: request.course_id,
            })
            .await
            .map(|_| tonic::Response::new(proto::CloseCourseResponse {}))
            .map_err(|e| tonic::Status::internal(e.to_string()))
    }

    async fn rename(
        &self,
        request: tonic::Request<proto::RenameCourseRequest>,
    ) -> Result<tonic::Response<proto::RenameCourseResponse>, tonic::Status> {
        let request = request.into_inner();

        self.app
            .rename_course(application::RenameCourse {
                course_id: request.course_id,
                name: request.name,
            })
            .await
            .map(|_| tonic::Response::new(proto::RenameCourseResponse {}))
            .map_err(|e| tonic::Status::internal(e.to_string()))
    }

    async fn get(
        &self,
        request: tonic::Request<proto::GetCourseRequest>,
    ) -> Result<tonic::Response<proto::GetCourseResponse>, tonic::Status> {
        let request = request.into_inner();
        self.app
            .course_by_id(request.course_id)
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?
            .map(|c| {
                tonic::Response::new(proto::GetCourseResponse {
                    course_id: c.course_id,
                    name: c.name,
                    available_seats: c.available_seats,
                    closed: c.closed,
                })
            })
            .ok_or(tonic::Status::not_found("course not found"))
    }
}

#[derive(Clone)]
pub struct StudentApi<R>
where
    R: disintegrate::StateStore<DomainEvent>,
{
    app: Application<R>,
}

impl<R> StudentApi<R>
where
    R: disintegrate::StateStore<DomainEvent>,
{
    pub fn new(app: Application<R>) -> Self {
        Self { app }
    }
}

#[async_trait]
impl<R> proto::student_server::Student for StudentApi<R>
where
    R: disintegrate::StateStore<DomainEvent> + Send + Sync + 'static,
{
    async fn register(
        &self,
        request: tonic::Request<proto::RegisterStudentRequest>,
    ) -> Result<tonic::Response<proto::RegisterStudentResponse>, tonic::Status> {
        let request = request.into_inner();
        self.app
            .register_student(application::RegisterStudent {
                student_id: request.student_id,
                name: request.name,
            })
            .await
            .map(|_| tonic::Response::new(proto::RegisterStudentResponse {}))
            .map_err(|e| tonic::Status::internal(e.to_string()))
    }
}

#[derive(Clone)]
pub struct SubscriptionApi<R>
where
    R: disintegrate::StateStore<DomainEvent>,
{
    app: Application<R>,
}

impl<R> SubscriptionApi<R>
where
    R: disintegrate::StateStore<DomainEvent>,
{
    pub fn new(app: Application<R>) -> Self {
        Self { app }
    }
}

#[async_trait]
impl<R> proto::subscription_server::Subscription for SubscriptionApi<R>
where
    R: disintegrate::StateStore<DomainEvent> + Send + Sync + 'static,
{
    async fn subscribe(
        &self,
        request: tonic::Request<proto::SubscribeStudentRequest>,
    ) -> Result<tonic::Response<proto::SubscribeStudentResponse>, tonic::Status> {
        let request = request.into_inner();
        self.app
            .subscribe_student(application::SubscribeStudent {
                course_id: request.course_id,
                student_id: request.student_id,
            })
            .await
            .map(|_| tonic::Response::new(proto::SubscribeStudentResponse {}))
            .map_err(|e| tonic::Status::internal(e.to_string()))
    }
    async fn unsubscribe(
        &self,
        request: tonic::Request<proto::UnsubscribeStudentRequest>,
    ) -> Result<tonic::Response<proto::UnsubscribeStudentResponse>, tonic::Status> {
        let request = request.into_inner();
        self.app
            .unsubscribe_student(application::UnsubscribeStudent {
                course_id: request.course_id,
                student_id: request.student_id,
            })
            .await
            .map(|_| tonic::Response::new(proto::UnsubscribeStudentResponse {}))
            .map_err(|e| tonic::Status::internal(e.to_string()))
    }
}
