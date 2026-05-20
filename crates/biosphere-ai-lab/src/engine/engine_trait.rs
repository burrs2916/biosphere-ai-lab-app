use async_trait::async_trait;

use crate::core::{LabError, Plugin, Result};
use crate::core::config::TrainingConfig;
use crate::types::{ComputeBackend, SessionId, SessionStatus};

#[derive(Debug, Clone)]
pub struct SessionHandle {
    pub session_id: SessionId,
    pub experiment_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InferenceOutput {
    pub predictions: Vec<f32>,
    pub predicted_classes: Vec<usize>,
    pub probabilities: Vec<Vec<f32>>,
}

#[async_trait]
pub trait Engine: Plugin {
    fn supported_backends(&self) -> Vec<ComputeBackend>;

    async fn create_session(&self, config: &TrainingConfig) -> Result<SessionHandle>;

    async fn start(&self, handle: SessionHandle) -> Result<()>;

    async fn start_from_checkpoint(&self, handle: SessionHandle, checkpoint_epoch: usize, artifact_dir: &str) -> Result<()> {
        let _ = (handle, checkpoint_epoch, artifact_dir);
        Err(LabError::Custom("Checkpoint resume not supported by this engine".to_string()))
    }

    async fn pause(&self, handle: SessionHandle) -> Result<()>;

    async fn resume(&self, handle: SessionHandle) -> Result<()>;

    async fn stop(&self, handle: SessionHandle) -> Result<()>;

    fn get_status(&self, handle: &SessionHandle) -> SessionStatus;

    fn supports_gpu(&self) -> bool {
        self.supported_backends().iter().any(|b| !matches!(b, ComputeBackend::Cpu))
    }

    async fn run_inference(&self, config: &TrainingConfig, artifact_dir: &str, input_data: &[Vec<f32>]) -> Result<InferenceOutput>;
}

pub type EngineRef = Box<dyn Engine>;
