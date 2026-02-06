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

/// Test that SessionManager validation works
#[tokio::test]
async fn test_session_validation() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;

    // Verify that SessionManager can verify the token
    let claims = app.sessions.verify_access_token(&alice.devices[0].token);
    assert!(claims.is_ok(), "Token should be valid");

    let claims = claims.unwrap();
    assert_eq!(claims.sub, alice.uid, "Token should contain correct UID");
}
