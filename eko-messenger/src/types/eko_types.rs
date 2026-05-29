use crate::devices::DeviceId;
use crate::server::context;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::base64::Base64;
use serde_with::serde_as;

/// Key bundle for establishing encrypted sessions
#[serde_as]
#[derive(Debug, Deserialize, Serialize, Clone)]
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
