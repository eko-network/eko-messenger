use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;
use sqlx::PgPool;

use crate::activitypub::types::activity::ActivityType;
use crate::activitypub::{Activity, Create, EncryptedMessageEntry};
use crate::devices::DeviceId;
use crate::errors::AppError;
use crate::storage::traits::ActivityStore;

#[derive(Serialize)]
struct CreateStorageView<'a> {
    #[serde(rename = "@context")]
    context: &'a serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<&'a str>,
    actor: &'a str,
    object: EncryptedMessageStorageView<'a>,
    to: &'a str,
    #[serde(rename = "type")]
    type_field: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EncryptedMessageStorageView<'a> {
    #[serde(rename = "@context")]
    context: &'a serde_json::Value,
    #[serde(rename = "type")]
    type_field: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<&'a str>,
    content: &'a [EncryptedMessageEntry],
    attributed_to: &'a str,
    to: &'a [String],
}

pub struct PostgresActivityStore {
    pool: PgPool,
    domain: Arc<String>,
}

impl PostgresActivityStore {
    pub fn new(domain: Arc<String>, pool: PgPool) -> Self {
        Self { domain, pool }
    }
}

#[async_trait]
impl ActivityStore for PostgresActivityStore {
    async fn inbox_activities(&self, did: DeviceId) -> Result<Vec<Activity>, AppError> {
        // Fetch all activities that have a delivery request for this device
        let rows = sqlx::query!(
            r#"
            SELECT 
                ia.id,
                ia.type::text as "activity_type!",
                ia.activity_json,
                me.from_did as "from_did?",
                me.content as "content?"
            FROM inbox_activities ia
            JOIN deliveries d ON ia.id = d.activity_id
            LEFT JOIN message_entries me ON ia.id = me.activity_id AND me.to_did = d.to_did
            WHERE d.to_did = $1
            ORDER BY ia.created_at ASC
            "#,
            did.as_uuid()
        )
        .fetch_all(&self.pool)
        .await?;

        let mut activities = Vec::new();
        let did_url = did.to_url(&self.domain);
        for row in rows {
            let mut activity: Activity = serde_json::from_value(row.activity_json)?;

            match activity {
                Activity::Create(ref mut create) => {
                    if let (Some(from_did), Some(content)) = (&row.from_did, &row.content) {
                        create.object.content = vec![crate::activitypub::EncryptedMessageEntry {
                            from: from_did.clone(),
                            to: did_url.clone(),
                            content: content.clone(),
                        }];
                    }
                }
                Activity::Take(_) | Activity::Delivered(_) => {
                    if let Some(id) = activity.as_base().id() {
                        //TODO run these all at one time
                        self.delete_delivery(id, &did).await?;
                    }
                }
            };

            activities.push(activity);
        }

        Ok(activities)
    }

    async fn insert_create(&self, create: &Create) -> Result<(), AppError> {
        let activity_id = create
            .id
            .as_ref()
            .ok_or_else(|| AppError::BadRequest("Create activity must have an id".to_string()))?;

        // Serialize with empty content array - entries are stored separately in message_entries table
        let empty_content: &[EncryptedMessageEntry] = &[];
        let activity_view = CreateStorageView {
            context: &create.context,
            id: create.id.as_deref(),
            actor: &create.actor,
            object: EncryptedMessageStorageView {
                context: &create.object.context,
                type_field: &create.object.type_field,
                id: create.object.id.as_deref(),
                content: empty_content,
                attributed_to: &create.object.attributed_to,
                to: &create.object.to,
            },
            to: &create.to,
            type_field: "Create",
        };
        let activity_json = serde_json::to_value(&activity_view)?;

        let mut tx = self.pool.begin().await?;

        // Insert the activity
        sqlx::query!(
            r#"
            INSERT INTO inbox_activities (id, type, activity_json)
            VALUES ($1, 'Create', $2)
            "#,
            activity_id,
            activity_json
        )
        .execute(&mut *tx)
        .await?;

        // Insert message entries and delivery requests for each device
        for entry in &create.object.content {
            let to_did = DeviceId::from_url(&entry.to)?;

            // Insert message entry
            sqlx::query!(
                r#"
                INSERT INTO message_entries (from_did, to_did, activity_id, content)
                VALUES ($1, $2, $3, $4)
                "#,
                &entry.from,
                to_did.as_uuid(),
                activity_id,
                &entry.content
            )
            .execute(&mut *tx)
            .await?;

            // Insert delivery request
            sqlx::query!(
                r#"
                INSERT INTO deliveries (activity_id, to_did)
                VALUES ($1, $2)
                "#,
                activity_id,
                to_did.as_uuid()
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn insert_non_create(
        &self,
        activity: &Activity,
        dids: &Vec<DeviceId>,
    ) -> Result<(), AppError> {
        let activity_id = activity
            .as_base()
            .id()
            .ok_or_else(|| AppError::BadRequest("Activity must have an id".to_string()))?;

        let activity_json = serde_json::to_value(activity)?;

        // Start a transaction
        let mut tx = self.pool.begin().await?;

        let activity_type: ActivityType = activity.activity_type();
        // Insert the activity
        sqlx::query!(
            r#"
            INSERT INTO inbox_activities (id, type, activity_json)
            VALUES ($1, $2, $3)
            "#,
            activity_id,
            activity_type as ActivityType,
            activity_json
        )
        .execute(&mut *tx)
        .await?;

        // Insert delivery requests for each device
        for did in dids {
            sqlx::query!(
                r#"
                INSERT INTO deliveries (activity_id, to_did)
                VALUES ($1, $2)
                "#,
                activity_id,
                did.as_uuid()
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn delete_delivery(&self, activity_id: &str, did: &DeviceId) -> Result<bool, AppError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM deliveries
            WHERE activity_id = $1 AND to_did = $2
            "#,
            activity_id,
            did.as_uuid()
        )
        .execute(&self.pool)
        .await?;

        // Return true if a row was deleted, false if not found
        Ok(result.rows_affected() > 0)
    }

    async fn claim_first_delivery(&self, create_id: &str) -> Result<bool, AppError> {
        // Atomically check if this is the first delivery and set the timestamp if so
        // Returns true if first_delivery_at was NULL (this is the first delivery)
        let result = sqlx::query!(
            r#"
            UPDATE inbox_activities
            SET first_delivery_at = NOW()
            WHERE id = $1 AND first_delivery_at IS NULL
            RETURNING id
            "#,
            create_id
        )
        .fetch_optional(&self.pool)
        .await?;

        // If a row was updated, this was the first delivery
        Ok(result.is_some())
    }
}
