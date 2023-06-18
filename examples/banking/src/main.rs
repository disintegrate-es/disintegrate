mod application;
mod domain;

use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    post,
    web::{Data, Json, Path},
    App, HttpResponse, HttpServer, Result,
};
use application::Application;
use disintegrate_postgres::PgEventStore;
use domain::DomainEvent;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgConnectOptions, PgPool};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().unwrap();

    let connect_options = PgConnectOptions::new();
    let pool = PgPool::connect_with(connect_options).await?;

    let serde = disintegrate::serde::json::Json::<DomainEvent>::default();

    let event_store = PgEventStore::new(pool, serde).await?;

    let application = Application::new(event_store);

    Ok(HttpServer::new(move || {
        App::new()
            .app_data(Data::new(application.clone()))
            .service(open_account)
            .service(close_account)
            .service(deposit)
            .service(withdraw)
            .service(transfer)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?)
}

#[derive(Serialize, Deserialize)]
struct Amount {
    amount: i32,
}

#[post("/account/{id}/open")]
async fn open_account(
    app: Data<Application>,
    id: Path<String>,
    data: Json<Amount>,
) -> Result<&'static str, application::Error> {
    app.open_account(id.to_string(), data.amount).await?;
    Ok("success!")
}

#[post("/account/{id}/deposit")]
async fn deposit(
    app: Data<Application>,
    id: Path<String>,
    data: Json<Amount>,
) -> Result<&'static str, application::Error> {
    app.deposit_amount(id.to_string(), data.amount).await?;
    Ok("success!")
}

#[post("/account/{id}/withdraw")]
async fn withdraw(
    app: Data<Application>,
    id: Path<String>,
    data: Json<Amount>,
) -> Result<&'static str, application::Error> {
    app.withdraw_amount(id.to_string(), data.amount).await?;
    Ok("success!")
}

#[post("/account/{id}/close")]
async fn close_account(
    app: Data<Application>,
    id: Path<String>,
) -> Result<&'static str, application::Error> {
    app.close_account(id.to_string()).await?;
    Ok("success!")
}

#[post("account/{id}/transfer/{beneficiary_id}")]
async fn transfer(
    app: Data<Application>,
    accounts: Path<(String, String)>,
    data: Json<Amount>,
) -> Result<&'static str, application::Error> {
    app.transfer_amount(accounts.0.to_string(), accounts.1.to_string(), data.amount)
        .await?;
    Ok("success!")
}

impl error::ResponseError for application::Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            application::Error::Domain(_) => StatusCode::BAD_REQUEST,
            application::Error::EventStore(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
