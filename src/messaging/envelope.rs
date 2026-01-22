use crate::errors::AppError;

/// Validates an encrypted message envelope
/// Ensures the envelope has the correct number of messages for recipient devices
pub fn validate_envelope_for_recipient(
    envelope_message_count: usize,
    recipient_device_count: usize,
) -> Result<(), AppError> {
    // One message per device
    if envelope_message_count != recipient_device_count {
        tracing::debug!(
            "Device count mismatch: expected {} devices, got {} messages",
            recipient_device_count,
            envelope_message_count
        );
        return Err(AppError::BadRequest("device_list_mismatch".to_string()));
    }

    Ok(())
}

/// Validates that all device IDs in the envelope match the recipient's devices
pub fn validate_device_ids(
    envelope_device_ids: &[String],
    recipient_device_ids: &[String],
) -> Result<(), AppError> {
    // Convert to sets for comparison
    let envelope_set: std::collections::HashSet<_> = envelope_device_ids.iter().collect();
    let recipient_set: std::collections::HashSet<_> = recipient_device_ids.iter().collect();

    // Check that all device IDs in envelope are valid
    for device_id in envelope_device_ids {
        if !recipient_set.contains(&device_id) {
            tracing::debug!("Unknown device ID: {}", device_id);
            return Err(AppError::BadRequest("device_list_mismatch".to_string()));
        }
    }

    // Check that all recipient devices are included
    if envelope_set.len() != recipient_set.len() {
        tracing::debug!("Not all recipient devices included in message");
        return Err(AppError::BadRequest("device_list_mismatch".to_string()));
    }

    Ok(())
}
