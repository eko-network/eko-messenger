use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    activitypub::PreKeyBundle,
    auth::{PreKey, SignedPreKey},
    errors::AppError,
    storage::{
        models::{RegisterDeviceResult, RotatedRefreshToken},
        traits::DeviceStore,
    },
};

pub struct PostgresDeviceStore {
    pool: PgPool,
}

impl PostgresDeviceStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DeviceStore for PostgresDeviceStore {
    async fn key_bundles_for_user(&self, uid: &str) -> Result<Vec<PreKeyBundle>, AppError> {
        let mut tx = self.pool.begin().await?;

        let devices = sqlx::query!(
            "SELECT did, identity_key, registration_id FROM devices WHERE uid = $1",
            uid
        )
        .fetch_all(&mut *tx)
        .await?;

        let mut bundles = Vec::new();

        for device in devices {
            let pre_key = sqlx::query!(
                r#"
                DELETE FROM pre_keys
                WHERE ctid = (
                    SELECT ctid
                    FROM pre_keys
                    WHERE did = $1
                    LIMIT 1
                )
                RETURNING key_id, key
                "#,
                device.did
            )
            .fetch_optional(&mut *tx)
            .await?;

            let signed_pre_key = sqlx::query!(
                "SELECT key_id, key, signature FROM signed_pre_keys WHERE did = $1",
                device.did
            )
            .fetch_one(&mut *tx)
            .await?;

            if let Some(pre_key) = pre_key {
                bundles.push(PreKeyBundle {
                    did: device.did,
                    identity_key: device.identity_key.clone(),
                    registration_id: device.registration_id,
                    pre_key_id: pre_key.key_id,
                    pre_key: pre_key.key,
                    signed_pre_key_id: signed_pre_key.key_id,
                    signed_pre_key: signed_pre_key.key,
                    signed_pre_key_signature: signed_pre_key.signature,
                });
            }
        }

        tx.commit().await?;
        Ok(bundles)
    }
    async fn register_device(
        &self,
        uid: &str,
        device_name: &str,
        identity_key: &[u8],
        registration_id: i32,
        pre_keys: &[PreKey],
        signed_pre_key: &SignedPreKey,
        ip_address: &str,
        user_agent: &str,
        expires_at: time::OffsetDateTime,
    ) -> Result<RegisterDeviceResult, AppError> {
        let mut tx = self.pool.begin().await?;

        let did = sqlx::query!(
            r#"
            INSERT INTO devices (uid, name, identity_key, registration_id)
            VALUES ($1, $2, $3, $4)
            RETURNING did
            "#,
            uid,
            device_name,
            identity_key,
            registration_id
        )
        .fetch_one(&mut *tx)
        .await?
        .did;

        let refresh_token = sqlx::query!(
            r#"
            INSERT INTO refresh_tokens (did, ip_address, user_agent, expires_at)
            VALUES ($1, $2, $3, $4)
            RETURNING token
            "#,
            did,
            ip_address,
            user_agent,
            expires_at
        )
        .fetch_one(&mut *tx)
        .await?
        .token;

        for pre_key in pre_keys {
            sqlx::query!(
                "INSERT INTO pre_keys (did, key_id, key) VALUES ($1, $2, $3)",
                did,
                pre_key.id,
                pre_key.key
            )
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query!(
            r#"
            INSERT INTO signed_pre_keys (did, key_id, key, signature)
            VALUES ($1, $2, $3, $4)
            "#,
            did,
            signed_pre_key.id,
            signed_pre_key.key,
            signed_pre_key.signature
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(RegisterDeviceResult { did, refresh_token })
    }

    async fn rotate_refresh_token(
        &self,
        old_token: &Uuid,
        ip_address: &str,
        user_agent: &str,
    ) -> Result<Option<RotatedRefreshToken>, AppError> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query!(
            r#"
            SELECT d.did, d.uid, r.user_agent, r.expires_at
            FROM refresh_tokens r
            JOIN devices d ON d.did = r.did
            WHERE r.token = $1
            "#,
            old_token
        )
        .fetch_optional(&mut *tx)
        .await?;

        let row = match row {
            Some(r) => r,
            None => {
                tx.commit().await?;
                return Ok(None);
            }
        };

        if row.expires_at <= time::OffsetDateTime::now_utc() || row.user_agent != user_agent {
            tx.commit().await?;
            return Ok(None);
        }

        sqlx::query!("DELETE FROM refresh_tokens WHERE did = $1", row.did)
            .execute(&mut *tx)
            .await?;

        let expires_at = time::OffsetDateTime::now_utc()
            + time::Duration::seconds(crate::auth::REFRESH_EXPIRATION);

        let new_token = sqlx::query!(
            r#"
            INSERT INTO refresh_tokens (did, ip_address, user_agent, expires_at)
            VALUES ($1, $2, $3, $4)
            RETURNING token
            "#,
            row.did,
            ip_address,
            user_agent,
            expires_at
        )
        .fetch_one(&mut *tx)
        .await?
        .token;

        tx.commit().await?;
        Ok(Some(RotatedRefreshToken {
            refresh_token: new_token,
            uid: row.uid,
            did: row.did,
            expires_at,
        }))
    }

    async fn logout_device(&self, refresh_token: &Uuid) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            DELETE FROM devices
            WHERE did = (SELECT did FROM refresh_tokens WHERE token = $1)
            "#,
            refresh_token
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_dids_for_user(&self, uid: &str) -> Result<Vec<i32>, AppError> {
        let dids = sqlx::query!("SELECT did FROM devices WHERE uid = $1", uid)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|r| r.did)
            .collect();

        Ok(dids)
    }
}
