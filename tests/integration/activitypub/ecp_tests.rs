use crate::common::*;

/// Test getting ECP capabilities endpoint
#[tokio::test]
async fn test_well_known_ecp() {
    let app = spawn_app().await;

    let ecp_url = format!("{}/.well-known/ecp", &app.address);

    let response = app
        .client
        .get(&ecp_url)
        .send()
        .await
        .expect("Failed to get ECP capabilities");

    let response = assert_status(response, 200).await;

    // Parse capabilities
    let body_text = response.text().await.unwrap();
    let capabilities: serde_json::Value =
        serde_json::from_str(&body_text).expect("Failed to parse ECP capabilities response");

    // Verify required fields
    assert_has_field(&capabilities, "spec");

    assert!(
        capabilities["spec"].is_string(),
        "spec field should be a string"
    );

    assert_field_equals(&capabilities, "protocol", &"eko-chat".into());
}
