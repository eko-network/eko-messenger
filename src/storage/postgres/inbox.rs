use async_trait::async_trait;
use sqlx::PgPool;

use crate::activitypub::{Activity, Create};
use crate::devices::DeviceId;
use crate::errors::AppError;
use crate::storage::traits::InboxStore;

pub struct PostgresInboxStore {
    pool: PgPool,
}

impl PostgresInboxStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InboxStore for PostgresInboxStore {
    async fn inbox_activities(
        &self,
        inbox_actor_id: &str,
        did: DeviceId,
    ) -> Result<Vec<Activity>, AppError> {
        // Fetch all activities that have a delivery request for this device
        let rows = sqlx::query!(
            r#"
            SELECT 
                ia.id,
                ia.type::text as "activity_type!",
                ia.activity_json,
                me.from_did,
                me.content
            FROM inbox_activities ia
            JOIN deliveries d ON ia.id = d.activity_id
            LEFT JOIN message_entries me ON ia.id = me.activity_id AND me.to_did = d.to_did
            WHERE ia.inbox_actor_id = $1 AND d.to_did = $2
            ORDER BY ia.created_at ASC
            "#,
            inbox_actor_id,
            did.as_uuid()
        )
        .fetch_all(&self.pool)
        .await?;

        let mut activities = Vec::new();
        for row in rows {
            let mut activity: Activity = serde_json::from_value(row.activity_json)?;

            // For Create activities, inject the specific message entry for this device
            if let Activity::Create(ref mut create) = activity {
                if let (Some(from_did), Some(content)) = (row.from_did, row.content) {
                    // Find the to_did URL from the original content
                    // We stored from_did as a string, we need to find the matching entry's 'to' field
                    let to_url = create
                        .object
                        .content
                        .iter()
                        .find(|e| {
                            if let Ok(entry_did) = DeviceId::from_url(&e.to) {
                                entry_did == did
                            } else {
                                false
                            }
                        })
                        .map(|e| e.to.clone())
                        .unwrap_or_else(|| {
                            // Fallback: construct URL from actor's domain
                            // Extract domain from actor URL
                            let domain = create.to.split("/").take(3).collect::<Vec<_>>().join("/");
                            did.to_url(&domain)
                        });

                    // Replace the content array with just this device's entry
                    create.object.content = vec![crate::activitypub::EncryptedMessageEntry {
                        from: from_did,
                        to: to_url,
                        content,
                    }];
                }
            }

            activities.push(activity);
        }

        Ok(activities)
    }

    async fn insert_create(&self, create: &Create) -> Result<(), AppError> {
        let activity_id = create
            .id
            .as_ref()
            .ok_or_else(|| AppError::BadRequest("Create activity must have an id".to_string()))?;

        let activity_json = serde_json::to_value(create)?;

        // Extract inbox_actor_id from the 'to' field
        let inbox_actor_id = &create.to;

        // Start a transaction
        let mut tx = self.pool.begin().await?;

        // Insert the activity
        sqlx::query!(
            r#"
            INSERT INTO inbox_activities (id, type, inbox_actor_id, activity_json)
            VALUES ($1, 'Create', $2, $3)
            "#,
            activity_id,
            inbox_actor_id,
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
        let inbox_actor_id = activity.as_base().to();

        // Determine activity type
        let activity_type = match activity {
            Activity::Take(_) => "Take",
            Activity::Delivered(_) => "Delivered",
            Activity::Create(_) => {
                return Err(AppError::BadRequest(
                    "Use insert_create for Create activities".to_string(),
                ));
            }
        };

        // Start a transaction
        let mut tx = self.pool.begin().await?;

        // Insert the activity
        sqlx::query!(
            r#"
            INSERT INTO inbox_activities (id, type, inbox_actor_id, activity_json)
            VALUES ($1, $2, $3, $4)
            "#,
            activity_id,
            activity_type,
            inbox_actor_id,
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

    async fn is_first_delivery(&self, create_id: &str) -> Result<bool, AppError> {
        // Check if any deliveries have been made by comparing:
        // - total message entries (original number of devices)
        // - remaining deliveries (devices that haven't acknowledged yet)
        // If they're equal, this is the first delivery
        let result = sqlx::query!(
            r#"
            SELECT 
                (SELECT COUNT(*) FROM message_entries WHERE activity_id = $1) as "total_entries!",
                (SELECT COUNT(*) FROM deliveries WHERE activity_id = $1) as "remaining_deliveries!"
            "#,
            create_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => {
                // If total_entries == remaining_deliveries, no deliveries have been deleted yet
                // Since we just deleted one in delete_delivery, this means it was the first
                Ok(row.total_entries == row.remaining_deliveries + 1)
            }
            None => {
                // Activity doesn't exist
                Ok(false)
            }
        }
    }
}
