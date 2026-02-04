use crate::common::*;

/// Test that Delivered activity deletes the delivery request for a Create activity
#[tokio::test]
async fn test_delivered_deletes_delivery_request() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    // Alice sends a message to Bob
    let response = alice.send_message_to(&app, &bob, "Test message").await;
    let create_response = assert_success(response).await;
    let create_activity: serde_json::Value = create_response.json().await.unwrap();
    let create_id = create_activity["id"].as_str().unwrap();

    // Verify Bob has the message in inbox
    let bob_inbox = bob.get_inbox(&app).await;
    assert_collection_size(&bob_inbox, 1);

    // Bob sends a Delivered activity for the Create
    let delivered_response = bob.send_delivered(&app, create_id, &alice).await;
    assert_success(delivered_response).await;

    // Bob's inbox should now be empty (delivery request deleted)
    let bob_inbox_after = bob.get_inbox(&app).await;
    assert_collection_size(&bob_inbox_after, 0);
}

/// Test that Delivered activity is sent to ALL sender's devices (including the one that sent the original message)
#[tokio::test]
async fn test_delivered_fanout_to_sender_devices() {
    let app = spawn_app().await;

    let mut alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    // Alice adds two more devices
    alice.add_device(&app, "alice-laptop").await;
    alice.add_device(&app, "alice-tablet").await;
    assert_eq!(alice.device_count(), 3);

    // Alice sends a message to Bob from device 0
    let response = alice.send_message_to(&app, &bob, "Test message").await;
    let create_response = assert_success(response).await;
    let create_activity: serde_json::Value = create_response.json().await.unwrap();
    let create_id = create_activity["id"].as_str().unwrap();

    // Bob sends a Delivered activity
    let delivered_response = bob.send_delivered(&app, create_id, &alice).await;
    assert_success(delivered_response).await;

    // ALL of Alice's devices should receive the Delivered activity
    // Device 0 gets only the Delivered (it sent the message to Bob, so no sync Create)
    let device0_inbox = alice.get_inbox_with_device(&app, 0).await;
    assert_collection_size(&device0_inbox, 1);
    assert_activity_type(&device0_inbox["orderedItems"][0], "Delivered");

    // Devices 1 and 2 get both the sync Create AND the Delivered
    let device1_inbox = alice.get_inbox_with_device(&app, 1).await;
    let device2_inbox = alice.get_inbox_with_device(&app, 2).await;

    assert_collection_size(&device1_inbox, 2);
    assert_collection_size(&device2_inbox, 2);

    // One should be Create (from device sync), one should be Delivered
    let device1_types: Vec<&str> = device1_inbox["orderedItems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["type"].as_str().unwrap())
        .collect();
    assert!(device1_types.contains(&"Create"));
    assert!(device1_types.contains(&"Delivered"));
}

/// Test that Delivered activity is only sent once (first delivery claim)
/// When multiple recipient devices send Delivered, only the first creates an inbox entry for the sender
#[tokio::test]
async fn test_delivered_sent_only_once() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;
    let mut bob = TestUser::create(&app, "bob").await;

    // Bob adds a second device
    bob.add_device(&app, "bob-laptop").await;

    // Alice sends a message to Bob (both Bob's devices receive it)
    let response = alice.send_message_to(&app, &bob, "Test message").await;
    let create_response = assert_success(response).await;
    let create_activity: serde_json::Value = create_response.json().await.unwrap();
    let create_id = create_activity["id"].as_str().unwrap();

    // Verify both Bob devices have the message
    let bob_inbox_0 = bob.get_inbox_with_device(&app, 0).await;
    let bob_inbox_1 = bob.get_inbox_with_device(&app, 1).await;
    assert_collection_size(&bob_inbox_0, 1);
    assert_collection_size(&bob_inbox_1, 1);

    // Bob's first device sends a Delivered
    let delivered_response = bob
        .send_delivered_from_device(&app, 0, create_id, &alice)
        .await;
    assert_success(delivered_response).await;

    // Alice should receive the Delivered (first delivery claimed)
    let alice_inbox = alice.get_inbox(&app).await;
    assert_collection_size(&alice_inbox, 1);
    assert_activity_type(&alice_inbox["orderedItems"][0], "Delivered");

    // Bob's device 0 should no longer have the Create (delivery deleted)
    let bob_inbox_0_after = bob.get_inbox_with_device(&app, 0).await;
    assert_collection_size(&bob_inbox_0_after, 0);

    // Bob's second device also sends a Delivered for the same Create
    let delivered_response_2 = bob
        .send_delivered_from_device(&app, 1, create_id, &alice)
        .await;
    assert_success(delivered_response_2).await;

    // Bob's device 1 should also have the delivery deleted
    let bob_inbox_1_after = bob.get_inbox_with_device(&app, 1).await;
    assert_collection_size(&bob_inbox_1_after, 0);

    // Alice should still only have ONE Delivered (the first one, already consumed from previous get)
    // No new Delivered should be created because first_delivery was already claimed
    let alice_inbox_after = alice.get_inbox(&app).await;
    assert_collection_size(&alice_inbox_after, 0);
}

/// Test that Delivered to non-existent Create is ignored gracefully
#[tokio::test]
async fn test_delivered_for_nonexistent_create() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    // Bob sends a Delivered for a Create that doesn't exist
    let fake_create_id = "https://example.com/activities/nonexistent-id";
    let delivered_response = bob.send_delivered(&app, fake_create_id, &alice).await;

    // Should succeed (the activity is ignored per spec)
    assert_success(delivered_response).await;

    // Alice's inbox should remain empty
    let alice_inbox = alice.get_inbox(&app).await;
    assert_collection_size(&alice_inbox, 0);
}

/// Test that getting a Delivered from inbox deletes the delivery request
#[tokio::test]
async fn test_get_delivered_from_inbox_deletes_delivery() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    // Alice sends a message to Bob (single device, no sync message)
    let response = alice.send_message_to(&app, &bob, "Test message").await;
    let create_response = assert_success(response).await;
    let create_activity: serde_json::Value = create_response.json().await.unwrap();
    let create_id = create_activity["id"].as_str().unwrap();

    // Bob sends a Delivered
    let delivered_response = bob.send_delivered(&app, create_id, &alice).await;
    assert_success(delivered_response).await;

    // Alice should have the Delivered in inbox
    let alice_inbox = alice.get_inbox(&app).await;
    assert_collection_size(&alice_inbox, 1);
    assert_activity_type(&alice_inbox["orderedItems"][0], "Delivered");

    // Get inbox again - the Delivered should be deleted after first retrieval
    let alice_inbox_after = alice.get_inbox(&app).await;
    assert_collection_size(&alice_inbox_after, 0);
}

/// Test multiple Create activities with separate Delivered responses
#[tokio::test]
async fn test_multiple_creates_with_delivered() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    // Alice sends 3 messages to Bob
    let mut create_ids = Vec::new();
    for i in 1..=3 {
        let response = alice
            .send_message_to(&app, &bob, &format!("Message {}", i))
            .await;
        let create_response = assert_success(response).await;
        let create_activity: serde_json::Value = create_response.json().await.unwrap();
        let create_id = create_activity["id"].as_str().unwrap().to_string();
        create_ids.push(create_id);
    }

    // Bob should have 3 messages
    let bob_inbox = bob.get_inbox(&app).await;
    assert_collection_size(&bob_inbox, 3);

    // Bob sends Delivered for each Create
    for create_id in &create_ids {
        let delivered_response = bob.send_delivered(&app, create_id, &alice).await;
        assert_success(delivered_response).await;
    }

    // Bob's inbox should now be empty (all deliveries deleted)
    let bob_inbox_after = bob.get_inbox(&app).await;
    assert_collection_size(&bob_inbox_after, 0);

    // Alice should have 3 Delivered activities
    let alice_inbox = alice.get_inbox(&app).await;
    assert_collection_size(&alice_inbox, 3);

    // All should be Delivered activities
    for i in 0..3 {
        assert_activity_type(&alice_inbox["orderedItems"][i], "Delivered");
    }
}

/// Test that Delivered doesn't affect other users' delivery requests
#[tokio::test]
async fn test_delivered_isolation_between_users() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;
    let mut bob = TestUser::create(&app, "bob").await;
    let charlie = TestUser::create(&app, "charlie").await;

    bob.add_device(&app, "bob-laptop").await;

    // Alice sends messages to both Bob and Charlie
    let response_to_bob = alice.send_message_to(&app, &bob, "Message to Bob").await;
    let create_response_bob = assert_success(response_to_bob).await;
    let create_activity_bob: serde_json::Value = create_response_bob.json().await.unwrap();
    let create_id_bob = create_activity_bob["id"].as_str().unwrap();

    alice
        .send_message_to(&app, &charlie, "Message to Charlie")
        .await;

    // Bob sends Delivered
    let delivered_response = bob
        .send_delivered_from_device(&app, 0, create_id_bob, &alice)
        .await;
    assert_success(delivered_response).await;

    // Bob's first device should no longer have the message
    let bob_inbox_0 = bob.get_inbox_with_device(&app, 0).await;
    assert_collection_size(&bob_inbox_0, 0);

    // But Bob's second device should still have it
    let bob_inbox_1 = bob.get_inbox_with_device(&app, 1).await;
    assert_collection_size(&bob_inbox_1, 1);

    // Charlie's inbox should be unaffected
    let charlie_inbox = charlie.get_inbox(&app).await;
    assert_collection_size(&charlie_inbox, 1);
}
