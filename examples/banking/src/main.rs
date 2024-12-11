mod domain;

use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    post,
    web::{Data, Json, Path},
    App, HttpResponse, HttpServer, Result,
};

use disintegrate::WithSnapshot;
use disintegrate_postgres::{PgDecisionMaker, PgEventStore, PgSnapshotter, WithPgSnapshot};
use domain::DomainEvent;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgConnectOptions, PgPool};

use crate::domain::{CloseAccount, DepositAmount, OpenAccount, SendMoney, WithdrawAmount};

type DecisionMaker =
    PgDecisionMaker<DomainEvent, disintegrate::serde::json::Json<DomainEvent>, WithPgSnapshot>;
#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub struct Error {
    #[from]
    source: disintegrate::DecisionError<crate::domain::Error>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().unwrap();

    let connect_options = PgConnectOptions::new();
    let pool = PgPool::connect_with(connect_options).await?;

    let serde = disintegrate::serde::json::Json::<DomainEvent>::default();
    let event_store = PgEventStore::new_uninitialized(pool.clone(), serde);
    let snapshotter = PgSnapshotter::new(pool, 10).await?;
    let decision_maker =
        disintegrate_postgres::decision_maker(event_store, WithSnapshot::new(snapshotter));

    Ok(HttpServer::new(move || {
        App::new()
            .app_data(Data::new(decision_maker.clone()))
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
    decision_maker: Data<DecisionMaker>,
    id: Path<i64>,
) -> Result<&'static str, Error> {
    decision_maker.make(OpenAccount::new(*id)).await?;
    Ok("success!")
}

#[post("/account/{id}/deposit")]
async fn deposit(
    decision_maker: Data<DecisionMaker>,
    id: Path<i64>,
    data: Json<Amount>,
) -> Result<&'static str, Error> {
    decision_maker
        .make(DepositAmount::new(*id, data.amount))
        .await?;
    Ok("success!")
}

#[post("/account/{id}/withdraw")]
async fn withdraw(
    decision_maker: Data<DecisionMaker>,
    id: Path<i64>,
    data: Json<Amount>,
) -> Result<&'static str, Error> {
    decision_maker
        .make(WithdrawAmount::new(*id, data.amount))
        .await?;
    Ok("success!")
}

#[post("/account/{id}/close")]
async fn close_account(
    decision_maker: Data<DecisionMaker>,
    id: Path<i64>,
) -> Result<&'static str, Error> {
    decision_maker.make(CloseAccount::new(*id)).await?;
    Ok("success!")
}

#[post("account/{id}/transfer/{beneficiary_id}")]
async fn transfer(
    decision_maker: Data<DecisionMaker>,
    accounts: Path<(i64, i64)>,
    data: Json<Amount>,
) -> Result<&'static str, Error> {
    decision_maker
        .make(SendMoney::new(accounts.0, accounts.1, data.amount))
        .await?;
    Ok("success!")
}

impl error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match self.source {
            disintegrate::DecisionError::Domain(_) => StatusCode::BAD_REQUEST,
            disintegrate::DecisionError::EventStore(_) => StatusCode::INTERNAL_SERVER_ERROR,
            disintegrate::DecisionError::StateStore(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
