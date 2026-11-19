use axum::{
    Extension, Json, debug_handler,
    extract::{Path, State},
};
use serde::Serialize;

use crate::{AppError, MessengerContext, RequestAuth, notification_tokens};

#[derive(Serialize)]
pub struct NotificationToken {
    token: String,
}

/// Mints the calling device a token for its notification service, which replaces the one it
/// had.
#[debug_handler]
pub async fn post_notification_token(
    State(ctx): State<MessengerContext>,
    Path(uid): Path<String>,
    Extension(auth): Extension<RequestAuth>,
) -> Result<Json<NotificationToken>, AppError> {
    if auth.uid != uid || auth.notification_service {
        return Err(AppError::Forbidden(
            "Only a device signed in to the account mints its notification token".into(),
        ));
    }
    let (token, hash) = notification_tokens::mint();
    ctx.storage
        .replace_notification_token(&auth.uid, auth.did, &hash)
        .await?;
    Ok(Json(NotificationToken { token }))
}
