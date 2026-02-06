pub mod jwt;
pub mod session;

use crate::{activitypub::Person, errors::AppError};
use async_trait::async_trait;
use std::sync::Arc;

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

#[derive(Clone)]
pub enum AuthProvider {
    #[cfg(feature = "auth-firebase")]
    Firebase(Arc<firebase::Firebase>),
    #[cfg(feature = "auth-oidc")]
    Oidc(Arc<oidc::Oidc>),
}

impl AuthProvider {
    #[cfg(feature = "auth-firebase")]
    pub fn as_firebase(&self) -> &Arc<firebase::Firebase> {
        match self {
            Self::Firebase(fb) => fb,
            #[cfg(feature = "auth-oidc")]
            _ => unreachable!("Called as_firebase() on non-Firebase provider"),
        }
    }

    #[cfg(feature = "auth-oidc")]
    pub fn as_oidc(&self) -> &Arc<oidc::Oidc> {
        match self {
            Self::Oidc(o) => o,
            #[cfg(feature = "auth-firebase")]
            _ => unreachable!("Called as_oidc() on non-OIDC provider"),
        }
    }
}

// Shared types and utilities
pub mod types;
pub use types::{
    LoginRequest, LoginResponse, LogoutRequest, PreKey, RefreshRequest, RefreshResponse,
    SignedPreKey,
};

pub use jwt::{Claims, JwtHelper};
pub use session::SessionManager;
