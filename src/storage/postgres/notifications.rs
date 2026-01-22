use async_trait::async_trait;
use sqlx::PgPool;
use web_push::{SubscriptionInfo, SubscriptionKeys};

use crate::{errors::AppError, storage::traits::NotificationStore};

pub struct PostgresNotificationStore {
    pool: PgPool,
}

impl PostgresNotificationStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
#[async_trait]
impl NotificationStore for PostgresNotificationStore {
    async fn upsert_endpoint(
        &self,
        did: &str,
        endpoint: &SubscriptionInfo,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO notifications (did, endpoint, p256dh, auth)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (did) 
            DO UPDATE SET
                endpoint = EXCLUDED.endpoint, p256dh = EXCLUDED.p256dh, auth = EXCLUDED.auth
            "#,
            did,
            endpoint.endpoint,
            endpoint.keys.p256dh,
            endpoint.keys.auth
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
    async fn retrive_endpoints(&self, dids: &[String]) -> Result<Vec<SubscriptionInfo>, AppError> {
        let rows = sqlx::query!(
            r#"
            SELECT endpoint, p256dh, auth from notifications WHERE did = ANY($1)
            "#,
            dids,
        )
        .fetch_all(&self.pool)
        .await?;

        let endpoints: Vec<SubscriptionInfo> = rows
            .into_iter()
            .map(|row| SubscriptionInfo {
                endpoint: row.endpoint,
                keys: SubscriptionKeys {
                    p256dh: row.p256dh,
                    auth: row.auth,
                },
            })
            .collect();

        Ok(endpoints)
    }
}
