use async_trait::async_trait;
use sqlx::PgPool;
use crate::storage::models::StoredOutboxActivity;
use crate::storage::traits::OutboxStore;
use crate::errors::AppError;

pub struct PostgresOutboxStore {
    pool: PgPool,
}

impl PostgresOutboxStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OutboxStore for PostgresOutboxStore {
    async fn insert_activity(
        &self,
        activity: &StoredOutboxActivity,
        inbox_actor_id: &str,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"
            INSERT INTO activities (id, actor_id, activity_type, activity_json)
            VALUES ($1, $2, $3, $4)
            "#,
            activity.activity_id,
            activity.actor_id,
            activity.activity_type,
            activity.activity
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO inbox_entries (inbox_actor_id, activity_id)
            VALUES ($1, $2)
            "#,
            inbox_actor_id,
            activity.activity_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}

