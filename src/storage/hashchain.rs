// use crate::errors::AppError;
// use sha2::{Digest, Sha256};
// use sqlx::PgPool;
//
// /// Genesis value for the first action in a user's hashchain
// pub const GENESIS_ACTION_ID: &str =
//     "0000000000000000000000000000000000000000000000000000000000000000";
//
// /// Compute the action_id hash for a device action
// /// This creates the hashchain by incorporating the previous action_id
// pub fn compute_action_id(did: &str, prev: &str, timestamp: i64, nonce: &[u8; 32]) -> String {
//     let mut hasher = Sha256::new();
//     hasher.update(did.as_bytes());
//     hasher.update(prev.as_bytes());
//     hasher.update(timestamp.to_le_bytes());
//     hasher.update(nonce);
//
//     let result = hasher.finalize();
//     hex::encode(result)
// }
//
// /// Get the latest action_id for a user (for computing prev in new actions)
// /// Returns None if this is the first action for the user
// pub async fn get_latest_action_id_for_user(
//     pool: &PgPool,
//     uid: &str,
// ) -> Result<Option<String>, AppError> {
//     let result = sqlx::query!(
//         "SELECT action_id FROM device_actions
//          WHERE uid = $1
//          ORDER BY created_at DESC
//          LIMIT 1",
//         uid
//     )
//     .fetch_optional(pool)
//     .await?;
//
//     Ok(result.map(|r| r.action_id))
// }
//
// /// Get the count of approved devices for a user
// /// This is used to determine if a new device should be auto-approved (first device)
// pub async fn get_approved_device_count(pool: &PgPool, uid: &str) -> Result<i64, AppError> {
//     let result = sqlx::query!(
//         "SELECT COUNT(*) as count FROM device_actions
//          WHERE uid = $1 AND is_add = TRUE AND approved_at IS NOT NULL",
//         uid
//     )
//     .fetch_one(pool)
//     .await?;
//
//     Ok(result.count.unwrap_or(0))
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn test_compute_action_id() {
//         let did = "test-device-id";
//         let prev = GENESIS_ACTION_ID;
//         let timestamp = 1234567890;
//         let nonce = [0u8; 32];
//
//         let action_id = compute_action_id(did, prev, timestamp, &nonce);
//
//         // Should produce a 64-character hex string (SHA256)
//         assert_eq!(action_id.len(), 64);
//         assert!(action_id.chars().all(|c| c.is_ascii_hexdigit()));
//     }
//
//     #[test]
//     fn test_different_inputs_produce_different_hashes() {
//         let nonce = [0u8; 32];
//         let timestamp = 1234567890;
//
//         let hash1 = compute_action_id("device1", GENESIS_ACTION_ID, timestamp, &nonce);
//         let hash2 = compute_action_id("device2", GENESIS_ACTION_ID, timestamp, &nonce);
//
//         assert_ne!(hash1, hash2);
//     }
//
//     #[test]
//     fn test_hashchain_linking() {
//         let nonce = [0u8; 32];
//         let timestamp = 1234567890;
//
//         // First action links to genesis
//         let action1 = compute_action_id("device1", GENESIS_ACTION_ID, timestamp, &nonce);
//
//         // Second action links to first action
//         let action2 = compute_action_id("device2", &action1, timestamp + 1, &nonce);
//
//         // Third action links to second action
//         let action3 = compute_action_id("device3", &action2, timestamp + 2, &nonce);
//
//         // All should be different
//         assert_ne!(action1, action2);
//         assert_ne!(action2, action3);
//         assert_ne!(action1, action3);
//     }
// }
