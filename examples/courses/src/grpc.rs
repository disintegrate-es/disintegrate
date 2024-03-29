use async_trait::async_trait;

use crate::{
    application::Application,
    domain::{self},
    proto,
};

#[derive(Clone)]
pub struct CourseApi {
    app: Application,
}

impl CourseApi {
    pub fn new(app: Application) -> Self {
        Self { app }
    }
}

#[async_trait]
impl proto::course_server::Course for CourseApi {
    async fn create(
        &self,
        request: tonic::Request<proto::CreateCourseRequest>,
    ) -> Result<tonic::Response<proto::CreateCourseResponse>, tonic::Status> {
        let request = request.into_inner();

        self.app
            .create_course(domain::CreateCourse {
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
            .close_course(domain::CloseCourse {
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
            .rename_course(domain::RenameCourse {
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
pub struct StudentApi {
    app: Application,
}

impl StudentApi {
    pub fn new(app: Application) -> Self {
        Self { app }
    }
}

#[async_trait]
impl proto::student_server::Student for StudentApi {
    async fn register(
        &self,
        request: tonic::Request<proto::RegisterStudentRequest>,
    ) -> Result<tonic::Response<proto::RegisterStudentResponse>, tonic::Status> {
        let request = request.into_inner();
        self.app
            .register_student(domain::RegisterStudent {
                student_id: request.student_id,
                name: request.name,
            })
            .await
            .map(|_| tonic::Response::new(proto::RegisterStudentResponse {}))
            .map_err(|e| tonic::Status::internal(e.to_string()))
    }
}

#[derive(Clone)]
pub struct SubscriptionApi {
    app: Application,
}

impl SubscriptionApi {
    pub fn new(app: Application) -> Self {
        Self { app }
    }
}

#[async_trait]
impl proto::subscription_server::Subscription for SubscriptionApi {
    async fn subscribe(
        &self,
        request: tonic::Request<proto::SubscribeStudentRequest>,
    ) -> Result<tonic::Response<proto::SubscribeStudentResponse>, tonic::Status> {
        let request = request.into_inner();
        self.app
            .subscribe_student(domain::SubscribeStudent {
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
            .unsubscribe_student(domain::UnsubscribeStudent {
                course_id: request.course_id,
                student_id: request.student_id,
            })
            .await
            .map(|_| tonic::Response::new(proto::UnsubscribeStudentResponse {}))
            .map_err(|e| tonic::Status::internal(e.to_string()))
    }
}
