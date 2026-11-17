use std::sync::OnceLock;

use aes_gcm::{Aes128Gcm, KeyInit, aead::Aead};
use base64::Engine;
use chrono::Utc;
use hkdf::Hkdf;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use p256::{EncodedPoint, PublicKey, SecretKey, ecdh::diffie_hellman, pkcs8::EncodePrivateKey};
use rand_core::{OsRng, RngCore};
use reqwest::Client;
use serde_json::{Map, Value};
use sha2::Sha256;
use tracing::{error, info, warn};

use crate::{
    devices::DeviceId,
    storage::{
        NotificationStore,
        models::{DeviceNotificationInfo, NotificationType},
    },
};

struct ApnsConfig {
    team_id: String,
    key_id: String,
    private_key_p8: String,
    bundle_id: String,
    sandbox: bool,
}

fn apns_config() -> ApnsConfig {
    ApnsConfig {
        team_id: std::env::var("APNS_TEAM_ID").unwrap_or_default(),
        key_id: std::env::var("APNS_KEY_ID").unwrap_or_default(),
        private_key_p8: std::env::var("APNS_PRIVATE_KEY")
            .unwrap_or_default()
            .replace("\\n", "\n"),
        bundle_id: std::env::var("APNS_BUNDLE_ID").unwrap_or_default(),
        sandbox: std::env::var("APNS_USE_SANDBOX").as_deref() == Ok("true"),
    }
}

fn apns_token_cache() -> &'static tokio::sync::Mutex<Option<(String, std::time::Instant)>> {
    static CACHE: OnceLock<tokio::sync::Mutex<Option<(String, std::time::Instant)>>> =
        OnceLock::new();
    CACHE.get_or_init(|| tokio::sync::Mutex::new(None))
}

async fn get_apns_token(config: &ApnsConfig) -> Option<String> {
    let cache = apns_token_cache();
    {
        let guard = cache.lock().await;
        if let Some((token, expires_at)) = &*guard
            && std::time::Instant::now() < *expires_at
        {
            return Some(token.clone());
        }
    }

    if config.private_key_p8.is_empty() {
        error!("APNS Error: Missing private key for token generation.");
        return None;
    }

    let header = Header {
        alg: Algorithm::ES256,
        kid: Some(config.key_id.clone()),
        ..Default::default()
    };

    let claims = serde_json::json!({
        "iss": config.team_id,
        "iat": Utc::now().timestamp(),
    });

    let key = EncodingKey::from_ec_pem(config.private_key_p8.as_bytes()).ok()?;

    let jwt = jsonwebtoken::encode(&header, &claims, &key).ok()?;

    let expires_at = std::time::Instant::now() + std::time::Duration::from_secs(28 * 60);
    let mut guard = cache.lock().await;
    *guard = Some((jwt.clone(), expires_at));

    info!("Generated new APNS token");
    Some(jwt)
}

async fn deactivate_device(store: &dyn NotificationStore, did: DeviceId) {
    if let Err(e) = store.mark_inactive(did).await {
        error!("Failed to deactivate device {did}: {e}");
    } else {
        info!("Deactivated expired device {did}");
    }
}

fn vapid_signing_key() -> Option<EncodingKey> {
    let priv_key_b64 = std::env::var("VAPID_PRIVATE_KEY").ok()?;
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(priv_key_b64.as_bytes())
        .ok()?;

    let pkcs8 = p256::SecretKey::from_be_bytes(&raw)
        .ok()?
        .to_pkcs8_der()
        .ok()?;

    Some(EncodingKey::from_ec_der(pkcs8.as_bytes()))
}

fn vapid_public_key_b64() -> String {
    std::env::var("VAPID_PUBLIC_KEY").unwrap_or_default()
}

/// Push services accept at most 4096 bytes of encrypted body (RFC 8030 / 8291).
const WEB_PUSH_MAX_BODY: usize = 4096;
const WEB_PUSH_RECORD_SIZE: u32 = 4096;

fn decode_b64_key(value: &str) -> Option<Vec<u8>> {
    let trimmed = value.trim_end_matches('=');
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(trimmed)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(trimmed))
        .ok()
}

fn hkdf_expand<const N: usize>(prk: &Hkdf<Sha256>, info: &[u8]) -> Option<[u8; N]> {
    let mut out = [0u8; N];
    prk.expand(info, &mut out).ok()?;
    Some(out)
}

/// Encrypts a single-record `aes128gcm` Web Push body as described in RFC 8291.
/// The ephemeral key and salt are passed in so the output is reproducible in tests.
fn encrypt_aes128gcm(
    ua_public: &[u8],
    auth_secret: &[u8],
    as_secret: &SecretKey,
    salt: &[u8; 16],
    plaintext: &[u8],
) -> Option<Vec<u8>> {
    let ua_key = PublicKey::from_sec1_bytes(ua_public).ok()?;
    let ua_public = EncodedPoint::from(ua_key);
    let as_public = EncodedPoint::from(as_secret.public_key());

    let shared = diffie_hellman(as_secret.to_nonzero_scalar(), ua_key.as_affine());

    let mut key_info = b"WebPush: info\0".to_vec();
    key_info.extend_from_slice(ua_public.as_bytes());
    key_info.extend_from_slice(as_public.as_bytes());
    let ikm: [u8; 32] = hkdf_expand(
        &Hkdf::<Sha256>::new(Some(auth_secret), shared.raw_secret_bytes()),
        &key_info,
    )?;

    let prk = Hkdf::<Sha256>::new(Some(salt), &ikm);
    let cek: [u8; 16] = hkdf_expand(&prk, b"Content-Encoding: aes128gcm\0")?;
    let nonce: [u8; 12] = hkdf_expand(&prk, b"Content-Encoding: nonce\0")?;

    // 0x02 marks the last (and only) record, no padding
    let mut record = plaintext.to_vec();
    record.push(0x02);
    let ciphertext = Aes128Gcm::new(&cek.into())
        .encrypt(&nonce.into(), record.as_slice())
        .ok()?;

    let mut out = Vec::with_capacity(16 + 4 + 1 + 65 + ciphertext.len());
    out.extend_from_slice(salt);
    out.extend_from_slice(&WEB_PUSH_RECORD_SIZE.to_be_bytes());
    out.push(as_public.as_bytes().len() as u8);
    out.extend_from_slice(as_public.as_bytes());
    out.extend_from_slice(&ciphertext);
    Some(out)
}

fn encrypt_web_push(p256dh: &str, auth: &str, plaintext: &[u8]) -> Option<Vec<u8>> {
    let ua_public = decode_b64_key(p256dh)?;
    let auth_secret = decode_b64_key(auth)?;
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    encrypt_aes128gcm(
        &ua_public,
        &auth_secret,
        &SecretKey::random(&mut OsRng),
        &salt,
        plaintext,
    )
}

/// The MLS ciphertext is left out because welcome and commit messages easily exceed
/// the 4KB Web Push limit. Clients fetch their inbox when they get this push.
async fn send_web_push(
    client: &Client,
    store: &dyn NotificationStore,
    device: &DeviceNotificationInfo,
    title: &str,
    body: &str,
    activity_id: &str,
    object_id: &str,
) {
    let sub: Map<String, Value> = match serde_json::from_str(&device.token) {
        Ok(v) => v,
        Err(_) => {
            error!(
                "WebPush Error: invalid subscription JSON for device {}",
                device.did
            );
            return;
        }
    };

    let endpoint = match sub.get("endpoint").and_then(|v| v.as_str()) {
        Some(e) if !e.is_empty() => e.to_string(),
        _ => {
            error!("WebPush Error: missing endpoint for device {}", device.did);
            return;
        }
    };

    let keys = sub.get("keys").and_then(|v| v.as_object());
    let (p256dh, auth) = match keys.map(|k| (k.get("p256dh"), k.get("auth"))) {
        Some((Some(Value::String(p256dh)), Some(Value::String(auth)))) => (p256dh, auth),
        _ => {
            error!("WebPush Error: missing keys for device {}", device.did);
            return;
        }
    };

    // same shape as the posts notify-user function, data.type routes it on the client
    let plaintext = serde_json::to_vec(&serde_json::json!({
        "title": title,
        "body": body,
        "data": {
            "type": "message",
            "activity_id": activity_id,
            "object_id": object_id,
        },
    }))
    .unwrap_or_default();

    let payload = match encrypt_web_push(p256dh, auth, &plaintext) {
        Some(p) => p,
        None => {
            error!(
                "WebPush Error: failed to encrypt payload for device {}",
                device.did
            );
            return;
        }
    };
    if payload.len() > WEB_PUSH_MAX_BODY {
        error!(
            "WebPush Error: payload of {} bytes too large for device {}",
            payload.len(),
            device.did
        );
        return;
    }

    let signing_key = match vapid_signing_key() {
        Some(k) => k,
        None => {
            error!("WebPush Error: missing or invalid VAPID private key");
            return;
        }
    };
    let pub_key_b64 = vapid_public_key_b64();
    if pub_key_b64.is_empty() {
        error!("WebPush Error: missing VAPID public key");
        return;
    }

    let origin = endpoint.split('/').nth(2).unwrap_or("").to_string();

    let vapid_claims = serde_json::json!({
        "aud": format!("https://{origin}"),
        "exp": (Utc::now().timestamp() + 43200) as u64,
        "sub": "mailto:support@eko-app.com",
    });

    let header = Header::new(Algorithm::ES256);
    let jwt = jsonwebtoken::encode(&header, &vapid_claims, &signing_key).unwrap_or_default();

    let res = match client
        .post(&endpoint)
        .header("Content-Encoding", "aes128gcm")
        .header("Content-Type", "application/octet-stream")
        .header("TTL", "86400")
        // lets the distributor wake the device out of doze for a chat message
        .header("Urgency", "high")
        .header("Authorization", format!("vapid t={jwt}, k={pub_key_b64}"))
        .body(payload)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("WebPush Error sending to device {}: {e}", device.did);
            return;
        }
    };

    if res.status().as_u16() == 410 {
        warn!(
            "WebPush: subscription expired for device {}, deactivating",
            device.did
        );
        deactivate_device(store, device.did).await;
    } else if !res.status().is_success() {
        error!(
            "WebPush delivery failed for device {} with status {}",
            device.did,
            res.status()
        );
    } else {
        info!("WebPush sent to device {}", device.did);
    }
}

#[allow(clippy::too_many_arguments)]
async fn send_apns(
    client: &Client,
    store: &dyn NotificationStore,
    device: &DeviceNotificationInfo,
    config: &ApnsConfig,
    title: &str,
    body: &str,
    payload_data: &Value,
    activity_id: String,
    object_id: String,
) {
    if config.private_key_p8.is_empty() {
        error!("APNS Error: Missing private key.");
        return;
    }

    let jwt = match get_apns_token(config).await {
        Some(t) => t,
        None => {
            error!("APNS Error: No valid APNS token available.");
            return;
        }
    };

    let apns_payload = serde_json::json!({
        "aps": {
            "alert": {
                "title": title,
                "body": body,
            },
            "mutable-content": 1,
        },
        "mls_message": payload_data,
        "activity_id": activity_id,
        "object_id": object_id,
    });

    let apns_host = if config.sandbox {
        "api.sandbox.push.apple.com"
    } else {
        "api.development.push.apple.com" // FIXME need to have api.push.apple.com for PROD!!
    };

    let url = format!("https://{apns_host}/3/device/{}", device.token);
    let mut retry = true;

    loop {
        let res = match client
            .post(&url)
            .header("authorization", format!("bearer {jwt}"))
            .header("apns-topic", &config.bundle_id)
            .header("apns-push-type", "alert")
            .json(&apns_payload)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                error!(
                    "APNS Error sending to device {}: {} {e}",
                    device.did, device.token
                );
                return;
            }
        };

        let status = res.status();

        if status.as_u16() == 403 && retry {
            warn!("APNS token invalid or expired, clearing cache and retrying");
            let cache = apns_token_cache();
            let mut guard = cache.lock().await;
            *guard = None;
            drop(guard);
            retry = false;
            info!("Retrying APNS send for device {}", device.did);
            continue;
        }

        if status.as_u16() == 410 {
            warn!(
                "APNS: token expired for device {}, deactivating",
                device.did
            );
            deactivate_device(store, device.did).await;
        } else if !status.is_success() {
            let body_text = res.text().await.unwrap_or_default();
            error!(
                "APNS delivery failed for device {} {} with status {status}: {body_text}",
                device.token, device.did
            );
        } else {
            info!("APNS sent to device {}", device.did);
        }

        break;
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn send_push_notifications(
    client: &Client,
    store: &dyn NotificationStore,
    target_devices: &[DeviceId],
    title: &str,
    body: &str,
    payload_data: &Value,
    activity_id: Option<String>,
    object_id: Option<String>,
) {
    let endpoints = match store.retrive_endpoints(target_devices.to_vec()).await {
        Ok(Some(e)) => e,
        Ok(None) => {
            info!("No push notification endpoints found");
            return;
        }
        Err(e) => {
            error!("Failed to retrieve notification endpoints: {e}");
            return;
        }
    };

    let config = apns_config();
    let mut futs: Vec<std::pin::Pin<Box<dyn futures::Future<Output = ()> + Send>>> = Vec::new();

    for device in &endpoints {
        match device.notification_type {
            NotificationType::Apns => {
                futs.push(Box::pin(send_apns(
                    client,
                    store,
                    device,
                    &config,
                    title,
                    body,
                    payload_data,
                    activity_id.clone().unwrap_or("".to_string()),
                    object_id.clone().unwrap_or("".to_string()),
                )));
            }
            NotificationType::WebPush => {
                futs.push(Box::pin(send_web_push(
                    client,
                    store,
                    device,
                    title,
                    body,
                    activity_id.as_deref().unwrap_or(""),
                    object_id.as_deref().unwrap_or(""),
                )));
            }
        }
    }

    futures::future::join_all(futs).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 8291 appendix A
    #[test]
    fn encrypts_the_rfc_8291_example() {
        let b64 = |s| decode_b64_key(s).unwrap();
        let as_secret =
            SecretKey::from_be_bytes(&b64("yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw")).unwrap();
        let salt: [u8; 16] = b64("DGv6ra1nlYgDCS1FRnbzlw").try_into().unwrap();

        let body = encrypt_aes128gcm(
            &b64("BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4"),
            &b64("BTBZMqHH6r4Tts7J_aSIgg"),
            &as_secret,
            &salt,
            b"When I grow up, I want to be a watermelon",
        )
        .unwrap();

        assert_eq!(
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(body),
            "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPTpK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN"
        );
    }

    #[test]
    fn accepts_padded_and_standard_base64_keys() {
        assert_eq!(
            decode_b64_key("BTBZMqHH6r4Tts7J_aSIgg=="),
            decode_b64_key("BTBZMqHH6r4Tts7J/aSIgg")
        );
    }
}
