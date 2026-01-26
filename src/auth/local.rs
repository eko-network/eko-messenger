use crate::{
    activitypub::{Person, create_person},
    auth::{IdentityProvider, SignupRequest},
    errors::AppError,
    storage::Storage,
};
use argon2::PasswordVerifier;
use async_trait::async_trait;
use axum::http::StatusCode;
use std::sync::Arc;
use uuid::Uuid;

pub struct LocalIdentityProvider {
    storage: Arc<Storage>,
    domain: Arc<String>,
}

impl LocalIdentityProvider {
    pub fn new(domain: Arc<String>, storage: Arc<Storage>) -> Self {
        Self { storage, domain }
    }
}

#[async_trait]
impl IdentityProvider for LocalIdentityProvider {
    async fn login_with_email(
        &self,
        email: String,
        password: String,
    ) -> Result<(Person, String), AppError> {
        let user = self
            .storage
            .users
            .get_user_by_email(&email)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Invalid email or password".to_string()))?;

        let password_hash = user
            .password_hash
            .as_ref()
            .ok_or_else(|| AppError::Unauthorized("Invalid email or password".to_string()))?;

        let parsed_hash = argon2::PasswordHash::new(password_hash).map_err(|_| {
            AppError::InternalError(anyhow::anyhow!("Invalid password hash format"))
        })?;

        argon2::Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::Unauthorized("Invalid email or password".to_string()))?;

        let person = create_person(
            &self.domain,
            &user.uid,
            None,
            user.username.clone(),
            None,
            None,
        );

        Ok((person, user.uid))
    }

    async fn person_from_uid(&self, uid: &str) -> Result<Person, AppError> {
        let user = self
            .storage
            .users
            .get_user_by_uid(uid)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        Ok(create_person(
            &self.domain,
            &user.uid,
            None,
            user.username,
            None,
            None,
        ))
    }

    async fn uid_from_username(&self, username: &str) -> Result<String, AppError> {
        let user = self
            .storage
            .users
            .get_user_by_username(username)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        Ok(user.uid)
    }

    async fn signup(&self, req: SignupRequest) -> Result<StatusCode, AppError> {
        use argon2::password_hash::SaltString;
        use argon2::{Argon2, PasswordHasher};

        if self
            .storage
            .users
            .get_user_by_email(&req.email)
            .await?
            .is_some()
        {
            return Err(AppError::BadRequest("User already exists".to_string()));
        }

        if self
            .storage
            .users
            .get_user_by_username(&req.username)
            .await?
            .is_some()
        {
            return Err(AppError::BadRequest("Username already taken".to_string()));
        }

        // Hash password
        let salt = SaltString::generate(argon2::password_hash::rand_core::OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(req.password.as_bytes(), &salt)
            .map_err(|e| AppError::InternalError(anyhow::anyhow!(e)))?
            .to_string();

        let uid = Uuid::new_v4().to_string();

        self.storage
            .users
            .create_user(&uid, &req.username, &req.email, &password_hash)
            .await?;

        Ok(StatusCode::CREATED)
    }
}
