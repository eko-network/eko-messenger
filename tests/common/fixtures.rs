use crate::common::{TestApp, assert_success};
use eko_messenger::activitypub::{Activity, Create, EncryptedMessage, EncryptedMessageEntry};
use eko_messenger::devices::DeviceId;
use serde_json::{Value, json};

pub struct TestUser {
    pub username: String,
    pub email: String,
    pub password: String,
    pub uid: String,
    pub actor_id: String,
    pub token: String,
    pub did_url: String, // TODO for now these will act as the default device inside the tests
    pub did: DeviceId,   // see other TODO that i need to fix
    pub devices: Vec<DeviceId>,
}

impl TestUser {
    /// Creates a new test user via signup and login endpoints
    /// Creating a TestUser also automatically creates that uers first device
    pub async fn create(app: &TestApp, username: &str) -> Self {
        let email = format!("{}@example.com", username);
        let password = "password";

        app.signup_http(&username, &email, password).await;

        // Login to get credentials (and the first device)
        let login_response = app.login_http(&email, password).await;
        let did = DeviceId::from_url(&login_response.did)
            .expect("Failed to parse device ID from login response");

        Self {
            username: username.to_string(),
            email,
            password: password.to_string(),
            uid: login_response.uid.clone(),
            actor_id: app.actor_url(&login_response.uid),
            token: login_response.access_token,
            did_url: login_response.did,
            did,
            devices: vec![did],
        }
    }

    /// Get the number of devices for a user
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Add a device by logging in with new device credentials
    /// Each login creates a new device with keys
    pub async fn add_device(&mut self, app: &TestApp, device_name: &str) -> DeviceId {
        let login_req = app.generate_login_request(
            self.email.clone(),
            self.password.clone(),
            Some(device_name),
        );

        let login_url = format!("{}/auth/v1/login", &app.address);
        let login_res = app
            .client
            .post(&login_url)
            .header("User-Agent", "test-client")
            .json(&login_req)
            .send()
            .await
            .expect("Failed to add device via login");

        let login_res = assert_success(login_res).await;

        let login_response: crate::common::LoginResponse = login_res
            .json()
            .await
            .expect("Failed to parse login response");

        let new_did = DeviceId::from_url(&login_response.did)
            .expect("Failed to parse device ID from login response");

        self.devices.push(new_did);

        new_did
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
    /// Creates encrypted messages for the recipient's devices (FIXME and sender's?)
    ///
    /// `device_index` is the index into this user's devices array
    pub async fn send_message_from_device(
        &self,
        app: &TestApp,
        device_index: usize,
        recipient: &TestUser,
        content: &str,
    ) -> reqwest::Response {
        // TODO im not entirely in love with this way to send from specific devices?
        // maybe if you want to mess around with devices, you create a device and in the
        // test you pass the name or id of the device. or the device object.
        // i think i need to handle devices and keys and stuff better. ill do that when i
        // actually properly handle sending and "encrypting" messages with the keys
        let sender_device = self.devices.get(device_index).unwrap_or_else(|| {
            panic!(
                "Device index {} out of bounds for user {} (has {} devices)",
                device_index,
                self.username,
                self.devices.len()
            )
        });
        let sender_device_url = sender_device.to_url(&app.domain);

        // Build envelope with messages for all recipient devices
        let mut builder = SignalEnvelope::new().add_messages_for_all_devices(
            sender_device_url.clone(),
            recipient,
            app,
            content,
        );

        // Add messages for the sender's devices (exclude the sending device)
        // FIXME shouuldnt we be adding messages from the sender's other devices?
        // for (idx, device) in self.devices.iter().enumerate() {
        //     if idx != device_index {
        //         let device_url = device.to_url(&app.domain);
        //         builder =
        //             builder.add_device_message(sender_device_url.clone(), device_url, content);
        //     }
        // }

        let envelope = builder.build_message(&self.actor_id, &recipient.actor_id);
        let activity = self.create_message_activity(envelope);
        self.post_to_outbox(app, activity).await
    }

    /// Send a manually constructed envelope to another user
    pub async fn send_envelope(
        &self,
        app: &TestApp,
        envelope: EncryptedMessage,
    ) -> reqwest::Response {
        let activity = self.create_message_activity(envelope);
        self.post_to_outbox(app, activity).await
    }

    /// Post an activity to this user's outbox
    pub async fn post_to_outbox<T: serde::Serialize>(
        &self,
        app: &TestApp,
        activity: T,
    ) -> reqwest::Response {
        let outbox_url = format!("{}/outbox", &self.actor_id);

        app.client
            .post(&outbox_url)
            .bearer_auth(&self.token)
            .header("Content-Type", "application/activity+json")
            .json(&activity)
            .send()
            .await
            .expect("Failed to post to outbox")
    }

    /// Get this user's inbox
    /// TODO i need to implement a get_inbox for individual devices...
    /// FIXME actually i just realized the token needs to be by device, not per user. oml im going crazy.
    pub async fn get_inbox(&self, app: &TestApp) -> Value {
        let inbox_url = format!("{}/inbox", &self.actor_id);

        let resp = app
            .client
            .get(&inbox_url)
            .bearer_auth(&self.token)
            .header("Accept", "application/activity+json")
            .send()
            .await
            .expect("Failed to get inbox");

        resp.json().await.expect("Failed to parse inbox")
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
            object: envelope,
        })
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
    /// `from_did`: The device ID of the sender
    /// `to_did`: The device ID of the recipient  
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
        app: &TestApp,
        content: &str,
    ) -> Self {
        for device in &recipient.devices {
            let to_did_url = device.to_url(&app.domain);
            self.messages.push(EncryptedMessageEntry {
                to: to_did_url,
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
            to: vec![recipient_id.to_string()],
        }
    }
}
