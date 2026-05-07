use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::types::SessionId;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum LabEvent {
    SessionCreated { session_id: SessionId },
    SessionStarted { session_id: SessionId },
    SessionPaused { session_id: SessionId },
    SessionResumed { session_id: SessionId },
    SessionCompleted {
        session_id: SessionId,
        final_metrics: serde_json::Value,
    },
    SessionFailed {
        session_id: SessionId,
        error: String,
    },
    SessionCancelled { session_id: SessionId },

    EpochCompleted {
        session_id: SessionId,
        epoch: usize,
        total_epochs: usize,
        train_loss: f64,
        val_loss: Option<f64>,
        metrics: serde_json::Value,
    },

    BatchCompleted {
        session_id: SessionId,
        batch: usize,
        total_batches: usize,
        loss: f64,
    },

    CheckpointSaved {
        session_id: SessionId,
        path: String,
        epoch: usize,
    },

    DataLoaded {
        session_id: SessionId,
        rows: usize,
        columns: usize,
    },

    HardwareAlert {
        session_id: Option<SessionId>,
        cpu_usage: f32,
        memory_usage: f32,
        memory_total_mb: u64,
        memory_available_mb: u64,
        disk_total_gb: u64,
        disk_available_gb: u64,
        disk_usage_percent: f32,
        gpu_usage: Option<f32>,
        gpu_memory_used_mb: Option<u64>,
        gpu_memory_total_mb: Option<u64>,
        message: String,
    },

    ProgressUpdate {
        session_id: SessionId,
        progress: f64,
        message: String,
    },

    LogOutput {
        session_id: SessionId,
        level: String,
        message: String,
    },

    Heartbeat {
        session_id: SessionId,
        epoch: usize,
        total_epochs: usize,
        elapsed_secs: f64,
    },

    Custom(String, serde_json::Value),

    DatasetRegistered {
        dataset_id: String,
        name: String,
        format: String,
        rows: usize,
        columns: usize,
    },
    DatasetDeleted {
        dataset_id: String,
    },
    DatasetArchived {
        dataset_id: String,
    },
    DatasetRestored {
        dataset_id: String,
    },

    DownloadProgress {
        task_id: String,
        progress: f64,
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
        speed_mbps: f64,
        message: String,
    },
    DedupProgress {
        task_id: String,
        progress: f64,
        processed: usize,
        total: usize,
        duplicates_found: usize,
        message: String,
    },
    CurationProgress {
        task_id: String,
        step: String,
        progress: f64,
        message: String,
    },

    OperationCompleted {
        task_id: String,
        operation: String,
        result: serde_json::Value,
    },
    OperationFailed {
        task_id: String,
        operation: String,
        error: String,
    },
}

impl LabEvent {
    pub fn session_id(&self) -> Option<&SessionId> {
        match self {
            LabEvent::SessionCreated { session_id } => Some(session_id),
            LabEvent::SessionStarted { session_id } => Some(session_id),
            LabEvent::SessionPaused { session_id } => Some(session_id),
            LabEvent::SessionResumed { session_id } => Some(session_id),
            LabEvent::SessionCompleted { session_id, .. } => Some(session_id),
            LabEvent::SessionFailed { session_id, .. } => Some(session_id),
            LabEvent::SessionCancelled { session_id } => Some(session_id),
            LabEvent::EpochCompleted { session_id, .. } => Some(session_id),
            LabEvent::BatchCompleted { session_id, .. } => Some(session_id),
            LabEvent::CheckpointSaved { session_id, .. } => Some(session_id),
            LabEvent::DataLoaded { session_id, .. } => Some(session_id),
            LabEvent::HardwareAlert { .. } => None,
            LabEvent::ProgressUpdate { session_id, .. } => Some(session_id),
            LabEvent::LogOutput { session_id, .. } => Some(session_id),
            LabEvent::Heartbeat { session_id, .. } => Some(session_id),
            LabEvent::Custom(..) => None,
            LabEvent::DatasetRegistered { .. } => None,
            LabEvent::DatasetDeleted { .. } => None,
            LabEvent::DatasetArchived { .. } => None,
            LabEvent::DatasetRestored { .. } => None,
            LabEvent::DownloadProgress { .. } => None,
            LabEvent::DedupProgress { .. } => None,
            LabEvent::CurationProgress { .. } => None,
            LabEvent::OperationCompleted { .. } => None,
            LabEvent::OperationFailed { .. } => None,
        }
    }
}

pub struct EventBus {
    tx: broadcast::Sender<LabEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn emit(&self, event: LabEvent) {
        if let Err(e) = self.tx.send(event) {
            crate::infrastructure::log("EVENT", &format!("Failed to emit event: {}", e), None);
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LabEvent> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl std::fmt::Debug for EventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBus").finish()
    }
}
