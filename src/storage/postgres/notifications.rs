use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;
use web_push::{SubscriptionInfo, SubscriptionKeys};

use crate::{devices::DeviceId, errors::AppError, storage::traits::NotificationStore};

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
        did: DeviceId,
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
            did.as_uuid(),
            endpoint.endpoint,
            endpoint.keys.p256dh,
            endpoint.keys.auth
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
    async fn retrive_endpoints(
        &self,
        dids: &[DeviceId],
    ) -> Result<Vec<(SubscriptionInfo, DeviceId)>, AppError> {
        let uuid_dids: Vec<_> = dids.iter().map(|d| d.as_uuid()).collect();
        let rows = sqlx::query!(
            r#"
            SELECT did, endpoint, p256dh, auth from notifications WHERE did = ANY($1)
            "#,
            &uuid_dids,
        )
        .fetch_all(&self.pool)
        .await?;

        let endpoints: Vec<(SubscriptionInfo, DeviceId)> = rows
            .into_iter()
            .map(|row| {
                (
                    SubscriptionInfo {
                        endpoint: row.endpoint,
                        keys: SubscriptionKeys {
                            p256dh: row.p256dh,
                            auth: row.auth,
                        },
                    },
                    DeviceId::new(row.did),
                )
            })
            .collect();

        Ok(endpoints)
    }

    async fn delete_endpoint(&self, did: DeviceId) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            DELETE FROM notifications WHERE did = $1
            "#,
            did.as_uuid(),
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
