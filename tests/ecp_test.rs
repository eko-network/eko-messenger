mod common;

use common::spawn_app;
use serde_json::Value;

#[tokio::test]
async fn test_well_known_ecp() {
    let app = spawn_app().await;
    let client = &app.client;

    let ecp_url = format!("{}/.well-known/ecp", &app.address);

    let res = client
        .get(&ecp_url)
        .send()
        .await
        .expect("Failed to get ECP capabilities");

    let status = res.status().as_u16();
    let body = res.text().await.unwrap();
    assert_eq!(status, 200, "Expected OK status, got {}: {}", status, body);

    let capabilities: Value =
        serde_json::from_str(&body).expect("Failed to parse ECP capabilities response");

    assert!(
        capabilities["spec"].is_string(),
        "spec field should be a string"
    );

    assert_eq!(
        capabilities["protocol"], "eko-chat",
        "Protocol should be eko-chat"
    );
}
