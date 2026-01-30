use reqwest::Response;
use serde_json::Value;

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
