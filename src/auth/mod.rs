#[cfg(feature = "auth-firebase")]
pub mod firebase;
pub mod handlers;
pub mod jwt;
#[cfg(feature = "auth-local")]
pub mod local;

#[cfg(feature = "auth-firebase")]
pub use firebase::FirebaseAuth;
pub use handlers::{
    Auth, IdentityProvider, LoginRequest, LoginResponse, PreKey, REFRESH_EXPIRATION,
    RefreshRequest, RefreshResponse, SignedPreKey, SignupRequest, login_handler, logout_handler,
    refresh_token_handler, signup_handler,
};
pub use jwt::{Claims, JwtHelper};
#[cfg(feature = "auth-local")]
pub use local::LocalIdentityProvider;
