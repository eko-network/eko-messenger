#![cfg(feature = "auth-oidc")]

use crate::auth_providers::helpers::*;
use crate::common::*;
use eko_messenger::auth::oidc::{Oidc, OidcCompleteRequest};
use std::{env, sync::Arc};
use uuid::Uuid;

//
// DIRECT PROVIDER TESTS (Unit Tests)
// These test OIDC provider methods directly without HTTP
//

/// Test that OIDC provider can be initialized from environment variables
#[tokio::test]
async fn test_oidc_provider_initialization() {
    let domain = Arc::new("http://test.example.com".to_string());
    let client = reqwest::Client::new();

    // Create minimal storage for testing
    let app = spawn_app().await;

    let oidc = Oidc::new_from_env(domain.clone(), app.storage.clone(), client)
        .await
        .expect("OIDC provider should initialize from env vars");

    // Verify config was loaded - start_auth should work
    let (auth_url, _csrf, _nonce) = oidc
        .start_auth()
        .expect("Should be able to start auth flow");

    assert!(!auth_url.is_empty(), "Auth URL should not be empty");
    assert!(
        auth_url.contains(&env::var("OIDC_ISSUER").unwrap()),
        "Auth URL should contain issuer"
    );
}

/// Test OIDC start_auth creates auth URL and tokens
#[tokio::test]
async fn test_oidc_start_auth() {
    let domain = Arc::new("http://test.example.com".to_string());
    let client = reqwest::Client::new();
    let app = spawn_app().await;

    let oidc = Oidc::new_from_env(domain.clone(), app.storage.clone(), client)
        .await
        .expect("Failed to create OIDC provider");

    let (auth_url, csrf_token, nonce) = oidc.start_auth().expect("start_auth should succeed");

    // Verify all parts are present
    assert!(!auth_url.is_empty(), "Auth URL should not be empty");
    assert!(
        !csrf_token.secret().is_empty(),
        "CSRF token should not be empty"
    );
    assert!(!nonce.secret().is_empty(), "Nonce should not be empty");

    // Auth URL should contain expected params
    let issuer = env::var("OIDC_ISSUER").unwrap();
    assert!(auth_url.contains(&issuer), "Auth URL should contain issuer");
    assert!(
        auth_url.contains("state="),
        "Auth URL should contain state param"
    );
    assert!(
        auth_url.contains("nonce="),
        "Auth URL should contain nonce param"
    );
}

/// Test OIDC person_from_uid
#[tokio::test]
async fn test_oidc_person_from_uid() {
    let domain = Arc::new("http://test.example.com".to_string());
    let client = reqwest::Client::new();
    let app = spawn_app().await;

    let oidc = Oidc::new_from_env(domain.clone(), app.storage.clone(), client)
        .await
        .expect("Failed to create OIDC provider");

    // Create a test user directly in storage
    let uid = Uuid::new_v4().to_string();
    let username = "testuser";
    let email = "test@example.com";
    let issuer = env::var("OIDC_ISSUER").unwrap();
    let oidc_sub = "oidc-sub-123";

    app.storage
        .users
        .create_oidc_user(&uid, username, email, &issuer, oidc_sub)
        .await
        .expect("Should create test user");

    // Now fetch Person by UID
    let person = oidc
        .person_from_uid(&uid)
        .await
        .expect("Should be able to fetch Person by UID");

    assert_eq!(person.type_field, "Person");
    assert!(!person.id.is_empty());
    assert_eq!(person.preferred_username, username);
}

/// Test OIDC uid_from_username
#[tokio::test]
async fn test_oidc_uid_from_username() {
    let domain = Arc::new("http://test.example.com".to_string());
    let client = reqwest::Client::new();
    let app = spawn_app().await;

    let oidc = Oidc::new_from_env(domain.clone(), app.storage.clone(), client)
        .await
        .expect("Failed to create OIDC provider");

    // Create a test user directly in storage
    let expected_uid = Uuid::new_v4().to_string();
    let username = format!("testuser_{}", Uuid::new_v4().simple());
    let email = "test@example.com";
    let issuer = env::var("OIDC_ISSUER").unwrap();
    let oidc_sub = "oidc-sub-456";

    app.storage
        .users
        .create_oidc_user(&expected_uid, &username, email, &issuer, oidc_sub)
        .await
        .expect("Should create test user");

    // Now lookup UID by username
    let uid = oidc
        .uid_from_username(&username)
        .await
        .expect("Should be able to fetch UID by username");

    assert_eq!(uid, expected_uid, "UID should match");
}

/// Test OIDC verification token flow (create and verify)
#[tokio::test]
async fn test_oidc_verification_token_flow() {
    let domain = Arc::new("http://test.example.com".to_string());
    let client = reqwest::Client::new();
    let app = spawn_app().await;

    let oidc = Oidc::new_from_env(domain.clone(), app.storage.clone(), client)
        .await
        .expect("Failed to create OIDC provider");

    let test_email = "test@example.com";
    let test_uid = Uuid::new_v4().to_string();

    // Create verification token
    let token = oidc
        .create_verification_token(test_email, &test_uid)
        .expect("Should create verification token");

    assert!(!token.is_empty(), "Token should not be empty");

    // Verify token
    let (provider, email, uid) = oidc
        .verify_verification_token(&token)
        .expect("Should verify valid token");

    assert_eq!(provider, "oidc");
    assert_eq!(email, test_email);
    assert_eq!(uid, test_uid);
}

/// Test OIDC CSRF token validation
#[tokio::test]
async fn test_oidc_csrf_validation() {
    let domain = Arc::new("http://test.example.com".to_string());
    let client = reqwest::Client::new();
    let app = spawn_app().await;

    let oidc = Oidc::new_from_env(domain.clone(), app.storage.clone(), client)
        .await
        .expect("Failed to create OIDC provider");

    // Start auth to create a valid CSRF token
    let (_auth_url, csrf_token, _nonce) = oidc.start_auth().expect("start_auth should succeed");

    // Try to exchange with wrong CSRF token
    let result = oidc.exchange_code("fake-code", "wrong-csrf-token").await;

    assert!(
        result.is_err(),
        "exchange_code should fail with invalid CSRF token"
    );

    // Note: We can't test with the correct CSRF token without a real OAuth code
    // That would require browser interaction with the IdP
}

//
// HTTP INTEGRATION TESTS
// These test OIDC endpoints through the production router
//

/// Test OIDC login start endpoint
#[tokio::test]
async fn test_oidc_http_login_start() {
    let app = spawn_app_with_oidc().await;

    let response = app
        .client
        .get(&format!("{}/auth/v1/oidc/login", app.address))
        .send()
        .await
        .expect("Login start request should succeed");

    assert!(
        response.status().is_success(),
        "Login start should return 200 OK"
    );

    let login_response: eko_messenger::auth::oidc::OidcLoginResponse = response
        .json()
        .await
        .expect("Response should be valid OidcLoginResponse");

    assert!(
        !login_response.login_url.is_empty(),
        "Login URL should not be empty"
    );
    assert!(
        !login_response.state.is_empty(),
        "State (CSRF token) should not be empty"
    );

    // Verify login URL contains issuer
    let issuer = env::var("OIDC_ISSUER").unwrap();
    assert!(
        login_response.login_url.contains(&issuer),
        "Login URL should contain OIDC issuer"
    );
}

/// Test OIDC complete flow (with manually created verification token)
#[tokio::test]
async fn test_oidc_http_complete_flow() {
    let app = spawn_app_with_oidc().await;

    // Create a test user directly in storage
    let uid = Uuid::new_v4().to_string();
    let username = format!("testuser_{}", Uuid::new_v4().simple());
    let email = "test@example.com";
    let issuer = env::var("OIDC_ISSUER").unwrap();
    let oidc_sub = "oidc-sub-complete-test";

    app.storage
        .users
        .create_oidc_user(&uid, &username, &email, &issuer, oidc_sub)
        .await
        .expect("Should create test user");

    // Create OIDC provider to generate verification token
    let oidc = Oidc::new_from_env(
        app.domain.clone(),
        app.storage.clone(),
        reqwest::Client::new(),
    )
    .await
    .expect("Should create OIDC provider");

    let verification_token = oidc
        .create_verification_token(&email, &uid)
        .expect("Should create verification token");

    // Complete login with verification token
    let complete_req = OidcCompleteRequest {
        verification_token,
        device_name: "test_device".to_string(),
        identity_key: vec![1, 2, 3],
        registration_id: 123,
        pre_keys: vec![eko_messenger::auth::PreKey {
            id: 1,
            key: vec![4, 5, 6],
        }],
        signed_pre_key: eko_messenger::auth::SignedPreKey {
            id: 1,
            key: vec![7, 8, 9],
            signature: vec![10, 11, 12],
        },
    };

    let response = app
        .client
        .post(&format!("{}/auth/v1/oidc/complete", app.address))
        .header("User-Agent", "test-agent")
        .json(&complete_req)
        .send()
        .await
        .expect("Complete request should succeed");

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Could not read error".to_string());
        panic!("Complete should return 200 OK, got: {} - Error: {}", status, error_text);
    }

    let login_response: eko_messenger::auth::LoginResponse = response
        .json()
        .await
        .expect("Response should be valid LoginResponse");

    assert_valid_login_response(&login_response);

    // Verify the access token is valid
    let claims = assert_valid_access_token(&app.sessions, &login_response.access_token);
    assert_eq!(claims.sub, uid, "Token should have correct user UID");
}

/// Test OIDC complete with invalid verification token
#[tokio::test]
async fn test_oidc_http_complete_invalid_token() {
    let app = spawn_app_with_oidc().await;

    let complete_req = OidcCompleteRequest {
        verification_token: "invalid-token".to_string(),
        device_name: "test_device".to_string(),
        identity_key: vec![1, 2, 3],
        registration_id: 123,
        pre_keys: vec![eko_messenger::auth::PreKey {
            id: 1,
            key: vec![4, 5, 6],
        }],
        signed_pre_key: eko_messenger::auth::SignedPreKey {
            id: 1,
            key: vec![7, 8, 9],
            signature: vec![10, 11, 12],
        },
    };

    let response = app
        .client
        .post(&format!("{}/auth/v1/oidc/complete", app.address))
        .header("User-Agent", "test-agent")
        .json(&complete_req)
        .send()
        .await
        .expect("Request should complete");

    assert_eq!(
        response.status(),
        401,
        "Complete with invalid token should return 401 Unauthorized"
    );
}

/// Test OIDC complete and refresh token flow
#[tokio::test]
async fn test_oidc_http_complete_and_refresh() {
    let app = spawn_app_with_oidc().await;

    // Create test user and verification token
    let uid = Uuid::new_v4().to_string();
    let username = format!("testuser_{}", Uuid::new_v4().simple());
    let email = "test@example.com";
    let issuer = env::var("OIDC_ISSUER").unwrap();
    let oidc_sub = format!("oidc-sub-refresh-{}", Uuid::new_v4().simple());

    app.storage
        .users
        .create_oidc_user(&uid, &username, &email, &issuer, &oidc_sub)
        .await
        .expect("Should create test user");

    let oidc = Oidc::new_from_env(
        app.domain.clone(),
        app.storage.clone(),
        reqwest::Client::new(),
    )
    .await
    .expect("Should create OIDC provider");

    let verification_token = oidc
        .create_verification_token(&email, &uid)
        .expect("Should create verification token");

    // Step 1: Complete login
    let complete_req = OidcCompleteRequest {
        verification_token,
        device_name: "test_device".to_string(),
        identity_key: vec![1, 2, 3],
        registration_id: 123,
        pre_keys: vec![eko_messenger::auth::PreKey {
            id: 1,
            key: vec![4, 5, 6],
        }],
        signed_pre_key: eko_messenger::auth::SignedPreKey {
            id: 1,
            key: vec![7, 8, 9],
            signature: vec![10, 11, 12],
        },
    };

    let login_response: eko_messenger::auth::LoginResponse = app
        .client
        .post(&format!("{}/auth/v1/oidc/complete", app.address))
        .header("User-Agent", "test-agent")
        .json(&complete_req)
        .send()
        .await
        .expect("Complete should succeed")
        .json()
        .await
        .expect("Should parse LoginResponse");

    let first_claims = assert_valid_access_token(&app.sessions, &login_response.access_token);

    // Step 2: Refresh token
    let refresh_req = eko_messenger::auth::RefreshRequest {
        refresh_token: login_response.refresh_token.clone(),
    };

    let refresh_response: eko_messenger::auth::RefreshResponse = app
        .client
        .post(&format!("{}/auth/v1/refresh", app.address))
        .header("User-Agent", "test-agent")
        .json(&refresh_req)
        .send()
        .await
        .expect("Refresh request should succeed")
        .json()
        .await
        .expect("Should parse RefreshResponse");

    assert!(
        !refresh_response.access_token.is_empty(),
        "Refresh should return new access token"
    );

    // Verify new token is valid and has same user
    let new_claims = assert_valid_access_token(&app.sessions, &refresh_response.access_token);
    assert_eq!(
        new_claims.sub, first_claims.sub,
        "Refreshed token should have same user"
    );
}

/// Test OIDC complete and logout flow
#[tokio::test]
async fn test_oidc_http_complete_and_logout() {
    let app = spawn_app_with_oidc().await;

    // Create test user and verification token
    let uid = Uuid::new_v4().to_string();
    let username = format!("testuser_{}", Uuid::new_v4().simple());
    let email = "test@example.com";
    let issuer = env::var("OIDC_ISSUER").unwrap();
    let oidc_sub = format!("oidc-sub-logout-{}", Uuid::new_v4().simple());

    app.storage
        .users
        .create_oidc_user(&uid, &username, &email, &issuer, &oidc_sub)
        .await
        .expect("Should create test user");

    let oidc = Oidc::new_from_env(
        app.domain.clone(),
        app.storage.clone(),
        reqwest::Client::new(),
    )
    .await
    .expect("Should create OIDC provider");

    let verification_token = oidc
        .create_verification_token(&email, &uid)
        .expect("Should create verification token");

    // Step 1: Complete login
    let complete_req = OidcCompleteRequest {
        verification_token,
        device_name: "test_device".to_string(),
        identity_key: vec![1, 2, 3],
        registration_id: 123,
        pre_keys: vec![eko_messenger::auth::PreKey {
            id: 1,
            key: vec![4, 5, 6],
        }],
        signed_pre_key: eko_messenger::auth::SignedPreKey {
            id: 1,
            key: vec![7, 8, 9],
            signature: vec![10, 11, 12],
        },
    };

    let login_response: eko_messenger::auth::LoginResponse = app
        .client
        .post(&format!("{}/auth/v1/oidc/complete", app.address))
        .header("User-Agent", "test-agent")
        .json(&complete_req)
        .send()
        .await
        .expect("Complete should succeed")
        .json()
        .await
        .expect("Should parse LoginResponse");

    assert_valid_login_response(&login_response);

    // Step 2: Logout
    let logout_req = eko_messenger::auth::LogoutRequest {
        refresh_token: login_response.refresh_token.clone(),
    };

    let logout_response = app
        .client
        .post(&format!("{}/auth/v1/logout", app.address))
        .header(
            "Authorization",
            format!("Bearer {}", login_response.access_token),
        )
        .json(&logout_req)
        .send()
        .await
        .expect("Logout request should succeed");

    assert!(
        logout_response.status().is_success(),
        "Logout should succeed"
    );

    // Step 3: Try to refresh - should fail
    let refresh_req = eko_messenger::auth::RefreshRequest {
        refresh_token: login_response.refresh_token,
    };

    let refresh_response = app
        .client
        .post(&format!("{}/auth/v1/refresh", app.address))
        .header("User-Agent", "test-agent")
        .json(&refresh_req)
        .send()
        .await
        .expect("Refresh request should complete");

    assert_eq!(
        refresh_response.status(),
        401,
        "Refresh should fail after logout with 401 Unauthorized"
    );
}
