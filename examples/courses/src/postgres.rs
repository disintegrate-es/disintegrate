use sqlx::{postgres::PgConnectOptions, PgPool};

pub async fn connect() -> anyhow::Result<PgPool> {
    dotenv::dotenv().unwrap();
    let connect_options = PgConnectOptions::new();
    Ok(PgPool::connect_with(connect_options).await?)
}
