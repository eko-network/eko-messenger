use chrono::{DateTime, Utc};
use eko_messenger::{AppError, DeviceId, RequestAuth};
use jsonwebtoken::{
    Algorithm, DecodingKey, Validation, dangerous::insecure_decode, decode, decode_header,
};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;
#[derive(Debug, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
struct Jwk {
    kid: Option<String>,
    kty: String,
    crv: Option<String>,
    x: Option<String>,
    y: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Claims {
    app_metadata: AppMetaData,
    sub: Uuid,
}

#[derive(Debug, Deserialize)]
struct AppMetaData {
    #[serde(default)]
    did: Option<DeviceId>,
    #[serde(default)]
    dat: Option<DateTime<Utc>>,
}

#[derive(Clone)]
struct KeyService {
    key_by_kid: Arc<RwLock<HashMap<String, Arc<DecodingKey>>>>,
    client: reqwest::Client,
    jwks_url: Arc<String>,
}

impl KeyService {
    async fn populate(&self) -> anyhow::Result<()> {
        let jwks: Jwks = self
            .client
            .get(self.jwks_url.as_str())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let mut map: HashMap<String, Arc<DecodingKey>> = HashMap::new();
        for k in jwks.keys {
            let Some(kid) = k.kid else { continue };
            if k.kty != "EC" || k.crv.as_deref() != Some("P-256") {
                continue;
            }
            let (Some(x), Some(y)) = (k.x.as_deref(), k.y.as_deref()) else {
                continue;
            };
            let key = DecodingKey::from_ec_components(x, y)?;
            map.insert(kid, Arc::new(key));
        }

        anyhow::ensure!(
            !map.is_empty(),
            "JWT_JWKS_URL returned no usable ES256 (P-256) keys"
        );
        let mut write_guard = self.key_by_kid.write().await;
        *write_guard = map;
        Ok(())
    }
    pub async fn new(jwks_url: &str) -> anyhow::Result<Self> {
        let map: HashMap<String, Arc<DecodingKey>> = HashMap::new();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;
        let ks = Self {
            jwks_url: Arc::new(jwks_url.to_string()),
            key_by_kid: Arc::new(RwLock::new(map)),
            client,
        };
        ks.populate().await?;
        Ok(ks)
    }
    pub async fn lookup(&self, kid: &str) -> anyhow::Result<Arc<DecodingKey>> {
        {
            let read_guard = self.key_by_kid.read().await;
            if let Some(key) = read_guard.get(kid) {
                return Ok(key.clone());
            }
        }

        self.populate().await?;
        let read_guard = self.key_by_kid.read().await;
        read_guard
            .get(kid)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Key ID '{}' not found after JWKS refresh", kid))
    }
}

pub fn debug_token(token: &str) -> Option<String> {
    let data = insecure_decode::<Value>(token).ok()?;
    Some(format!("header={:?} claims={}", data.header, data.claims))
}

#[derive(Clone)]
pub struct JWTVerifier {
    validation: Validation,
    key_service: KeyService,
}

impl JWTVerifier {
    pub async fn new(jwks_url: &str) -> anyhow::Result<Self> {
        let mut validation = Validation::new(Algorithm::ES256);
        validation.set_audience(&["authenticated"]);

        Ok(Self {
            key_service: KeyService::new(jwks_url).await?,
            validation,
        })
    }

    pub async fn verify(&self, token: &str) -> Result<RequestAuth, AppError> {
        let header = decode_header(token)?;
        let kid = header.kid.ok_or_else(|| {
            jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidToken)
        })?;
        if let Ok(key) = self.key_service.lookup(&kid).await {
            let claims = decode::<Claims>(token, &key, &self.validation).map(|v| v.claims)?;
            if let Some(did) = claims.app_metadata.did {
                return Ok(RequestAuth {
                    uid: claims.sub.to_string(),
                    did,
                    device_approved: claims.app_metadata.dat.is_some(),
                });
            }
            return Err(AppError::Unauthorized("Missing or invalid did".to_string()));
        } else {
            Err(AppError::Unauthorized("Bad token".to_string()))
        }
    }
}
