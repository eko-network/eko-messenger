use crate::common::{TestApp, assert_success};
use eko_messenger::activitypub::{Activity, Create, EncryptedMessage, EncryptedMessageEntry};
use eko_messenger::devices::DeviceId;
use serde_json::{Value, json};

pub struct TestDevice {
    pub id: DeviceId,
    pub url: String,
    pub token: String,
    pub name: String,
}

impl TestDevice {
    pub fn new(id: DeviceId, url: String, token: String, name: String) -> Self {
        Self {
            id,
            url,
            token,
            name,
        }
    }
}

pub struct TestUser {
    pub username: String,
    pub email: String,
    pub password: String,
    pub uid: String,
    pub actor_id: String,
    pub devices: Vec<TestDevice>,
}

impl TestUser {
    /// Creates a new test user via storage + SessionManager
    /// Creating a TestUser also automatically creates that user's first device
    pub async fn create(app: &TestApp, username: &str) -> Self {
        let email = format!("{}@example.com", username);
        let password = "password"; // Not used, kept for consistency

        // Create user directly via storage (no HTTP)
        let uid = app.create_test_user(username, &email).await;

        // Create actor
        let actor = eko_messenger::activitypub::create_person(
            &app.domain,
            &uid,
            None,                 // summary
            username.to_string(), // preferred_username
            None,                 // name
            None,                 // profile_picture
        );

        // Complete login directly via SessionManager (creates first device)
        let login_response = app
            .sessions
            .complete_login(
                &uid,
                actor,
                "default",
                &vec![1, 2, 3], // test identity key
                123,            // test registration_id
                &vec![eko_messenger::auth::PreKey {
                    id: 1,
                    key: vec![4, 5, 6],
                }],
                &eko_messenger::auth::SignedPreKey {
                    id: 1,
                    key: vec![7, 8, 9],
                    signature: vec![10, 11, 12],
                },
                "127.0.0.1",
                "test-agent",
            )
            .await
            .expect("Login failed");

        // Unwrap Json wrapper
        let login_response = login_response.0;

        let did = DeviceId::from_url(&login_response.did)
            .expect("Failed to parse device ID from login response");

        let first_device = TestDevice::new(
            did,
            login_response.did.clone(),
            login_response.access_token.clone(),
            "default".to_string(),
        );

        Self {
            username: username.to_string(),
            email,
            password: password.to_string(),
            uid: login_response.uid.clone(),
            actor_id: app.actor_url(&login_response.uid),
            devices: vec![first_device],
        }
    }

    /// Get the number of devices for a user
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Add a device by calling SessionManager directly
    /// Each device registration creates a new device with keys
    pub async fn add_device(&mut self, app: &TestApp, device_name: &str) -> &TestDevice {
        // Create actor
        let actor = eko_messenger::activitypub::create_person(
            &app.domain,
            &self.uid,
            None,                  // summary
            self.username.clone(), // preferred_username
            None,                  // name
            None,                  // profile_picture
        );

        // Register new device directly via SessionManager
        let login_response = app
            .sessions
            .complete_login(
                &self.uid,
                actor,
                device_name,
                &vec![1, 2, 3], // test identity key
                123,            // test registration_id
                &vec![eko_messenger::auth::PreKey {
                    id: 1,
                    key: vec![4, 5, 6],
                }],
                &eko_messenger::auth::SignedPreKey {
                    id: 1,
                    key: vec![7, 8, 9],
                    signature: vec![10, 11, 12],
                },
                "127.0.0.1",
                "test-agent",
            )
            .await
            .expect("Failed to add device");

        // Unwrap Json wrapper
        let login_response = login_response.0;

        let new_did = DeviceId::from_url(&login_response.did)
            .expect("Failed to parse device ID from login response");

        let new_device = TestDevice::new(
            new_did,
            login_response.did.clone(),
            login_response.access_token.clone(),
            device_name.to_string(),
        );

        self.devices.push(new_device);
        self.devices.last().unwrap()
    }

    /// Send a message to another user from this user's first device
    /// (when youre not testing which device is sending a message)
    /// Creates encrypted messages for all devices
    pub async fn send_message_to(
        &self,
        app: &TestApp,
        recipient: &TestUser,
        content: &str,
    ) -> reqwest::Response {
        self.send_message_from_device(app, 0, recipient, content)
            .await
    }

    /// Send a message from a specific device to another user
    /// Creates encrypted messages for the recipient's devices
    ///
    /// `device_index` is the index into this user's devices array
    pub async fn send_message_from_device(
        &self,
        app: &TestApp,
        device_index: usize,
        recipient: &TestUser,
        content: &str,
    ) -> reqwest::Response {
        let sender_device = self.devices.get(device_index).unwrap_or_else(|| {
            panic!(
                "Device index {} out of bounds for user {} (has {} devices)",
                device_index,
                self.username,
                self.devices.len()
            )
        });

        // Check if sending to self or to another user
        if self.actor_id != recipient.actor_id {
            // Send to sender's other devices and recipient's devices
            if self.device_count() > 1 {
                self.send_to_own_devices(app, device_index, &sender_device.url, content)
                    .await;
            }

            // Send to recipient's devices
            let envelope = SignalEnvelope::new()
                .add_messages_for_all_devices(sender_device.url.clone(), recipient, content)
                .build_message(&self.actor_id, &recipient.actor_id);

            let activity = self.create_message_activity(envelope);
            self.post_to_outbox_with_device(app, activity, device_index)
                .await
        } else {
            // Sending to self: send to own devices (excluding the sending device)
            self.send_to_own_devices(app, device_index, &sender_device.url, content)
                .await
        }
    }

    /// Send a message to the user's own devices (excluding the sending device)
    async fn send_to_own_devices(
        &self,
        app: &TestApp,
        exclude_device_index: usize,
        sender_device_url: &str,
        content: &str,
    ) -> reqwest::Response {
        if self.devices.len() <= 1 {
            panic!(
                "User {} must have more than one device to send to own devices",
                self.username
            );
        }

        let mut envelope = SignalEnvelope::new();
        for (idx, device) in self.devices.iter().enumerate() {
            if idx != exclude_device_index {
                envelope = envelope.add_device_message(
                    sender_device_url.to_string(),
                    device.url.clone(),
                    content,
                );
            }
        }

        let envelope = envelope.build_message(&self.actor_id, &self.actor_id);
        let activity = self.create_message_activity(envelope);
        self.post_to_outbox_with_device(app, activity, exclude_device_index)
            .await
    }

    /// Send a manually constructed envelope to another user from the first device
    pub async fn send_envelope(
        &self,
        app: &TestApp,
        envelope: EncryptedMessage,
    ) -> reqwest::Response {
        let activity = self.create_message_activity(envelope);
        self.post_to_outbox_with_device(app, activity, 0).await
    }

    /// Post an activity to this user's outbox using a specific device's token
    pub async fn post_to_outbox_with_device<T: serde::Serialize>(
        &self,
        app: &TestApp,
        activity: T,
        device_index: usize,
    ) -> reqwest::Response {
        let device = self.devices.get(device_index).unwrap_or_else(|| {
            panic!(
                "Device index {} out of bounds for user {} (has {} devices)",
                device_index,
                self.username,
                self.devices.len()
            )
        });
        let outbox_url = format!("{}/outbox", &self.actor_id);

        app.client
            .post(&outbox_url)
            .bearer_auth(&device.token)
            .header("Content-Type", "application/activity+json")
            .json(&activity)
            .send()
            .await
            .expect("Failed to post to outbox")
    }

    /// Post an activity to this user's outbox using the first device's token
    pub async fn post_to_outbox<T: serde::Serialize>(
        &self,
        app: &TestApp,
        activity: T,
    ) -> reqwest::Response {
        self.post_to_outbox_with_device(app, activity, 0).await
    }

    /// Get this user's inbox using a specific device's token
    pub async fn get_inbox_with_device(&self, app: &TestApp, device_index: usize) -> Value {
        let device = self.devices.get(device_index).unwrap_or_else(|| {
            panic!(
                "Device index {} out of bounds for user {} (has {} devices)",
                device_index,
                self.username,
                self.devices.len()
            )
        });
        let inbox_url = format!("{}/inbox", &self.actor_id);

        let resp = app
            .client
            .get(&inbox_url)
            .bearer_auth(&device.token)
            .header("Accept", "application/activity+json")
            .send()
            .await
            .expect("Failed to get inbox");

        resp.json().await.expect("Failed to parse inbox")
    }

    /// Get this user's inbox using the first device's token
    pub async fn get_inbox(&self, app: &TestApp) -> Value {
        self.get_inbox_with_device(app, 0).await
    }

    /// Get this user's actor profile
    pub async fn get_actor(&self, app: &TestApp) -> Value {
        app.client
            .get(&self.actor_id)
            .header("Accept", "application/activity+json")
            .send()
            .await
            .expect("Failed to get actor")
            .json()
            .await
            .expect("Failed to parse actor")
    }

    /// Create a message activity for a given SignalEnvelope
    pub fn create_message_activity(&self, envelope: EncryptedMessage) -> Activity {
        Activity::Create(Create {
            context: json!("https://www.w3.org/ns/activitystreams"),
            id: None,
            actor: self.actor_id.clone(),
            to: envelope.to.clone(),
            object: envelope,
        })
    }

    /// Create a Delivered activity for a given Create activity ID
    pub fn create_delivered_activity(&self, create_id: &str, recipient: &TestUser) -> Activity {
        Activity::Delivered(eko_messenger::activitypub::Delivered {
            context: json!("https://www.w3.org/ns/activitystreams"),
            id: None,
            actor: self.actor_id.clone(),
            to: recipient.actor_id.clone(),
            object: create_id.to_string(),
        })
    }

    /// Send a Delivered activity from a specific device
    pub async fn send_delivered_from_device(
        &self,
        app: &TestApp,
        device_index: usize,
        create_id: &str,
        recipient: &TestUser,
    ) -> reqwest::Response {
        let activity = self.create_delivered_activity(create_id, recipient);
        self.post_to_outbox_with_device(app, activity, device_index)
            .await
    }

    /// Send a Delivered activity from the first device
    pub async fn send_delivered(
        &self,
        app: &TestApp,
        create_id: &str,
        recipient: &TestUser,
    ) -> reqwest::Response {
        self.send_delivered_from_device(app, 0, create_id, recipient)
            .await
    }
}

/// Builder for Signal protocol encrypted message envelopes
pub struct SignalEnvelope {
    messages: Vec<EncryptedMessageEntry>,
}

impl SignalEnvelope {
    pub fn new() -> Self {
        Self { messages: vec![] }
    }

    /// Add an encrypted message for a specific device
    pub fn add_device_message(
        mut self,
        from_did_url: String,
        to_did_url: String,
        content: &str,
    ) -> Self {
        self.messages.push(EncryptedMessageEntry {
            to: to_did_url,
            from: from_did_url,
            content: content.as_bytes().to_vec(), // FIXME encrypt
        });
        self
    }

    /// Add messages for all devices of a recipient user to the SignalEnvelope
    pub fn add_messages_for_all_devices(
        mut self,
        from_did_url: String,
        recipient: &TestUser,
        content: &str,
    ) -> Self {
        for device in &recipient.devices {
            self.messages.push(EncryptedMessageEntry {
                to: device.url.clone(),
                from: from_did_url.clone(),
                content: content.as_bytes().to_vec(),
            });
        }
        self
    }

    /// Build the EncryptedMessage ActivityPUb payload
    pub fn build_message(self, actor_id: &str, recipient_id: &str) -> EncryptedMessage {
        EncryptedMessage {
            context: json!([
                "https://www.w3.org/ns/activitystreams",
                "https://w3id.org/security/v1"
            ]),
            type_field: "Note".to_string(),
            id: None,
            content: self.messages,
            attributed_to: actor_id.to_string(),
            to: recipient_id.to_string(),
        }
    }
}
