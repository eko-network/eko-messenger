use anyhow::Context;
use async_trait::async_trait;
use eko_messenger::{
    devices::DeviceId,
    errors::AppError,
    storage::{DeviceStore, models::StoredDevice},
};
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

pub fn pg_init(url: &str) -> anyhow::Result<sqlx::Pool<Postgres>> {
    let pool = PgPool::connect_lazy(url).context("Failed to connect to Postgres")?;
    Ok(pool)
}

pub struct Storage {
    spool: PgPool,
}

impl Storage {
    pub fn new(spool: PgPool) -> Self {
        Self { spool }
    }
}

#[async_trait]
impl DeviceStore for Storage {
    async fn list_devices_for_user(&self, uid: &str) -> Result<Vec<StoredDevice>, AppError> {
        let uid =
            Uuid::parse_str(uid).map_err(|e| AppError::BadRequest(format!("invalid uid: {e}")))?;
        let rows = sqlx::query!(
            r#"
            SELECT id, signer_public_key FROM public.devices
            WHERE uid = $1
            "#,
            uid
        )
        .fetch_all(&self.spool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|v| StoredDevice {
                did: DeviceId::new(v.id),
                public_key: v.signer_public_key,
            })
            .collect())
    }
}
