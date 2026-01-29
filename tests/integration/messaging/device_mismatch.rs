use crate::common::*;

/// Test that sending a message without encrypting for all devices is rejected
///
/// This test verifies that when a user has multiple devices, a message must
/// include encrypted content for ALL devices, not just a subset.
#[tokio::test]
async fn test_device_count_mismatch_rejected() {
    let app = spawn_app().await;

    let mut alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    // Alice adds a second device
    alice.add_device(&app, "alice-tablet").await;
    assert_eq!(alice.device_count(), 2);

    // Bob has 1 device from login
    assert_eq!(bob.device_count(), 1);

    // Build a message that only targets ONE of Alice's devices (should target both)
    let alice_device1_url = alice.devices[0].url.clone();
    let envelope = SignalEnvelope::new()
        .add_device_message(bob.devices[0].url.clone(), alice_device1_url, "incomplete message")
        .build_message(&bob.actor_id, &alice.actor_id);

    let response = bob.send_envelope(&app, envelope).await;

    // Server validates and rejects
    assert_error(response, 400, "device_list_mismatch").await;
}

/// Test that a message with correct device count is accepted
///
/// This test verifies that when all of a user's devices are properly targeted,
/// the message is accepted successfully.
#[tokio::test]
async fn test_correct_device_count_accepted() {
    let app = spawn_app().await;

    let mut alice = TestUser::create(&app, "alice").await;
    let bob = TestUser::create(&app, "bob").await;

    // Alice starts with 1 device, add a second
    alice.add_device(&app, "alice-laptop").await;
    assert_eq!(alice.device_count(), 2);

    // Bob has 1 device from login
    assert_eq!(bob.device_count(), 1);

    let response = bob.send_message_to(&app, &alice, "hello alice").await;

    // Should be accepted
    assert_success(response).await;
}

/// Test sending from a specific device
#[tokio::test]
async fn test_send_from_specific_device() {
    let app = spawn_app().await;

    let alice = TestUser::create(&app, "alice").await;
    let mut bob = TestUser::create(&app, "bob").await;

    // Bob adds a second device
    bob.add_device(&app, "bob-laptop").await;
    assert_eq!(bob.device_count(), 2);

    // Send from Bob's second device (index 1) to Alice
    let response = bob
        .send_message_from_device(&app, 1, &alice, "hello from laptop")
        .await;

    // Should be accepted
    assert_success(response).await;
}
