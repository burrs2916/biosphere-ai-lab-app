use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core::Result;
use crate::core::config::TrainingConfig;
use crate::types::PluginId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: RemoteAuth,
    pub working_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemoteAuth {
    Password { password: String },
    Key { key_path: String, passphrase: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionHandle {
    pub id: String,
    pub host: String,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteStatus {
    pub connected: bool,
    pub gpu_available: bool,
    pub available_memory_mb: u64,
    pub python_available: bool,
    pub working_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployResult {
    pub remote_id: String,
    pub working_dir: String,
    pub status: String,
}

#[async_trait]
pub trait RemoteBackend: Send + Sync {
    fn id(&self) -> &PluginId;
    fn name(&self) -> &str;

    async fn connect(&self, config: &RemoteConfig) -> Result<ConnectionHandle>;
    async fn disconnect(&self, handle: &ConnectionHandle) -> Result<()>;
    async fn deploy(&self, handle: &ConnectionHandle, config: &TrainingConfig) -> Result<DeployResult>;
    async fn get_status(&self, handle: &ConnectionHandle) -> Result<RemoteStatus>;
    async fn sync_data(&self, handle: &ConnectionHandle, local_path: &str, remote_path: &str) -> Result<()>;
    async fn fetch_results(&self, handle: &ConnectionHandle, remote_path: &str, local_path: &str) -> Result<()>;
    async fn start_training(&self, handle: &ConnectionHandle, remote_id: &str) -> Result<()>;
    async fn stop_training(&self, handle: &ConnectionHandle, remote_id: &str) -> Result<()>;
}
