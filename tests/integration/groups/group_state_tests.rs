use crate::common::*;
use serde_json::Value;
use uuid::Uuid;

/// Test creating a new group state via PUT
#[tokio::test]
async fn test_upsert_group_state_creates_new() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;

    let group_id = Uuid::new_v4();
    let content = b"encrypted-group-state-blob";

    let response = alice.upsert_group_state(&app, group_id, 1, content).await;
    assert_status(response, 200).await;

    // Verify we can read it back
    let response = alice.get_group_state(&app, group_id).await;
    let response = assert_status(response, 200).await;
    let body: Value = response.json().await.unwrap();

    assert_eq!(body["groupId"], group_id.to_string());
    assert_eq!(body["epoch"], 1);
    assert_eq!(body["mediaType"], "application/eko-group-state");
    assert_eq!(body["encoding"], "base64");
}

/// Test that upserting with a higher epoch replaces the stored state
#[tokio::test]
async fn test_upsert_group_state_higher_epoch_replaces() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;

    let group_id = Uuid::new_v4();

    // Create initial state at epoch 1
    let response = alice
        .upsert_group_state(&app, group_id, 1, b"epoch-1-state")
        .await;
    assert_status(response, 200).await;

    // Upsert with higher epoch
    let response = alice
        .upsert_group_state(&app, group_id, 5, b"epoch-5-state")
        .await;
    assert_status(response, 200).await;

    // Verify the state was updated
    let response = alice.get_group_state(&app, group_id).await;
    let response = assert_status(response, 200).await;
    let body: Value = response.json().await.unwrap();

    assert_eq!(body["epoch"], 5);
    // The content should be the newer blob
    let stored_content: Vec<u8> = serde_json::from_value(body["encryptedContent"].clone()).unwrap();
    assert_eq!(stored_content, b"epoch-5-state");
}

/// Test that upserting with a stale (lower or equal) epoch is rejected
#[tokio::test]
async fn test_upsert_group_state_stale_epoch_rejected() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;

    let group_id = Uuid::new_v4();

    // Create initial state at epoch 5
    let response = alice
        .upsert_group_state(&app, group_id, 5, b"epoch-5-state")
        .await;
    assert_status(response, 200).await;

    // Try to upsert with a lower epoch
    let response = alice
        .upsert_group_state(&app, group_id, 3, b"epoch-3-state")
        .await;
    assert_error(response, 400, "Epoch must be higher than the stored epoch").await;

    // Try to upsert with the same epoch
    let response = alice
        .upsert_group_state(&app, group_id, 5, b"same-epoch-state")
        .await;
    assert_error(response, 400, "Epoch must be higher than the stored epoch").await;

    // Verify the original state is unchanged
    let response = alice.get_group_state(&app, group_id).await;
    let response = assert_status(response, 200).await;
    let body: Value = response.json().await.unwrap();

    assert_eq!(body["epoch"], 5);
    let stored_content: Vec<u8> = serde_json::from_value(body["encryptedContent"].clone()).unwrap();
    assert_eq!(stored_content, b"epoch-5-state");
}

/// Test GET for a nonexistent group state returns 404
#[tokio::test]
async fn test_get_group_state_not_found() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;

    let group_id = Uuid::new_v4();
    let response = alice.get_group_state(&app, group_id).await;
    assert_error(response, 404, "Group state not found").await;
}

/// Test listing all group states for a user
#[tokio::test]
async fn test_get_all_group_states() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;

    // Initially empty
    let response = alice.get_all_group_states(&app).await;
    let response = assert_status(response, 200).await;
    let body: Vec<Value> = response.json().await.unwrap();
    assert_eq!(body.len(), 0);

    // Create two groups
    let group1 = Uuid::new_v4();
    let group2 = Uuid::new_v4();

    alice
        .upsert_group_state(&app, group1, 1, b"group-1-state")
        .await;
    alice
        .upsert_group_state(&app, group2, 3, b"group-2-state")
        .await;

    // Should now have two entries
    let response = alice.get_all_group_states(&app).await;
    let response = assert_status(response, 200).await;
    let body: Vec<Value> = response.json().await.unwrap();
    assert_eq!(body.len(), 2);

    // Verify both group IDs are present
    let group_ids: Vec<String> = body
        .iter()
        .map(|v| v["groupId"].as_str().unwrap().to_string())
        .collect();
    assert!(group_ids.contains(&group1.to_string()));
    assert!(group_ids.contains(&group2.to_string()));
}

/// Test deleting a group state
#[tokio::test]
async fn test_delete_group_state() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;

    let group_id = Uuid::new_v4();

    // Create a group state
    alice.upsert_group_state(&app, group_id, 1, b"state").await;

    // Delete it
    let response = alice.delete_group_state(&app, group_id).await;
    assert_status(response, 204).await;

    // Verify it's gone
    let response = alice.get_group_state(&app, group_id).await;
    assert_error(response, 404, "Group state not found").await;

    // Listing should be empty
    let response = alice.get_all_group_states(&app).await;
    let response = assert_status(response, 200).await;
    let body: Vec<Value> = response.json().await.unwrap();
    assert_eq!(body.len(), 0);
}

/// Test deleting a nonexistent group state returns 404
#[tokio::test]
async fn test_delete_group_state_not_found() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;

    let group_id = Uuid::new_v4();
    let response = alice.delete_group_state(&app, group_id).await;
    assert_error(response, 404, "Group state not found").await;
}

/// Test that users cannot access each other's group states
#[tokio::test]
async fn test_group_state_isolation_between_users() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    let group_id = Uuid::new_v4();

    // Alice stores a group state
    alice
        .upsert_group_state(&app, group_id, 1, b"alice-state")
        .await;

    // Bob stores the same group_id (his own copy)
    bob.upsert_group_state(&app, group_id, 2, b"bob-state")
        .await;

    // Alice's state should be her own
    let response = alice.get_group_state(&app, group_id).await;
    let response = assert_status(response, 200).await;
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["epoch"], 1);
    let content: Vec<u8> = serde_json::from_value(body["encryptedContent"].clone()).unwrap();
    assert_eq!(content, b"alice-state");

    // Bob's state should be his own
    let response = bob.get_group_state(&app, group_id).await;
    let response = assert_status(response, 200).await;
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["epoch"], 2);
    let content: Vec<u8> = serde_json::from_value(body["encryptedContent"].clone()).unwrap();
    assert_eq!(content, b"bob-state");

    // Alice's list should only contain her groups
    let response = alice.get_all_group_states(&app).await;
    let response = assert_status(response, 200).await;
    let body: Vec<Value> = response.json().await.unwrap();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["epoch"], 1);
}

/// Test that a user cannot PUT to another user's group state
#[tokio::test]
async fn test_upsert_group_state_forbidden_for_other_user() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    let group_id = Uuid::new_v4();

    // Alice tries to PUT to Bob's group state URL using her token
    let url = format!("{}/users/{}/groups/{}", app.address, bob.uid, group_id);
    let response = app
        .client
        .put(&url)
        .bearer_auth(&alice.devices[0].token)
        .json(&serde_json::json!({
            "epoch": 1,
            "encryptedContent": [1, 2, 3],
        }))
        .send()
        .await
        .unwrap();
    assert_error(response, 403, "Cannot modify another user's group state").await;
}

/// Test that a user cannot GET another user's group state
#[tokio::test]
async fn test_get_group_state_forbidden_for_other_user() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    let group_id = Uuid::new_v4();
    bob.upsert_group_state(&app, group_id, 1, b"bob-secret")
        .await;

    // Alice tries to GET Bob's group state
    let url = format!("{}/users/{}/groups/{}", app.address, bob.uid, group_id);
    let response = app
        .client
        .get(&url)
        .bearer_auth(&alice.devices[0].token)
        .send()
        .await
        .unwrap();
    assert_error(response, 403, "Cannot read another user's group state").await;
}

/// Test that a user cannot DELETE another user's group state
#[tokio::test]
async fn test_delete_group_state_forbidden_for_other_user() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    let group_id = Uuid::new_v4();
    bob.upsert_group_state(&app, group_id, 1, b"bob-secret")
        .await;

    // Alice tries to DELETE Bob's group state
    let url = format!("{}/users/{}/groups/{}", app.address, bob.uid, group_id);
    let response = app
        .client
        .delete(&url)
        .bearer_auth(&alice.devices[0].token)
        .send()
        .await
        .unwrap();
    assert_error(response, 403, "Cannot delete another user's group state").await;
}

/// Test that unauthenticated requests are rejected
#[tokio::test]
async fn test_group_state_unauthenticated_fails() {
    let app = spawn_app().await;
    let alice = TestUser::create(&app, "alice").await;
    let group_id = Uuid::new_v4();

    // PUT without token
    let url = format!("{}/users/{}/groups/{}", app.address, alice.uid, group_id);
    let response = app
        .client
        .put(&url)
        .json(&serde_json::json!({
            "epoch": 1,
            "encryptedContent": [1, 2, 3],
        }))
        .send()
        .await
        .unwrap();
    assert_status(response, 401).await;

    // GET without token
    let response = app.client.get(&url).send().await.unwrap();
    assert_status(response, 401).await;

    // GET all without token
    let url_all = format!("{}/users/{}/groups", app.address, alice.uid);
    let response = app.client.get(&url_all).send().await.unwrap();
    assert_status(response, 401).await;

    // DELETE without token
    let response = app.client.delete(&url).send().await.unwrap();
    assert_status(response, 401).await;
}

/// Test that different devices of the same user can read/write group state
#[tokio::test]
async fn test_group_state_device_sync() {
    let app = spawn_app().await;
    let mut alice = TestUser::create(&app, "alice").await;
    alice.add_device(&app, "alice-tablet").await;
    assert_eq!(alice.device_count(), 2);

    let group_id = Uuid::new_v4();

    // Device 0 creates the group state
    let response = alice
        .upsert_group_state_with_device(&app, 0, group_id, 1, b"initial-state")
        .await;
    assert_status(response, 200).await;

    // Device 1 can read it (device sync)
    let response = alice.get_group_state_with_device(&app, 1, group_id).await;
    let response = assert_status(response, 200).await;
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["epoch"], 1);
    let content: Vec<u8> = serde_json::from_value(body["encryptedContent"].clone()).unwrap();
    assert_eq!(content, b"initial-state");

    // Device 1 updates it with a higher epoch
    let response = alice
        .upsert_group_state_with_device(&app, 1, group_id, 2, b"updated-state")
        .await;
    assert_status(response, 200).await;

    // Device 0 sees the updated state
    let response = alice.get_group_state_with_device(&app, 0, group_id).await;
    let response = assert_status(response, 200).await;
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["epoch"], 2);
    let content: Vec<u8> = serde_json::from_value(body["encryptedContent"].clone()).unwrap();
    assert_eq!(content, b"updated-state");

    // Device 1 can list all groups
    let response = alice.get_all_group_states_with_device(&app, 1).await;
    let response = assert_status(response, 200).await;
    let body: Vec<Value> = response.json().await.unwrap();
    assert_eq!(body.len(), 1);

    // Device 0 can delete it
    let response = alice
        .delete_group_state_with_device(&app, 0, group_id)
        .await;
    assert_status(response, 204).await;

    // Device 1 confirms it's gone
    let response = alice.get_group_state_with_device(&app, 1, group_id).await;
    assert_error(response, 404, "Group state not found").await;
}
