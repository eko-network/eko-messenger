#![cfg(feature = "auth-firebase")]

use crate::common::*;

use eko_messenger::auth::{Auth, FirebaseAuth};
use std::env;

/// Helper to get Firebase auth service (for tests that need it)
async fn get_auth_service(app: &TestApp) -> Auth {
    let firebase_auth = FirebaseAuth::new_from_env(app.domain.clone(), reqwest::Client::new())
        .await
        .unwrap();
    Auth::new(app.domain.clone(), firebase_auth, app.storage.clone())
}

/// Test login and token verification with Firebase (requires env vars)
#[tokio::test]
async fn test_firebase_login_and_verify_token() {
    let email = env::var("TEST_USER_EMAIL").expect("TEST_USER_EMAIL not set");
    let password = env::var("TEST_USER_PASSWORD").expect("TEST_USER_PASSWORD not set");

    let app = spawn_app().await;
    let auth = get_auth_service(&app).await;

    let login_req = app.generate_login_request(email, password, None);

    let login_res = auth
        .login(login_req, "127.0.0.1", "test-agent")
        .await
        .unwrap();

    assert!(!login_res.access_token.is_empty());

    let claims = auth.verify_access_token(&login_res.access_token).unwrap();
    assert!(!claims.sub.is_empty());
}

/// Test refresh token flow with Firebase
#[tokio::test]
async fn test_firebase_refresh_token() {
    let email = env::var("TEST_USER_EMAIL").expect("TEST_USER_EMAIL not set");
    let password = env::var("TEST_USER_PASSWORD").expect("TEST_USER_PASSWORD not set");

    let app = spawn_app().await;
    let auth = get_auth_service(&app).await;

    let login_req = app.generate_login_request(email, password, None);

    let login_res = auth
        .login(login_req, "127.0.0.1", "test-agent")
        .await
        .unwrap();

    let refresh_res = auth
        .refresh_token(&login_res.refresh_token, "127.0.0.1", "test-agent")
        .await
        .unwrap();

    assert!(!refresh_res.access_token.is_empty());

    // Verify new token is valid
    let claims = auth.verify_access_token(&refresh_res.access_token).unwrap();
    let first_claims = auth.verify_access_token(&login_res.access_token).unwrap();
    assert_eq!(claims.sub, first_claims.sub);
}

/// Test logout invalidates refresh token with Firebase
#[tokio::test]
async fn test_firebase_logout() {
    let email = env::var("TEST_USER_EMAIL").expect("TEST_USER_EMAIL not set");
    let password = env::var("TEST_USER_PASSWORD").expect("TEST_USER_PASSWORD not set");

    let app = spawn_app().await;
    let auth = get_auth_service(&app).await;

    let login_req = app.generate_login_request(email, password, None);

    let login_res = auth
        .login(login_req, "127.0.0.1", "test-agent")
        .await
        .unwrap();

    // Logout
    auth.logout(&login_res.refresh_token).await.unwrap();

    // Try to refresh - should fail
    let refresh_result = auth
        .refresh_token(&login_res.refresh_token, "127.0.0.1", "test-agent")
        .await;

    assert!(refresh_result.is_err(), "Refresh should fail after logout");
}
