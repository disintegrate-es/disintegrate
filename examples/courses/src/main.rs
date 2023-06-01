use std::time::Duration;

use anyhow::{anyhow, Ok, Result};
use disintegrate::serde::prost::Prost;
use disintegrate_postgres::{PgEventListener, PgEventListenerConfig, PgEventStore};
use tokio::signal;
use tower_http::trace::TraceLayer;

use courses::{application, domain::DomainEvent, grpc, proto, read_model};

type EventStore = PgEventStore<DomainEvent, Prost<DomainEvent, proto::Event>>;
type Application = courses::application::Application<EventStore>;

#[tokio::main]
async fn main() -> Result<()> {
    let pool = courses::postgres::connect().await?;
    let serde = Prost::<DomainEvent, proto::Event>::default();

    let event_store = PgEventStore::new(pool.clone(), serde).await?;
    let read_model = read_model::Repository::new(pool.clone());
    let app = application::Application::new(event_store.clone(), read_model);

    tokio::try_join!(grpc_server(app), event_listener(pool, event_store))?;
    Ok(())
}

async fn grpc_server(app: Application) -> Result<()> {
    let addr = "0.0.0.0:10437"
        .parse()
        .map_err(|e| anyhow!("failed to parse grpc address: {}", e))?;

    let (_, health_svc) = tonic_health::server::health_reporter();

    let reflection_svc = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(tonic_health::pb::FILE_DESCRIPTOR_SET)
        .build()
        .map_err(|e| anyhow!("failed to build grpc reflection service: {}", e))?;

    let course_svc = proto::course_server::CourseServer::new(grpc::CourseApi::new(app.clone()));

    let student_svc = proto::student_server::StudentServer::new(grpc::StudentApi::new(app.clone()));

    let subscription_svc =
        proto::subscription_server::SubscriptionServer::new(grpc::SubscriptionApi::new(app));

    let layer = tower::ServiceBuilder::new()
        .layer(TraceLayer::new_for_grpc())
        .timeout(Duration::from_secs(5))
        .into_inner();

    tonic::transport::Server::builder()
        .layer(layer)
        .add_service(health_svc)
        .add_service(reflection_svc)
        .add_service(course_svc)
        .add_service(student_svc)
        .add_service(subscription_svc)
        .serve_with_shutdown(addr, shutdown())
        .await
        .map_err(|e| anyhow!("tonic server exited with error: {}", e))?;
    Ok(())
}

async fn event_listener(pool: sqlx::PgPool, event_store: EventStore) -> Result<()> {
    PgEventListener::builder(event_store)
        .register_listener(
            read_model::ReadModelProjection::new(pool.clone()).await?,
            PgEventListenerConfig::poller(Duration::from_millis(50)),
        )
        .start_with_shutdown(shutdown())
        .await
        .map_err(|e| anyhow!("event listener exited with error: {}", e))?;
    Ok(())
}

async fn shutdown() {
    signal::ctrl_c().await.expect("failed to listen for event");
}
