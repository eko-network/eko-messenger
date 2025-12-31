use gcp_auth::{Token, TokenProvider};
use std::sync::Arc;
use tokio::sync::OnceCell;

pub static TOKEN_PROVIDER: OnceCell<Arc<dyn TokenProvider>> = OnceCell::const_new();

pub async fn token_provider() -> &'static Arc<dyn TokenProvider> {
    TOKEN_PROVIDER
        .get_or_init(|| async {
            gcp_auth::provider()
                .await
                .expect("unable to initialize token provider")
        })
        .await
}

/// Get an access token with the default cloud platform scope
pub async fn get_token() -> Result<Arc<Token>, gcp_auth::Error> {
    let provider = token_provider().await;
    provider
        .token(&["https://www.googleapis.com/auth/datastore"])
        .await
}
