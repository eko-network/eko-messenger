use crate::{
    errors::AppError,
    storage::{models::StoredUser, traits::UserStore},
};
use async_trait::async_trait;
use sqlx::PgPool;

pub struct PostgresUserStore {
    pool: PgPool,
}

impl PostgresUserStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserStore for PostgresUserStore {
    async fn get_user_by_email(&self, email: &str) -> Result<Option<StoredUser>, AppError> {
        let user = sqlx::query_as!(
            StoredUser,
            r#"
            SELECT uid, username, email, password_hash, created_at
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn get_user_by_uid(&self, uid: &str) -> Result<Option<StoredUser>, AppError> {
        let user = sqlx::query_as!(
            StoredUser,
            r#"
            SELECT uid, username, email, password_hash, created_at
            FROM users
            WHERE uid = $1
            "#,
            uid
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<StoredUser>, AppError> {
        let user = sqlx::query_as!(
            StoredUser,
            r#"
            SELECT uid, username, email, password_hash, created_at
            FROM users
            WHERE username = $1
            "#,
            username
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn create_user(
        &self,
        uid: &str,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO users (uid, username, email, password_hash)
            VALUES ($1, $2, $3, $4)
            "#,
            uid,
            username,
            email,
            password_hash
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
