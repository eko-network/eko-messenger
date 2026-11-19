//! Tokens for a device's notification service, e.g. the iOS one, which reads the inbox and
//! acknowledges messages while the app is not running. Unlike the account's JWT a token
//! does not expire, so it is only good for the inbox and `Delivered` activities.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};

const PREFIX: &str = "ent_";

/// A new token and the hash to store for it.
pub fn mint() -> (String, Vec<u8>) {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let token = format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(bytes));
    let hash = Sha256::digest(&token).to_vec();
    (token, hash)
}

/// The stored hash of `token`, or `None` when it is no notification token, e.g. a JWT.
pub fn hash(token: &str) -> Option<Vec<u8>> {
    token
        .starts_with(PREFIX)
        .then(|| Sha256::digest(token).to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_minted_token_hashes_to_its_hash() {
        let (token, stored) = mint();
        assert_eq!(hash(&token), Some(stored));
        assert_ne!(mint().0, token);
    }

    #[test]
    fn a_jwt_is_no_notification_token() {
        assert_eq!(hash("eyJhbGciOiJFUzI1NiJ9.e30.sig"), None);
    }
}
