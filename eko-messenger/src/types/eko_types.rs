use crate::devices::DeviceId;
use crate::server::context;
use crate::types::{single_item_vec, single_item_vec_borrowed};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::base64::Base64;
use serde_with::serde_as;

/// Represents an encrypted message in the Eko protocol
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedMessage {
    #[serde(rename = "@context")]
    pub context: Value,
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(default)]
    pub id: Option<String>,
    pub content: Vec<EncryptedMessageEntry>,
    pub attributed_to: String,
    #[serde(with = "single_item_vec")]
    pub to: String,
}

/// A single encrypted message entry for a specific device
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EncryptedMessageEntry {
    pub to: String,
    pub from: String,
    #[serde_as(as = "Base64")]
    pub content: Vec<u8>,
}

/// Key bundle for establishing encrypted sessions
#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPackage {
    #[serde(rename = "@context")]
    pub context: Value,
    #[serde(rename = "type")]
    pub type_field: String,

    pub did: DeviceId,
    #[serde_as(as = "Base64")]
    pub key: Vec<u8>,
}

impl KeyPackage {
    pub fn new(did: DeviceId, key: Vec<u8>) -> Self {
        Self {
            context: context(),
            type_field: "KeyPackage".to_string(),
            did,
            key,
        }
    }
}
/// Represents a Device in the Eko protocol
#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    #[serde(rename = "@context")]
    pub context: Value,
    #[serde(rename = "type")]
    pub type_field: String,
    pub id: String,
    pub did: DeviceId,
    pub key_collection: String,
    #[serde_as(as = "Base64")]
    pub public_key: Vec<u8>,
}

impl Device {
    pub fn new(id: String, did: DeviceId, key_collection: String, public_key: Vec<u8>) -> Self {
        Device {
            context: context(),
            type_field: "Device".to_string(),
            id,
            did,
            key_collection,
            public_key,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedMessageView<'a> {
    #[serde(rename = "@context")]
    pub context: &'a serde_json::Value,
    #[serde(rename = "type")]
    pub type_field: &'a str,
    pub id: Option<&'a str>,
    pub content: &'a [EncryptedMessageEntry],
    pub attributed_to: &'a str,
    #[serde(with = "single_item_vec_borrowed")]
    pub to: &'a str,
}
