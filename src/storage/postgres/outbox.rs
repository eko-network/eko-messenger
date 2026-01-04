use crate::errors::AppError;
use crate::storage::models::StoredOutboxActivity;
use crate::storage::traits::OutboxStore;
use async_trait::async_trait;
use sqlx::PgPool;

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
    async fn insert_activity(&self, activity: &StoredOutboxActivity) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO activities (id, actor_id, activity_type, activity_json)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO NOTHING
            "#,
            activity.activity_id,
            activity.actor_id,
            activity.activity_type,
            activity.activity
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
