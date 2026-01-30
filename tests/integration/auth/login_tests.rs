use crate::common::*;

/// Test basic login with test identity provider
#[tokio::test]
async fn test_basic_login() {
    let app = spawn_app().await;

    // Create user via login
    let alice = TestUser::create(&app, "alice").await;

    // Verify token is not empty (first device has the token)
    assert!(
        !alice.devices[0].token.is_empty(),
        "Access token should not be empty"
    );
    assert!(!alice.uid.is_empty(), "UID should not be empty");
}

/// Test that login creates actor profile
#[tokio::test]
async fn test_login_creates_actor() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;

    // Verify actor profile was created
    let actor = alice.get_actor(&app).await;
    assert_field_equals(&actor, "type", &"Person".into());
    assert_field_equals(&actor, "preferredUsername", &alice.username.clone().into());
}

/// Test that login without credentials fails
#[tokio::test]
async fn test_login_without_credentials_fails() {
    let app = spawn_app().await;

    let login_url = format!("{}/auth/v1/login", &app.address);

    // Send empty/invalid request
    let response = app
        .client
        .post(&login_url)
        .header("User-Agent", "test-client")
        .json(&serde_json::json!({}))
        .send()
        .await
        .expect("Request failed");

    // Should fail
    assert!(
        !response.status().is_success(),
        "Login without credentials should fail"
    );
}
