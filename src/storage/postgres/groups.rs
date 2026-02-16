use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    storage::{models::StoredGroupState, traits::GroupStore},
};

pub struct PostgresGroupStore {
    pool: PgPool,
}

impl PostgresGroupStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GroupStore for PostgresGroupStore {
    async fn upsert_group_state(&self, state: &StoredGroupState) -> Result<bool, AppError> {
        let result = sqlx::query!(
            r#"
            INSERT INTO encrypted_group_states (id, group_id, user_id, epoch, encrypted_content, encoding)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (user_id, group_id) DO UPDATE
            SET epoch = EXCLUDED.epoch,
                encrypted_content = EXCLUDED.encrypted_content,
                encoding = EXCLUDED.encoding,
                id = EXCLUDED.id
            WHERE encrypted_group_states.epoch < EXCLUDED.epoch
            "#,
            state.id,
            state.group_id,
            state.user_id,
            state.epoch,
            state.encrypted_content,
            state.encoding,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn get_group_state(
        &self,
        user_id: &str,
        group_id: &Uuid,
    ) -> Result<Option<StoredGroupState>, AppError> {
        let row = sqlx::query!(
            r#"
            SELECT id, group_id, user_id, epoch, encrypted_content, encoding
            FROM encrypted_group_states
            WHERE user_id = $1 AND group_id = $2
            "#,
            user_id,
            group_id,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| StoredGroupState {
            id: r.id,
            group_id: r.group_id,
            user_id: r.user_id,
            epoch: r.epoch,
            encrypted_content: r.encrypted_content,
            encoding: r.encoding,
        }))
    }

    async fn get_all_group_states(&self, user_id: &str) -> Result<Vec<StoredGroupState>, AppError> {
        let rows = sqlx::query!(
            r#"
            SELECT id, group_id, user_id, epoch, encrypted_content, encoding
            FROM encrypted_group_states
            WHERE user_id = $1
            "#,
            user_id,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| StoredGroupState {
                id: r.id,
                group_id: r.group_id,
                user_id: r.user_id,
                epoch: r.epoch,
                encrypted_content: r.encrypted_content,
                encoding: r.encoding,
            })
            .collect())
    }

    async fn delete_group_state(&self, user_id: &str, group_id: &Uuid) -> Result<bool, AppError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM encrypted_group_states
            WHERE user_id = $1 AND group_id = $2
            "#,
            user_id,
            group_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
