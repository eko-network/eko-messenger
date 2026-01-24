use crate::errors::AppError;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeviceId(Uuid);

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl DeviceId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Convert to public-facing URL
    pub fn to_url(&self, domain: &str) -> String {
        format!("{}/devices/{}", domain, self.0)
    }

    /// Parse from public URL
    pub fn from_url(url: &str) -> Result<Self, AppError> {
        let uuid_str = url
            .rsplit_once('/')
            .map(|(_, id)| id)
            .ok_or(anyhow!("Invalid device URL format"))?;
        let uuid = Uuid::parse_str(uuid_str)?;
        Ok(Self(uuid))
    }

    /// Get key collection URL
    pub fn key_collection_url(&self, domain: &str) -> String {
        format!("{}/keyCollection", self.to_url(domain))
    }

    /// Get device action URL
    pub fn action_url(&self, domain: &str, is_add: bool) -> String {
        let prefix = if is_add { 'a' } else { 'r' };
        format!("{}/deviceActions/{}{}", domain, prefix, self.0)
    }
}
