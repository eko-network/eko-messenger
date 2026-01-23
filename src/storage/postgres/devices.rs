use anyhow::anyhow;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    activitypub::types::{
        actor::default_context_value,
        eko_types::{AddDevice, DeviceAction, RevokeDevice},
    },
    auth::{PreKey, SignedPreKey},
    devices::DeviceId,
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
    async fn get_approved_devices(&self, uid: &str) -> Result<Vec<DeviceId>, AppError> {
        Ok(sqlx::query!(
            r#"
            SELECT did FROM devices
            WHERE uid = $1 AND is_approved = TRUE
            "#,
            uid
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|v| DeviceId::new(v.did))
        .collect())
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
        let approved_devices = self.get_approved_devices(uid).await?;

        let tofu = approved_devices.is_empty() || true;

        let did = DeviceId::new(Uuid::new_v4());

        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
            INSERT INTO devices (did, uid)
            VALUES ($1, $2)
            "#,
            did.as_uuid(),
            uid,
        )
        .execute(&mut *tx)
        .await?;

        let refresh_token = sqlx::query!(
            r#"
            INSERT INTO refresh_tokens (did, ip_address, user_agent, expires_at)
            VALUES ($1, $2, $3, $4)
            RETURNING token
            "#,
            did.as_uuid(),
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
                did.as_uuid(),
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
            did.as_uuid(),
            signed_pre_key.id,
            signed_pre_key.key,
            signed_pre_key.signature
        )
        .execute(&mut *tx)
        .await?;

        if tofu {
            sqlx::query!(
                r#"
            INSERT INTO device_actions(is_add, did, uid, identity_key, registration_id, device_name)
            VALUES (TRUE, $1, $2, $3, $4, $5)
            "#,
                did.as_uuid(),
                uid,
                identity_key,
                registration_id,
                device_name
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(RegisterDeviceResult {
            did: did,
            refresh_token,
            approved: tofu,
        })
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
            did: DeviceId::new(row.did),
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

    async fn device_actions_for_user(&self, uid: &str) -> Result<Vec<DeviceAction>, AppError> {
        let dids: Vec<DeviceAction> = sqlx::query!(
            "SELECT did, is_add, prev, registration_id, identity_key FROM device_actions WHERE uid = $1 ORDER BY created_at ASC",
            uid
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|r| {
            let did = DeviceId::new(r.did);
            let global_did = did.to_url();
            let id = did.action_url(r.is_add);
            if r.is_add {
                Ok(DeviceAction::AddDevice(AddDevice {
                    id,
                    context: default_context_value(),
                    prev: r.prev.map(|v| v.try_into()).transpose().map_err(|_| anyhow!("Invalid hash stored"))?,
                    key_collection: did.key_collection_url(),
                    did: global_did,
                    identity_key: r.identity_key.ok_or(anyhow!("identity_key may not be null"))?,
                    registration_id: r.registration_id.ok_or(anyhow!("registration_id may not be null"))?,
                    //TODO
                    proof: vec![],
                }))
            } else {
                Ok(DeviceAction::RevokeDevice(RevokeDevice {
                    id,
                    context: default_context_value(),
                    did: global_did,
                    prev: r.prev.map(|v| v.try_into()).transpose().map_err(|_| anyhow!("Invalid hash stored"))?,
                    //TODO
                    proof: vec![],
                }))
            }
        })
        .collect::<Result<_, AppError>>()?;

        Ok(dids)
    }

    async fn get_device_status(&self, did: DeviceId) -> Result<bool, AppError> {
        let device = sqlx::query!(
            "SELECT is_approved FROM devices WHERE did = $1",
            did.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await?;

        match device {
            Some(d) => Ok(d.is_approved),
            None => Err(AppError::NotFound("Device not found".to_string())),
        }
    }

    async fn get_prekey_bundle(
        &self,
        did: DeviceId,
    ) -> Result<Option<crate::activitypub::types::eko_types::PreKeyBundle>, AppError> {
        use crate::activitypub::types::eko_types::PreKeyBundle;

        let mut tx = self.pool.begin().await?;

        // Count available prekeys for this device
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)
            FROM pre_keys
            WHERE did = $1
            "#,
            did.as_uuid()
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(0);

        if count == 0 {
            tx.commit().await?;
            return Ok(None);
        }

        // Get a prekey - delete if more than one exists, otherwise just select
        let (pre_key_id, pre_key) = if count > 1 {
            let result = sqlx::query!(
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
                did.as_uuid()
            )
            .fetch_one(&mut *tx)
            .await?;
            (result.key_id, result.key)
        } else {
            let result = sqlx::query!(
                r#"
                SELECT key_id, key
                FROM pre_keys
                WHERE did = $1
                LIMIT 1
                "#,
                did.as_uuid()
            )
            .fetch_one(&mut *tx)
            .await?;
            (result.key_id, result.key)
        };

        // Fetch the signed prekey
        let signed_pre_key = sqlx::query!(
            r#"
            SELECT key_id, key, signature
            FROM signed_pre_keys
            WHERE did = $1
            LIMIT 1
            "#,
            did.as_uuid()
        )
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;

        match signed_pre_key {
            Some(spk) => Ok(Some(PreKeyBundle {
                did,
                pre_key_id,
                pre_key,
                signed_pre_key_id: spk.key_id,
                signed_pre_key: spk.key,
                signed_pre_key_signature: spk.signature,
            })),
            None => Ok(None),
        }
    }
}
