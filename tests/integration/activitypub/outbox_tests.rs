use crate::common::*;
use serde_json::json;

/// Test posting a Create activity to outbox
#[tokio::test]
async fn test_post_create_activity_to_outbox() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    let response = alice.send_message_to(&app, &bob, "Test message").await;

    // Should return 201 Created
    assert_status(response, 201).await;
}

/// Test posting to outbox without authentication fails
#[tokio::test]
async fn test_post_to_outbox_unauthenticated_fails() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    // Build a proper envelope from Bob to Alice
    let envelope = SignalEnvelope::new()
        .add_messages_for_all_devices(alice.devices[0].url.clone(), &bob, "test message")
        .build_message(&alice.actor_id, &bob.actor_id);

    let activity = alice.create_message_activity(envelope);
    let outbox_url = format!("{}/outbox", &alice.actor_id);

    // Dont send token
    let response = app
        .client
        .post(&outbox_url)
        .header("Content-Type", "application/activity+json")
        .json(&activity)
        .send()
        .await
        .expect("Request failed");

    // Should return 401 Unauthorized
    assert_status(response, 401).await;
}

/// Test posting activity as wrong user fails
#[tokio::test]
async fn test_post_to_outbox_as_wrong_user_fails() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    // Build a proper envelope from Bob to Alice
    let envelope = SignalEnvelope::new()
        .add_messages_for_all_devices(bob.devices[0].url.clone(), &alice, "test message")
        .build_message(&bob.actor_id, &alice.actor_id);

    let activity = bob.create_message_activity(envelope);

    // Try to post to Alice's outbox using Bob's token
    let outbox_url = format!("{}/outbox", &alice.actor_id);

    let response = app
        .client
        .post(&outbox_url)
        .bearer_auth(&bob.devices[0].token)
        .header("Content-Type", "application/activity+json")
        .json(&activity)
        .send()
        .await
        .expect("Request failed");

    // Should return 403 or 401
    assert!(
        response.status().as_u16() == 401 || response.status().as_u16() == 403,
        "Expected 401 or 403, got {}",
        response.status()
    );
}

/// Test posting malformed activity fails
#[tokio::test]
async fn test_post_malformed_activity_fails() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    let outbox_url = format!("{}/outbox", &alice.actor_id);

    // Missing required fields
    let activity = json!({
        "type": "Create",
        // Missing actor and object
    });

    let response = bob.post_to_outbox(&app, activity).await;

    // Should return 422 unprocessable content or 400
    assert!(
        response.status().as_u16() == 422 || response.status().as_u16() == 400,
        "Expected 401 or 403, got {}",
        response.status()
    );
}
