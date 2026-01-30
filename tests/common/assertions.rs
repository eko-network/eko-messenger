use crate::common::{TestApp, TestUser};
use reqwest::Response;
use serde_json::Value;

/// Assert that all devices of a user received the same message in their inbox
///
/// * `user` whose devices should be checked
/// * `expected_count` expected number of messages in each device's inbox
/// * `expected_sender` Optional actor ID of the expected sender
/// * `expected_content_bytes` Optional byte content to verify in at least one message
/// * `exclude_sending_device` Optional device index to exclude from verification
pub async fn assert_all_devices_received_message(
    app: &TestApp,
    user: &TestUser,
    expected_count: usize,
    expected_sender: Option<&str>,
    expected_content_bytes: Option<&[u8]>,
    exclude_sending_device: Option<usize>,
) {
    assert!(
        user.device_count() > 0,
        "User {} has no devices",
        user.username
    );

    // Find the first device to use as reference
    let first_device_idx = (0..user.device_count())
        .find(|&idx| Some(idx) != exclude_sending_device)
        .unwrap_or_else(|| {
            panic!(
                "User {} must have at least one non-sending device to check. Device count: {}, excluded: {:?}",
                user.username,
                user.device_count(),
                exclude_sending_device
            )
        });

    // Get inbox from first non-excluded device as reference
    let first_inbox = user.get_inbox_with_device(app, first_device_idx).await;
    assert_collection_size(&first_inbox, expected_count);

    let first_items = first_inbox["orderedItems"]
        .as_array()
        .or_else(|| first_inbox["items"].as_array())
        .expect("First inbox should have orderedItems or items");

    // Verify all other devices have matching message content (skip the sending device if specified)
    for device_idx in 0..user.device_count() {
        // Skip the reference device and the sending device
        if device_idx == first_device_idx || Some(device_idx) == exclude_sending_device {
            continue;
        }

        let device_inbox = user.get_inbox_with_device(app, device_idx).await;

        // Check count matches
        assert_collection_size(&device_inbox, expected_count);

        let device_items = device_inbox["orderedItems"]
            .as_array()
            .or_else(|| device_inbox["items"].as_array())
            .expect("Device inbox should have orderedItems or items");

        // Compare each message's text content
        assert_eq!(
            first_items.len(),
            device_items.len(),
            "Device {} has different message count than device {}",
            device_idx,
            first_device_idx
        );

        for (msg_idx, (first_msg, device_msg)) in
            first_items.iter().zip(device_items.iter()).enumerate()
        {
            // Check actor is the same
            assert_eq!(
                first_msg["actor"], device_msg["actor"],
                "Device {} message {} has different actor than device {}",
                device_idx, msg_idx, first_device_idx
            );

            // Check activity type is the same
            assert_eq!(
                first_msg["type"], device_msg["type"],
                "Device {} message {} has different type than device {}",
                device_idx, msg_idx, first_device_idx
            );

            // Check object attributedTo is the same
            assert_eq!(
                first_msg["object"]["attributedTo"], device_msg["object"]["attributedTo"],
                "Device {} message {} has different attributedTo than device {}",
                device_idx, msg_idx, first_device_idx
            );

            // Check that the encrypted content exists for both (the "to" field will differ per device)
            let first_content = first_msg["object"]["content"]
                .as_array()
                .expect("Message should have content array");
            let device_content = device_msg["object"]["content"]
                .as_array()
                .expect("Message should have content array");

            // Both should have content for their respective devices
            assert!(
                !first_content.is_empty() && !device_content.is_empty(),
                "Device {} message {} missing encrypted content",
                device_idx,
                msg_idx
            );

            // Verify the actual encrypted bytes are the same (same message content)
            if let (Some(first_entry), Some(device_entry)) =
                (first_content.first(), device_content.first())
            {
                assert_eq!(
                    first_entry["content"], device_entry["content"],
                    "Device {} message {} has different encrypted content than device {}",
                    device_idx, msg_idx, first_device_idx
                );
            }
        }
    }

    // Verify sender if provided
    if let Some(sender) = expected_sender {
        let items = first_inbox["orderedItems"]
            .as_array()
            .or_else(|| first_inbox["items"].as_array())
            .expect("Inbox should have orderedItems or items");

        for (idx, item) in items.iter().enumerate() {
            let actor = item["actor"]
                .as_str()
                .unwrap_or_else(|| panic!("Message {} should have an actor field", idx));

            assert_eq!(
                actor, sender,
                "Message {} has wrong sender: expected '{}', got '{}'",
                idx, sender, actor
            );
        }
    }

    // Verify content if provided
    if let Some(expected_bytes) = expected_content_bytes {
        let items = first_inbox["orderedItems"]
            .as_array()
            .or_else(|| first_inbox["items"].as_array())
            .expect("Inbox should have orderedItems or items");

        // Find at least one message with matching content for this device
        let mut found_matching_content = false;

        for item in items.iter() {
            // Navigate to the encrypted message content array
            if let Some(content_array) = item["object"]["content"].as_array() {
                for entry in content_array {
                    // Check if this entry is for one of the user's devices
                    let to_did = entry["to"].as_str();
                    let is_for_user_device = to_did
                        .map(|did| user.devices.iter().any(|d| d.url == did))
                        .unwrap_or(false);

                    if is_for_user_device {
                        // Check the content - it could be a base64 string or an array of bytes
                        if let Some(content_str) = entry["content"].as_str() {
                            // Content is a base64 encoded string, decode it
                            if let Ok(decoded) = base64::Engine::decode(
                                &base64::engine::general_purpose::STANDARD,
                                content_str,
                            ) {
                                if decoded == expected_bytes {
                                    found_matching_content = true;
                                    break;
                                }
                            }
                        } else if let Some(content) = entry["content"].as_array() {
                            // Content is an array of bytes
                            let content_bytes: Vec<u8> = content
                                .iter()
                                .filter_map(|v| v.as_u64().map(|n| n as u8))
                                .collect();
                            if content_bytes == expected_bytes {
                                found_matching_content = true;
                                break;
                            }
                        }
                    }
                }
            }

            if found_matching_content {
                break;
            }
        }

        assert!(
            found_matching_content,
            "No message found with expected content for user {}. Expected bytes: {:?}",
            user.username, expected_bytes
        );
    }
}

/// Assert that an HTTP response has the expected status code
pub async fn assert_status(response: Response, expected: u16) -> Response {
    let status = response.status();
    assert_eq!(
        status.as_u16(),
        expected,
        "Expected status {}, got {}",
        expected,
        status
    );
    response
}

/// Assert that a response is successful (any 2xx response)
pub async fn assert_success(response: Response) -> Response {
    let status = response.status();
    assert!(
        status.is_success(),
        "Expected success status, got {}",
        status
    );
    response
}

/// Assert that an ActivityPub collection has the expected number of items
pub fn assert_collection_size(collection: &Value, expected_size: usize) {
    let items = collection["items"]
        .as_array()
        .or_else(|| collection["orderedItems"].as_array())
        .expect("Collection should have items or orderedItems");

    assert_eq!(
        items.len(),
        expected_size,
        "Expected {} items in collection, got {}",
        expected_size,
        items.len()
    );
}

/// Assert that an ActivityPub activity has the expected type
pub fn assert_activity_type(activity: &Value, expected_type: &str) {
    let activity_type = activity["type"]
        .as_str()
        .expect("Activity should have a type field");

    assert_eq!(
        activity_type, expected_type,
        "Expected activity type '{}', got '{}'",
        expected_type, activity_type
    );
}

/// Assert that a JSON value has a specific field
pub fn assert_has_field(value: &Value, field: &str) {
    assert!(
        value.get(field).is_some(),
        "Expected field '{}' to exist in JSON",
        field
    );
}

/// Assert that a JSON value has a specific field with expected value
pub fn assert_field_equals(value: &Value, field: &str, expected: &Value) {
    let actual = value
        .get(field)
        .unwrap_or_else(|| panic!("Expected field '{}' to exist", field));

    assert_eq!(
        actual, expected,
        "Field '{}': expected {:?}, got {:?}",
        field, expected, actual
    );
}

/// Assert that a response has an expected status and error message in the body
pub async fn assert_error(response: Response, expected_status: u16, expected_error: &str) {
    let response = assert_status(response, expected_status).await;
    let body_text = response.text().await.expect("Failed to read response body");
    let body: Value =
        serde_json::from_str(&body_text).expect("Failed to parse response body as JSON");

    let error = body["error"]
        .as_str()
        .expect("Response body should have an 'error' field");

    assert_eq!(
        error, expected_error,
        "Expected error '{}', got '{}'",
        expected_error, error
    );
}
