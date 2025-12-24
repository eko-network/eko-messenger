use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

use crate::{
    auth::{PreKey, SignedPreKey},
    errors::AppError,
    storage::models::{RegisterDeviceResult, RotatedRefreshToken},
    storage::traits::DeviceStore,
    types::PreKeyBundle,
};

#[derive(Clone, Debug)]
struct Device {
    did: i32,
    uid: String,
    name: String,
    identity_key: Vec<u8>,
    registration_id: i32,
}

#[derive(Clone, Debug)]
struct RefreshToken {
    token: Uuid,
    did: i32,
    ip_address: String,
    user_agent: String,
    expires_at: time::OffsetDateTime,
}

#[derive(Clone, Debug)]
struct PreKeyRecord {
    did: i32,
    key_id: i32,
    key: Vec<u8>,
}

#[derive(Clone, Debug)]
struct SignedPreKeyRecord {
    did: i32,
    key_id: i32,
    key: Vec<u8>,
    signature: Vec<u8>,
}

#[derive(Default)]
pub struct InMemoryDeviceStore {
    devices: RwLock<HashMap<i32, Device>>,
    refresh_tokens: RwLock<HashMap<Uuid, RefreshToken>>,
    pre_keys: RwLock<Vec<PreKeyRecord>>,
    signed_pre_keys: RwLock<HashMap<i32, SignedPreKeyRecord>>,
    next_did: RwLock<i32>,
}

impl InMemoryDeviceStore {
    pub fn new() -> Self {
        Self {
            devices: RwLock::new(HashMap::new()),
            refresh_tokens: RwLock::new(HashMap::new()),
            pre_keys: RwLock::new(Vec::new()),
            signed_pre_keys: RwLock::new(HashMap::new()),
            next_did: RwLock::new(1),
        }
    }
}

#[async_trait]
impl DeviceStore for InMemoryDeviceStore {
    async fn key_bundles_for_user(
        &self,
        uid: &str,
    ) -> Result<Vec<PreKeyBundle>, AppError> {
        let devices = self.devices.read().unwrap();
        let user_devices: Vec<_> = devices
            .values()
            .filter(|d| d.uid == uid)
            .cloned()
            .collect();
        drop(devices);

        let mut bundles = Vec::new();

        for device in user_devices {
            // Find and remove a pre_key for this device
            let mut pre_keys = self.pre_keys.write().unwrap();
            let pre_key_pos = pre_keys.iter().position(|pk| pk.did == device.did);
            
            if let Some(pos) = pre_key_pos {
                let pre_key = pre_keys.remove(pos);
                drop(pre_keys);

                // Get the signed pre key
                let signed_pre_keys = self.signed_pre_keys.read().unwrap();
                if let Some(signed_pre_key) = signed_pre_keys.get(&device.did) {
                    bundles.push(PreKeyBundle {
                        did: device.did,
                        identity_key: device.identity_key.clone(),
                        registration_id: device.registration_id,
                        pre_key_id: pre_key.key_id,
                        pre_key: pre_key.key,
                        signed_pre_key_id: signed_pre_key.key_id,
                        signed_pre_key: signed_pre_key.key.clone(),
                        signed_pre_key_signature: signed_pre_key.signature.clone(),
                    });
                }
            }
        }

        Ok(bundles)
    }

    async fn register_device(
        &self,
        uid: &str,
        device_name: &str,
        identity_key: &[u8],
        registration_id: i32,
        pre_keys: &[PreKey],
        signed_pre_key: &SignedPreKey,
        ip_address: &str,
        user_agent: &str,
        expires_at: time::OffsetDateTime,
    ) -> Result<RegisterDeviceResult, AppError> {
        let mut next_did = self.next_did.write().unwrap();
        let did = *next_did;
        *next_did += 1;
        drop(next_did);

        let device = Device {
            did,
            uid: uid.to_string(),
            name: device_name.to_string(),
            identity_key: identity_key.to_vec(),
            registration_id,
        };

        let mut devices = self.devices.write().unwrap();
        devices.insert(did, device);
        drop(devices);

        let refresh_token = Uuid::new_v4();
        let token_record = RefreshToken {
            token: refresh_token,
            did,
            ip_address: ip_address.to_string(),
            user_agent: user_agent.to_string(),
            expires_at,
        };

        let mut refresh_tokens = self.refresh_tokens.write().unwrap();
        refresh_tokens.insert(refresh_token, token_record);
        drop(refresh_tokens);

        let mut pre_key_records = self.pre_keys.write().unwrap();
        for pre_key in pre_keys {
            pre_key_records.push(PreKeyRecord {
                did,
                key_id: pre_key.id,
                key: pre_key.key.clone(),
            });
        }
        drop(pre_key_records);

        let signed_pre_key_record = SignedPreKeyRecord {
            did,
            key_id: signed_pre_key.id,
            key: signed_pre_key.key.clone(),
            signature: signed_pre_key.signature.clone(),
        };

        let mut signed_pre_keys = self.signed_pre_keys.write().unwrap();
        signed_pre_keys.insert(did, signed_pre_key_record);
        drop(signed_pre_keys);

        Ok(RegisterDeviceResult {
            did,
            refresh_token,
        })
    }

    async fn rotate_refresh_token(
        &self,
        old_token: &Uuid,
        ip_address: &str,
        user_agent: &str,
    ) -> Result<Option<RotatedRefreshToken>, AppError> {
        let refresh_tokens = self.refresh_tokens.read().unwrap();
        let old_token_record = refresh_tokens.get(old_token).cloned();
        drop(refresh_tokens);

        let old_token_record = match old_token_record {
            Some(r) => r,
            None => return Ok(None),
        };

        if old_token_record.expires_at <= time::OffsetDateTime::now_utc()
            || old_token_record.user_agent != user_agent
        {
            return Ok(None);
        }

        let devices = self.devices.read().unwrap();
        let device = devices.get(&old_token_record.did).cloned();
        drop(devices);

        let device = match device {
            Some(d) => d,
            None => return Ok(None),
        };

        // Remove old token
        let mut refresh_tokens = self.refresh_tokens.write().unwrap();
        refresh_tokens.remove(old_token);

        // Create new token
        let new_token = Uuid::new_v4();
        let expires_at =
            time::OffsetDateTime::now_utc() + time::Duration::seconds(crate::auth::REFRESH_EXPIRATION);

        let new_token_record = RefreshToken {
            token: new_token,
            did: device.did,
            ip_address: ip_address.to_string(),
            user_agent: user_agent.to_string(),
            expires_at,
        };

        refresh_tokens.insert(new_token, new_token_record);
        drop(refresh_tokens);

        Ok(Some(RotatedRefreshToken {
            refresh_token: new_token,
            uid: device.uid,
            did: device.did,
            expires_at,
        }))
    }

    async fn logout_device(
        &self,
        refresh_token: &Uuid,
    ) -> Result<(), AppError> {
        let refresh_tokens = self.refresh_tokens.read().unwrap();
        let token_record = refresh_tokens.get(refresh_token).cloned();
        drop(refresh_tokens);

        if let Some(token_record) = token_record {
            let mut devices = self.devices.write().unwrap();
            devices.remove(&token_record.did);
            drop(devices);

            let mut refresh_tokens = self.refresh_tokens.write().unwrap();
            refresh_tokens.remove(refresh_token);
        }

        Ok(())
    }
}
