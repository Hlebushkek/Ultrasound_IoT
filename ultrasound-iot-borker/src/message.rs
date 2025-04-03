use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct DeviceMessage {
    pub value: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ScanMessage {
    pub device: Uuid,
    pub session: String,
    pub scan: Vec<f32>,
}
