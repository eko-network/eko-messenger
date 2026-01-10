pub mod models;
pub mod postgres;
pub mod traits;

use crate::storage::traits::*;
use std::sync::Arc;

#[derive(Clone)]
pub struct Storage {
    pub notifications: Arc<dyn NotificationStore>,
    pub inbox: Arc<dyn InboxStore>,
    pub outbox: Arc<dyn OutboxStore>,
    pub devices: Arc<dyn DeviceStore>,
    pub actors: Arc<dyn ActorStore>,
}
