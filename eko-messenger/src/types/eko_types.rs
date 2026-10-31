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
