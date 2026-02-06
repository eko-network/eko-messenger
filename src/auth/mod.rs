pub mod jwt;
pub mod session;

use crate::{activitypub::Person, errors::AppError};
use async_trait::async_trait;

#[async_trait]
pub trait IdentityProvider: Send + Sync {
    async fn person_from_uid(&self, uid: &str) -> Result<Person, AppError>;
    async fn uid_from_username(&self, username: &str) -> Result<String, AppError>;
}

#[cfg(feature = "auth-firebase")]
pub mod firebase;
#[cfg(feature = "auth-firebase")]
pub use firebase::Firebase;

#[cfg(feature = "auth-oidc")]
pub mod oidc;
#[cfg(feature = "auth-oidc")]
pub use oidc::Oidc;

pub mod types;
pub use types::{
    LoginRequest, LoginResponse, LogoutRequest, PreKey, RefreshRequest, RefreshResponse,
    SignedPreKey,
};

pub use jwt::{Claims, JwtHelper};
pub use session::SessionManager;
