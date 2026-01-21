use crate::errors::AppError;
use axum::http::HeaderMap;

/// Verifies HTTP Signatures on incoming ActivityPub requests
/// to ensure the request actually came from the claimed actor
pub fn verify_http_signature(
    headers: &HeaderMap,
    _method: &str,
    _path: &str,
    _body: &[u8],
) -> Result<String, AppError> {
    // TODO we will need to implement HTTP Signature verification
    // https://swicg.github.io/activitypub-http-signature/
    
    tracing::warn!("HTTP signature verification not yet implemented");
    
    // Check if Signature header exists
    if let Some(signature_header) = headers.get("signature") {
        tracing::debug!("Signature header present: {:?}", signature_header);
        // TODO Parse and verify
    }
    
    // For now we'll just return a placeholder
    Ok("unknown".to_string())
}

/// Extracts the actor ID from the signature's keyId parameter
pub fn extract_actor_from_signature(signature_header: &str) -> Result<String, AppError> {
    // TODO: Parse keyId from signature header
    // Format: keyId="https://example.com/users/alice#main-key"
    // Should extract: https://example.com/users/alice
    
    tracing::debug!("Extracting actor from signature: {}", signature_header);
    Err(AppError::BadRequest("Signature parsing not implemented".to_string()))
}