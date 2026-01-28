use crate::common::*;

/// Test getting a user's actor profile
#[tokio::test]
async fn test_get_user_actor() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;

    // Get actor profile
    let actor = alice.get_actor(&app).await;

    // TODO probably deserialize into a struct?

    // Verify actor type and basic structure
    assert_field_equals(&actor, "type", &"Person".into());
    assert_field_equals(&actor, "id", &alice.actor_id.clone().into());
    assert_field_equals(&actor, "preferredUsername", &alice.username.clone().into());

    // Verify ActivityPub endpoints
    let inbox_url = format!("{}/inbox", alice.actor_id);
    let outbox_url = format!("{}/outbox", alice.actor_id);
    let devices_url = format!("{}/users/{}/deviceActions", app.domain, alice.uid);

    assert_field_equals(&actor, "inbox", &inbox_url.into());
    assert_field_equals(&actor, "outbox", &outbox_url.into());
    assert_field_equals(&actor, "devices", &devices_url.into());

    // Verify context is present
    assert_has_field(&actor, "@context");
}

/// Test getting public actor profile without authentication
#[tokio::test]
async fn test_get_actor_unauthenticated() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;

    // Get actor without authentication
    let response = app
        .client
        .get(&alice.actor_id)
        .header("Accept", "application/activity+json")
        .send()
        .await
        .expect("Request failed");

    let response = assert_status(response, 200).await;

    let actor: serde_json::Value = response.json().await.unwrap();
    assert_field_equals(&actor, "type", &"Person".into());
}

/// Test getting non existent actor returns 404
#[tokio::test]
async fn test_get_nonexistent_actor() {
    let app = spawn_app().await;

    let nonexistent_url = format!("{}/users/nonexistent", app.address);

    let response = app
        .client
        .get(&nonexistent_url)
        .header("Accept", "application/activity+json")
        .send()
        .await
        .expect("Request failed");

    assert_status(response, 404).await;
}
