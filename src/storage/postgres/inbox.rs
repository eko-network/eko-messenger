use async_trait::async_trait;
use serde_json::Value;
use sqlx::PgPool;

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
    ) -> Result<Vec<Value>, AppError> {
        let rows = sqlx::query!(
            r#"
            SELECT a.activity_json
            FROM activities a
            INNER JOIN inbox_entries ie ON a.id = ie.activity_id
            WHERE ie.inbox_actor_id = $1
            ORDER BY a.created_at DESC
            "#,
            inbox_actor_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.activity_json).collect())
    }
}

