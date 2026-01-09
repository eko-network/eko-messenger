use crate::storage::{Storage, postgres::connection::postgres_storage};
use anyhow::Context;
use sqlx::{PgPool, Postgres};
use std::env::var;
use tracing::info;

pub async fn db_config() -> anyhow::Result<sqlx::Pool<Postgres>> {
    let database_url = var("DATABASE_URL").context("DATABASE_URL not found in environment")?;
    let pool = PgPool::connect_lazy(&database_url).context("Failed to connect to Postgres")?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("Failed to run migrations")?;
    Ok(pool)
}

pub async fn storage_config() -> anyhow::Result<Storage> {
    // default storage choice to postgres
    let storage_backend = var("STORAGE_BACKEND").unwrap_or_else(|_| "postgres".to_string());

    match storage_backend.to_lowercase().as_str() {
        "postgres" => {
            info!("Using PostgreSQL storage backend");
            let pool = db_config().await?;
            Ok(postgres_storage(pool))
        }
        _ => {
            anyhow::bail!(
                "Invalid STORAGE_BACKEND: '{}'. Valid options are 'postgres'",
                storage_backend
            )
        }
    }
}
