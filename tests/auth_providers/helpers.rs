use eko_messenger::auth::{Claims, LoginResponse, SessionManager};
use std::env;

/// Helper to require an environment variable, panicking with a clear message if not set
pub fn require_env_var(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("{} must be set for auth provider tests", key))
}

/// Test user credentials loaded from environment
pub struct TestCredentials {
    pub email: String,
    pub password: String,
}

impl TestCredentials {
    pub fn from_env() -> Self {
        Self {
            email: require_env_var("TEST_USER_EMAIL"),
            password: require_env_var("TEST_USER_PASSWORD"),
        }
    }
}

/// Assert that a LoginResponse has all required fields populated
pub fn assert_valid_login_response(response: &LoginResponse) {
    assert!(
        !response.access_token.is_empty(),
        "Access token should not be empty"
    );
    // refresh_token is a Uuid, just check it exists (always valid)
    assert!(
        !response.actor.id.is_empty(),
        "Actor ID should not be empty"
    );
    assert_eq!(
        response.actor.type_field, "Person",
        "Actor should be a Person"
    );
}

/// Verify an access token and return the claims
pub fn assert_valid_access_token(sessions: &SessionManager, token: &str) -> Claims {
    sessions
        .verify_access_token(token)
        .expect("Access token should be valid")
}
