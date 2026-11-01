use anyhow::Context;
use async_trait::async_trait;
use eko_messenger::{
    Activity, ActivityStore, Create,
    devices::DeviceId,
    errors::AppError,
    mls::ContentType,
    storage::{
        DeviceStore,
        models::{DeviceNotificationInfo, GroupMessage, StoredDevice},
        traits::NotificationStore,
    },
    types::{Delivered, objects::ObjectBase},
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
            WHERE uid = $1 AND approved_at IS NOT NULL
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
    async fn device_chain_for_user(&self, uid: &str) -> Result<Vec<serde_json::Value>, AppError> {
        let uid =
            Uuid::parse_str(uid).map_err(|e| AppError::BadRequest(format!("invalid uid: {e}")))?;
        let entries = sqlx::query_scalar(
            r#"
            SELECT entry FROM public.device_chain
            WHERE uid = $1
            ORDER BY seq
            "#,
        )
        .bind(uid)
        .fetch_all(&self.spool)
        .await?;
        Ok(entries)
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
    async fn inbox_activities(&self, did: DeviceId) -> Result<Vec<Activity>, AppError> {
        let mut tx = self.pool.begin().await?;

        let rows = sqlx::query!(
            r#"
            SELECT messages.id AS message_id, activity_type, activity
            FROM messages
            JOIN entries ON messages.id = entries.message_id
            WHERE entries.did = $1
            ORDER BY messages.created_at ASC
            "#,
            did.as_uuid()
        )
        .fetch_all(&mut *tx)
        .await?;

        let mut activities = Vec::new();
        for row in rows {
            let activity: Activity = match row.activity_type.as_str() {
                "Create" => Activity::Create(serde_json::from_value(row.activity)?),
                "Delivered" => {
                    sqlx::query!(
                        r#"
                        DELETE FROM entries
                        WHERE message_id = $1 AND did = $2
                        "#,
                        row.message_id,
                        did.as_uuid()
                    )
                    .execute(&mut *tx)
                    .await?;

                    Activity::Delivered(serde_json::from_value(row.activity)?)
                }
                _ => {
                    return Err(AppError::BadRequest(format!(
                        "Unknown activity type: {}",
                        row.activity_type
                    )));
                }
            };
            activities.push(activity);
        }

        tx.commit().await?;
        Ok(activities)
    }

    async fn insert_create(
        &self,
        create: &Create,
        devices: &[DeviceId],
        group: Option<&GroupMessage<'_>>,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        if let Some(group) = group {
            let framing = group.framing;
            if framing.content_type == ContentType::Commit {
                // locks the row until the commit is stored, so the next epoch's commit, and
                // whatever is sent in it, lands after this one in every inbox
                let advanced = sqlx::query(
                    r#"
                    UPDATE group_epochs
                    SET epoch = epoch + 1, claimed_by = NULL, claimed_until = NULL
                    WHERE group_id = $1 AND epoch = $2 AND claimed_by = $3
                    "#,
                )
                .bind(&framing.group_id)
                .bind(framing.epoch as i64)
                .bind(group.sender_did.as_uuid())
                .execute(&mut *tx)
                .await?;
                if advanced.rows_affected() == 0 {
                    return Err(AppError::Conflict(format!(
                        "Epoch {} of the group is not claimed by this device",
                        framing.epoch
                    )));
                }
            } else {
                // a group's first commit adds its members and is never sent, so the group can
                // be ahead of where it was first seen
                sqlx::query(
                    r#"
                    INSERT INTO group_epochs (group_id, epoch)
                    VALUES ($1, $2)
                    ON CONFLICT (group_id) DO UPDATE
                    SET epoch = EXCLUDED.epoch, claimed_by = NULL, claimed_until = NULL
                    WHERE EXCLUDED.epoch > group_epochs.epoch
                    "#,
                )
                .bind(&framing.group_id)
                .bind(framing.epoch as i64)
                .execute(&mut *tx)
                .await?;
            }
        }

        let message_id: i32 = sqlx::query_scalar!(
            r#"
            INSERT INTO messages (activity_type, activity, object_id)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
            "Create",
            serde_json::to_value(create).map_err(AppError::from)?,
            create.object.id()
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
    async fn claim_epoch(
        &self,
        group_id: &[u8],
        epoch: u64,
        did: DeviceId,
    ) -> Result<(), AppError> {
        let claimed = sqlx::query(
            r#"
            INSERT INTO group_epochs (group_id, epoch, claimed_by, claimed_until)
            VALUES ($1, $2, $3, now() + interval '30 seconds')
            ON CONFLICT (group_id) DO UPDATE
            SET claimed_by = EXCLUDED.claimed_by, claimed_until = EXCLUDED.claimed_until
            WHERE group_epochs.epoch = EXCLUDED.epoch
              AND (group_epochs.claimed_by IS NULL
                OR group_epochs.claimed_by = EXCLUDED.claimed_by
                OR group_epochs.claimed_until < now())
            "#,
        )
        .bind(group_id)
        .bind(epoch as i64)
        .bind(did.as_uuid())
        .execute(&self.pool)
        .await?;
        if claimed.rows_affected() == 1 {
            return Ok(());
        }

        let current: i64 = sqlx::query_scalar("SELECT epoch FROM group_epochs WHERE group_id = $1")
            .bind(group_id)
            .fetch_one(&self.pool)
            .await?;
        Err(AppError::Conflict(if current as u64 != epoch {
            format!("The group is at epoch {current}, not {epoch}")
        } else {
            format!("Epoch {epoch} is claimed by another device")
        }))
    }

    async fn release_epoch(&self, group_id: &[u8], did: DeviceId) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE group_epochs SET claimed_by = NULL, claimed_until = NULL
            WHERE group_id = $1 AND claimed_by = $2
            "#,
        )
        .bind(group_id)
        .bind(did.as_uuid())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_inbox_entry(&self, object_id: &str, did: DeviceId) -> Result<(), AppError> {
        sqlx::query(
            r#"
            DELETE FROM entries
            USING messages
            WHERE entries.message_id = messages.id
              AND messages.object_id = $1
              AND entries.did = $2
            "#,
        )
        .bind(object_id)
        .bind(did.as_uuid())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn insert_delivered(
        &self,
        delivered: &Delivered,
        devices: &[DeviceId],
        device: DeviceId,
    ) -> Result<(), AppError> {
        let notify_dids: Vec<Uuid> = devices.iter().map(|d| d.as_uuid()).collect();

        sqlx::query!(
            r#"
            SELECT public.insert_delivered($1, $2, $3, $4, $5)
            "#,
            device.as_uuid(),
            delivered.object,
            serde_json::to_value(delivered).map_err(AppError::from)?,
            delivered.id.as_deref(),
            &notify_dids
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl NotificationStore for Storage {
    async fn retrive_endpoints(
        &self,
        device_ids: Vec<DeviceId>,
    ) -> Result<Option<Vec<DeviceNotificationInfo>>, AppError> {
        let uuids: Vec<Uuid> = device_ids.iter().map(|d| d.as_uuid()).collect();

        let rows = sqlx::query(
            r#"
            SELECT user_uid, device_id, token, device_type::text AS device_type, notification_type::text AS notification_type, active
            FROM public.notifications
            WHERE device_id = ANY($1) AND active = true
            "#,
        )
        .bind(&uuids)
        .fetch_all(&self.spool)
        .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let endpoints: Vec<DeviceNotificationInfo> = rows
            .into_iter()
            .map(|row| {
                let device_type_str: Option<String> = row.get("device_type");
                let notification_type_str: String = row.get("notification_type");

                Ok(DeviceNotificationInfo {
                    uid: row.get("user_uid"),
                    did: DeviceId::new(row.get("device_id")),
                    token: row.get("token"),
                    device_type: device_type_str
                        .map(|s| {
                            serde_json::from_value(serde_json::Value::String(s))
                                .map_err(AppError::from)
                        })
                        .transpose()?,
                    notification_type: serde_json::from_value(serde_json::Value::String(
                        notification_type_str,
                    ))?,
                    active: row.get("active"),
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(Some(endpoints))
    }

    async fn mark_inactive(&self, did: DeviceId) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE public.notifications
            SET active = false
            WHERE device_id = $1
            "#,
        )
        .bind(did.as_uuid())
        .execute(&self.spool)
        .await?;

        Ok(())
    }
}
