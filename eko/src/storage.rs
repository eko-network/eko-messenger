use anyhow::Context;
use async_trait::async_trait;
use eko_messenger::{
    ActivityStore, Create,
    devices::DeviceId,
    errors::AppError,
    storage::{DeviceStore, models::StoredDevice},
};
use sqlx::{PgPool, Postgres, Row};
use uuid::Uuid;

pub fn pg_init(url: &str) -> anyhow::Result<sqlx::Pool<Postgres>> {
    let pool = PgPool::connect_lazy(url).context("Failed to connect to Postgres")?;
    Ok(pool)
}
pub async fn pg_init_with_migration(url: &str) -> anyhow::Result<sqlx::Pool<Postgres>> {
    let pool = pg_init(url)?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("Failed to run database migrations")?;
    Ok(pool)
}

pub struct Storage {
    spool: PgPool,
    pool: PgPool,
}

impl Storage {
    pub fn new(spool: PgPool, pool: PgPool) -> Self {
        Self { spool, pool }
    }
}

#[async_trait]
impl DeviceStore for Storage {
    async fn list_devices_for_user(&self, uid: &str) -> Result<Vec<StoredDevice>, AppError> {
        let uid =
            Uuid::parse_str(uid).map_err(|e| AppError::BadRequest(format!("invalid uid: {e}")))?;
        let rows = sqlx::query(
            r#"
            SELECT id, signer_public_key FROM public.devices
            WHERE uid = $1
            "#,
        )
        .bind(uid)
        .fetch_all(&self.spool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| StoredDevice {
                did: DeviceId::new(row.get("id")),
                public_key: row.get("signer_public_key"),
            })
            .collect())
    }
    async fn take_key_package(&self, did: DeviceId) -> Result<Vec<u8>, AppError> {
        let result: Option<Vec<u8>> = sqlx::query_scalar(
            r#"
            SELECT public.take_key_package($1)
            "#,
        )
        .bind(did.as_uuid())
        .fetch_one(&self.spool)
        .await?;

        result.ok_or(AppError::NotFound(
            "Device does not exist or no keys available".to_string(),
        ))
    }
}

#[async_trait]
impl ActivityStore for Storage {
    async fn insert_create(&self, create: &Create, devices: &[DeviceId]) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        let message_id: i32 = sqlx::query_scalar!(
            r#"
            INSERT INTO messages (activity_type, object)
            VALUES ($1, $2)
            RETURNING id
            "#,
            "Create",
            serde_json::to_value(create).map_err(AppError::from)?
        )
        .fetch_one(&mut *tx)
        .await?;

        for device in devices {
            sqlx::query!(
                r#"
                INSERT INTO entries (message_id, did)
                VALUES ($1, $2)
                "#,
                message_id,
                device.as_uuid()
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
