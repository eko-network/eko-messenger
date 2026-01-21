use crate::errors::AppError;
use reqwest::Request;

/// Signs an HTTP request with HTTP Signatures
/// This is used for authenticating server-to-server ActivityPub requests
/// TODO https://swicg.github.io/activitypub-http-signature/
pub fn sign_request(
    _request: &mut Request,
    _private_key: &[u8],
    _key_id: &str,
) -> Result<(), AppError> {
    // TODO Implement HTTP Signatures
    tracing::warn!("HTTP signature signing not yet implemented");
    Ok(())
}