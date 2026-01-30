use crate::common::*;

/// Test sending a message from one user to another on the same server
#[tokio::test]
async fn test_send_message_between_local_users() {
    let app = spawn_app().await;

    // Create two users
    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    // Alice sends a message to Bob
    let response = alice.send_message_to(&app, &bob, "Hello Bob!").await;

    // Verify the message was accepted
    assert_success(response).await;

    // Bob should have the message in his inbox
    let bob_inbox = bob.get_inbox(&app).await;
    assert_collection_size(&bob_inbox, 1);

    // The activity should be a Create activity
    let activity = &bob_inbox["orderedItems"][0];
    assert_activity_type(activity, "Create");

    // The actor should be Alice
    assert_field_equals(activity, "actor", &alice.actor_id.into());
}

/// Test sending a message to yourself
#[tokio::test]
async fn test_send_message_to_self() {
    let app = spawn_app().await;

    let mut alice = TestUser::create(&app, "alice").await;

    // Alice adds two more devices
    alice.add_device(&app, "alice-1").await;
    alice.add_device(&app, "alice-2").await;
    assert_eq!(alice.device_count(), 3);

    // Alice sends a message to herself from device 0
    let response = alice
        .send_message_from_device(&app, 0, &alice, "Note to self")
        .await;
    assert_success(response).await;

    // All of Alice's devices except device 0 (the sender) should receive the message
    assert_all_devices_received_message(
        &app,
        &alice,
        1,
        Some(&alice.actor_id),
        Some(b"Note to self"),
        Some(0),
    )
    .await;

    // Verify that device 0 did NOT receive the message
    let device0_inbox = alice.get_inbox_with_device(&app, 0).await;
    assert_collection_size(&device0_inbox, 0);
}

/// Test empty inbox for new user
#[tokio::test]
async fn test_new_user_has_empty_inbox() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;

    let inbox = alice.get_inbox(&app).await;
    assert_collection_size(&inbox, 0);
}

/// Test concurrent message delivery to same recipient
#[tokio::test]
async fn test_concurrent_messages_to_same_user() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    // Alice sends 3 messages rapidly
    let messages: Vec<String> = (1..=3).map(|i| format!("Message {}", i)).collect();
    let mut handles = vec![];
    for message in &messages {
        let fut = alice.send_message_to(&app, &bob, message);
        handles.push(fut);
    }

    // Wait for all to complete
    for handle in handles {
        let response = handle.await;
        assert_success(response).await;
    }

    // Bob should have all 3 messages
    let bob_inbox = bob.get_inbox(&app).await;
    assert_collection_size(&bob_inbox, 3);
}

/// Test sending a message to a user with multiple devices
#[tokio::test]
async fn test_send_message_to_user_with_multiple_devices() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;
    let mut bob = TestUser::create(&app, "bob").await;

    // Bob adds two more devices
    bob.add_device(&app, "bob-laptop").await;
    bob.add_device(&app, "bob-tablet").await;
    assert_eq!(bob.device_count(), 3);

    // Alice sends a message to Bob
    let response = alice.send_message_to(&app, &bob, "Hello Bob!").await;
    assert_success(response).await;

    // All of Bob's devices should receive the same message with correct sender and content
    assert_all_devices_received_message(
        &app,
        &bob,
        1,
        Some(&alice.actor_id),
        Some(b"Hello Bob!"),
        Some(0),
    )
    .await;
}

/// Test multiple messages to a user with multiple devices
#[tokio::test]
async fn test_multiple_messages_to_multi_device_user() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;
    let mut bob = TestUser::create(&app, "bob").await;

    // Bob adds a second device
    bob.add_device(&app, "bob-phone").await;
    assert_eq!(bob.device_count(), 2);

    // Alice sends multiple messages to Bob
    for i in 1..=3 {
        let response = alice
            .send_message_to(&app, &bob, &format!("Message {}", i))
            .await;
        assert_success(response).await;
    }

    // All of Bob's devices should have all 3 messages from Alice
    // Vrify the count and sender, but not specific content since there are multiple messages
    assert_all_devices_received_message(&app, &bob, 3, Some(&alice.actor_id), None, Some(0)).await;
}
