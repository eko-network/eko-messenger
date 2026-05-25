use std::env::var_os;

use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{auth::handlers::JWT_LIFESPAN, devices::DeviceId};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub did: DeviceId,
    pub exp: usize,
    pub iat: usize,
    pub roles: Vec<String>,
}

pub struct JwtHelper {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtHelper {
    pub fn new_from_env() -> anyhow::Result<Self> {
        let secret = var_os("JWT_SECRET")
            .expect("JWT_SECRET not found in enviroment")
            .into_string()
            .map_err(|_| anyhow::anyhow!("Failed to convert from OsString to String"))?;
        let decoding_key = DecodingKey::from_secret(secret.as_ref());
        let encoding_key = EncodingKey::from_secret(secret.as_ref());

        Ok(JwtHelper {
            encoding_key,
            decoding_key,
        })
    }
    pub fn create_jwt(
        &self,
        uid: &str,
        did: DeviceId,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let now = OffsetDateTime::now_utc();
        let iat = now.unix_timestamp() as usize;
        // Set expiration to 15 minutes from now
        let exp = (now + JWT_LIFESPAN).unix_timestamp() as usize;

        let claims = Claims {
            sub: uid.to_string(),
            did,
            exp,
            iat,
            roles: vec!["user".to_string()],
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)?;

        Ok(token)
    }
    pub fn decrypt_jwt(
        &self,
        token: &str,
    ) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
        let validation = Validation::new(Algorithm::HS256);
        decode::<Claims>(token, &self.decoding_key, &validation)
    }
}
