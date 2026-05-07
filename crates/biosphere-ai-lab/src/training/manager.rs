use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use chrono::{DateTime, Utc};

use crate::core::EventBus;
use crate::types::SessionId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointInfo {
    pub id: String,
    pub session_id: SessionId,
    pub epoch: usize,
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub metrics: serde_json::Value,
    pub size_bytes: u64,
}

pub struct TrainingManager {
    event_bus: std::sync::Arc<EventBus>,
}

impl TrainingManager {
    pub fn new(event_bus: std::sync::Arc<EventBus>) -> Self {
        Self { event_bus }
    }

    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }
}
