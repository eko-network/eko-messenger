#[cfg(feature = "auth-firebase")]
use crate::auth::FirebaseAuth;
#[cfg(feature = "auth-oidc")]
use crate::auth::{
    OidcIdentityProvider, OidcProvider, oidc_callback_handler, oidc_complete_handler,
    oidc_login_handler, oidc_providers_handler,
};
use crate::{AppState, auth::Auth, storage::Storage};
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;

#[cfg(all(feature = "auth-firebase", feature = "auth-oidc"))]
compile_error!("Cannot enable both 'auth-firebase' and 'auth-oidc' features");
#[cfg(not(any(feature = "auth-firebase", feature = "auth-oidc")))]
compile_error!("Must enable exactly one auth provider: 'auth-firebase' or 'auth-oidc'");

#[cfg(feature = "auth-oidc")]
pub type OidcProviderState = Option<Arc<OidcProvider>>;
#[cfg(not(feature = "auth-oidc"))]
pub type OidcProviderState = Option<()>;

pub async fn build_auth(
    domain: Arc<String>,
    storage: Arc<Storage>,
) -> anyhow::Result<(Auth, OidcProviderState)> {
    #[cfg(feature = "auth-firebase")]
    {
        let client = reqwest::Client::new();
        let firebase_auth = FirebaseAuth::new_from_env(domain.clone(), client).await?;
        let auth = Auth::new(domain.clone(), firebase_auth, storage.clone());
        Ok((auth, None))
    }

    #[cfg(feature = "auth-oidc")]
    {
        let oidc_provider =
            Arc::new(OidcProvider::new_from_env(domain.clone(), storage.clone()).await?);
        let oidc_identity = OidcIdentityProvider::new(domain.clone(), oidc_provider.clone());
        let auth = Auth::new(domain.clone(), oidc_identity, storage.clone());
        Ok((auth, Some(oidc_provider)))
    }
}

pub fn add_oidc_routes(router: Router<AppState>) -> Router<AppState> {
    #[cfg(feature = "auth-oidc")]
    {
        router
            .route("/auth/v1/oidc/providers", get(oidc_providers_handler))
            .route("/auth/v1/oidc/login/{provider}", get(oidc_login_handler))
            .route(
                "/auth/v1/oidc/callback/{provider}",
                get(oidc_callback_handler),
            )
            .route("/auth/v1/oidc/complete", post(oidc_complete_handler))
    }

    #[cfg(not(feature = "auth-oidc"))]
    {
        router
    }
}
