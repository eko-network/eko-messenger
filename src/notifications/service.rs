use std::{env::var, sync::Arc};

use futures::future::join_all;
use web_push::{
    ContentEncoding, HyperWebPushClient, PartialVapidSignatureBuilder, SubscriptionInfo,
    VapidSignatureBuilder, WebPushClient, WebPushMessageBuilder,
};

use crate::{errors::AppError, notifications::vapid::maybe_create_vapid_key, storage::Storage};

pub struct NotificationService {
    storage: Arc<Storage>,
    client: HyperWebPushClient,
    vapid: PartialVapidSignatureBuilder,
    pub public_key: String,
}

impl NotificationService {
    pub async fn new(storage: Arc<Storage>) -> anyhow::Result<Self> {
        let pem_path = var("VAPID_KEY_PATH").expect("VAPID_KEY_PATH should be set in enviroment");
        let public_key = maybe_create_vapid_key(&pem_path).await?;
        let file = std::fs::File::open(pem_path)?;
        Ok(NotificationService {
            storage,
            client: HyperWebPushClient::new(),
            vapid: VapidSignatureBuilder::from_pem_no_sub(file)?,
            public_key,
        })
    }
    pub async fn register(&self, did: i32, endpoint: &SubscriptionInfo) -> Result<(), AppError> {
        self.storage
            .notifications
            .upsert_endpoint(did, endpoint)
            .await?;
        Ok(())
    }
    pub async fn notify(&self, dids: &[i32]) -> Result<(), AppError> {
        let endpoints = self.storage.notifications.retrive_endpoints(dids).await?;
        join_all(endpoints.into_iter().map(|sub| {
            let client = self.client.clone();
            let vapid = self.vapid.clone();
            async move {
                let Ok(sig) = vapid.add_sub_info(&sub).build() else {
                    tracing::error!("Failed to build vapid signature");
                    return;
                };
                let mut message = WebPushMessageBuilder::new(&sub);
                message.set_vapid_signature(sig);
                message.set_payload(ContentEncoding::Aes128Gcm, "wake".as_bytes());

                let Ok(payload) = message.build() else {
                    tracing::error!("Failed to build notifiaction");
                    return;
                };
                if let Err(e) = client.send(payload).await {
                    tracing::error!("POST failed: {e}");
                    return;
                }
                tracing::info!("Sent Notification")
            }
        }))
        .await;
        Ok(())
    }
}
