#[cfg(feature = "auth-firebase")]
pub mod firebase;
pub mod handlers;
pub mod jwt;

#[cfg(feature = "auth-oidc")]
pub mod oidc;

#[cfg(feature = "auth-firebase")]
pub use firebase::FirebaseAuth;
pub use handlers::{
    Auth, IdentityProvider, LoginRequest, LoginResponse, PreKey, REFRESH_EXPIRATION,
    RefreshRequest, RefreshResponse, SignedPreKey, SignupRequest, login_handler, logout_handler,
    refresh_token_handler, signup_handler,
};
pub use jwt::{Claims, JwtHelper};

#[cfg(feature = "auth-oidc")]
pub use oidc::{
    OidcConfig, OidcIdentityProvider, OidcProvider, oidc_callback_handler, oidc_complete_handler,
    oidc_login_handler, oidc_providers_handler,
};
