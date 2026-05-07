use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::config::TrainingConfig;
use crate::types::TaskType;
use crate::domain::model::aggregate::ModelId;

use super::metrics::MetricsTimeline;
use super::ArtifactRef;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExperimentId(String);

impl ExperimentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ExperimentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ExperimentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExperimentStatus {
    Created,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Archived,
}

impl ExperimentStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled | Self::Archived)
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running)
    }
}

impl std::fmt::Display for ExperimentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Archived => write!(f, "archived"),
        }
    }
}

impl std::str::FromStr for ExperimentStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "created" => Ok(Self::Created),
            "running" => Ok(Self::Running),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "archived" => Ok(Self::Archived),
            _ => Err(format!("Unknown experiment status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub commit_hash: Option<String>,
    pub branch: Option<String>,
    pub commit_message: Option<String>,
    pub is_dirty: Option<bool>,
    pub remote_url: Option<String>,
}

impl GitInfo {
    pub fn capture() -> Self {
        let commit_hash = Self::run_git(&["rev-parse", "HEAD"]);
        let branch = Self::run_git(&["rev-parse", "--abbrev-ref", "HEAD"]);
        let commit_message = Self::run_git(&["log", "-1", "--pretty=%s"]);
        let is_dirty = Self::run_git(&["status", "--porcelain"])
            .map(|s| !s.is_empty());
        let remote_url = Self::run_git(&["remote", "get-url", "origin"]);

        Self {
            commit_hash,
            branch,
            commit_message,
            is_dirty,
            remote_url,
        }
    }

    fn run_git(args: &[&str]) -> Option<String> {
        std::process::Command::new("git")
            .args(args)
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                } else {
                    None
                }
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub rust_version: Option<String>,
    pub burn_version: Option<String>,
    pub crates: HashMap<String, String>,
}

impl DependencyInfo {
    pub fn capture() -> Self {
        let rust_version = std::process::Command::new("rustc")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                } else {
                    None
                }
            });

        Self {
            rust_version,
            burn_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            crates: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub cpu_cores: usize,
    pub total_memory_mb: u64,
    pub hostname: String,
}

impl SystemInfo {
    pub fn capture() -> Self {
        let sys = sysinfo::System::new_all();
        Self {
            os: std::env::consts::OS.to_string(),
            os_version: SystemInfo::get_os_version(&sys),
            cpu_cores: sys.cpus().len(),
            total_memory_mb: sys.total_memory() / 1024 / 1024,
            hostname: SystemInfo::get_hostname(),
        }
    }

    fn get_os_version(_sys: &sysinfo::System) -> String {
        let name = sysinfo::System::name().unwrap_or_default();
        let version = sysinfo::System::os_version().unwrap_or_default();
        format!("{} {}", name, version)
    }

    fn get_hostname() -> String {
        sysinfo::System::host_name().unwrap_or_else(|| "unknown".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub git: Option<GitInfo>,
    pub dependencies: Option<DependencyInfo>,
    pub system: Option<SystemInfo>,
    pub captured_at: chrono::DateTime<chrono::Utc>,
}

impl EnvironmentInfo {
    pub fn capture_all() -> Self {
        Self {
            git: Some(GitInfo::capture()),
            dependencies: Some(DependencyInfo::capture()),
            system: Some(SystemInfo::capture()),
            captured_at: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub level: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub id: ExperimentId,
    pub name: String,
    pub status: ExperimentStatus,
    pub task_type: TaskType,
    pub config: TrainingConfig,
    pub metrics: MetricsTimeline,
    pub params: HashMap<String, serde_json::Value>,
    pub tags: Vec<String>,
    pub artifacts: Vec<ArtifactRef>,
    pub model_id: Option<ModelId>,
    pub dataset_id: Option<String>,
    pub dataset_version: Option<String>,
    pub group: Option<String>,
    pub environment: Option<EnvironmentInfo>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
    pub final_metrics: Option<serde_json::Value>,
    pub description: Option<String>,
}

use std::collections::HashMap;

impl Experiment {
    pub fn create(name: String, config: TrainingConfig) -> Self {
        let now = chrono::Utc::now();
        let task_type = config.task_type;
        Self {
            id: ExperimentId::new(),
            name,
            status: ExperimentStatus::Created,
            task_type,
            config,
            metrics: MetricsTimeline::new(),
            params: HashMap::new(),
            tags: Vec::new(),
            artifacts: Vec::new(),
            model_id: None,
            dataset_id: None,
            dataset_version: None,
            group: None,
            environment: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
            error_message: None,
            final_metrics: None,
            description: None,
        }
    }

    pub fn track(&mut self, metric_name: &str, value: f64, step: u64) {
        self.metrics.record(metric_name, value, step);
        self.updated_at = chrono::Utc::now();
    }

    pub fn start(&mut self) -> Result<(), String> {
        if self.status != ExperimentStatus::Created {
            return Err(format!("Cannot start experiment in {} state, expected Created", self.status));
        }
        self.status = ExperimentStatus::Running;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn restart(&mut self) -> Result<(), String> {
        if !self.status.is_terminal() && self.status != ExperimentStatus::Created {
            return Err(format!("Cannot restart experiment in {} state, expected terminal state (Completed/Failed/Cancelled) or Created", self.status));
        }
        self.status = ExperimentStatus::Running;
        self.updated_at = chrono::Utc::now();
        self.completed_at = None;
        self.error_message = None;
        Ok(())
    }

    pub fn link_dataset(&mut self, dataset_id: String, dataset_version: String) {
        self.dataset_id = Some(dataset_id);
        self.dataset_version = Some(dataset_version);
        self.updated_at = chrono::Utc::now();
    }

    pub fn resume(&mut self) -> Result<(), String> {
        if self.status != ExperimentStatus::Paused {
            return Err(format!("Cannot resume experiment in {} state, expected Paused", self.status));
        }
        self.status = ExperimentStatus::Running;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), String> {
        if self.status != ExperimentStatus::Running {
            return Err(format!("Cannot pause experiment in {} state", self.status));
        }
        self.status = ExperimentStatus::Paused;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn complete(&mut self, final_metrics: serde_json::Value) -> Result<(), String> {
        if self.status != ExperimentStatus::Running && self.status != ExperimentStatus::Paused {
            return Err(format!("Cannot complete experiment in {} state, expected Running or Paused", self.status));
        }
        self.status = ExperimentStatus::Completed;
        let now = chrono::Utc::now();
        self.updated_at = now;
        self.completed_at = Some(now);
        self.final_metrics = Some(final_metrics);
        Ok(())
    }

    pub fn fail(&mut self, error: String) -> Result<(), String> {
        if self.status != ExperimentStatus::Running && self.status != ExperimentStatus::Paused {
            return Err(format!("Cannot fail experiment in {} state, expected Running or Paused", self.status));
        }
        self.status = ExperimentStatus::Failed;
        let now = chrono::Utc::now();
        self.updated_at = now;
        self.completed_at = Some(now);
        self.error_message = Some(error);
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), String> {
        if self.status != ExperimentStatus::Running && self.status != ExperimentStatus::Paused && self.status != ExperimentStatus::Created {
            return Err(format!("Cannot cancel experiment in {} state", self.status));
        }
        self.status = ExperimentStatus::Cancelled;
        let now = chrono::Utc::now();
        self.updated_at = now;
        self.completed_at = Some(now);
        Ok(())
    }

    pub fn archive(&mut self) -> Result<(), String> {
        if self.status.is_terminal() && self.status != ExperimentStatus::Archived {
            self.status = ExperimentStatus::Archived;
            let now = chrono::Utc::now();
            self.updated_at = now;
            Ok(())
        } else {
            Err(format!("Cannot archive experiment in {} state, only terminal states can be archived", self.status))
        }
    }

    pub fn restore(&mut self) -> Result<ExperimentStatus, String> {
        if self.status != ExperimentStatus::Archived {
            return Err(format!("Cannot restore experiment in {} state, expected Archived", self.status));
        }
        let restored = if self.error_message.is_some() {
            ExperimentStatus::Failed
        } else if self.final_metrics.is_some() {
            ExperimentStatus::Completed
        } else {
            ExperimentStatus::Created
        };
        self.status = restored;
        self.updated_at = chrono::Utc::now();
        Ok(restored)
    }

    pub fn set_param(&mut self, key: String, value: serde_json::Value) {
        self.params.insert(key, value);
        self.updated_at = chrono::Utc::now();
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = chrono::Utc::now();
        }
    }

    pub fn add_artifact(&mut self, artifact: ArtifactRef) {
        if !self.artifacts.iter().any(|a| a.path == artifact.path) {
            self.artifacts.push(artifact);
            self.updated_at = chrono::Utc::now();
        }
    }

    pub fn link_model(&mut self, model_id: ModelId) {
        self.model_id = Some(model_id);
        self.updated_at = chrono::Utc::now();
    }

    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
        self.updated_at = chrono::Utc::now();
    }

    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
        self.updated_at = chrono::Utc::now();
    }

    pub fn set_group(&mut self, group: String) {
        self.group = Some(group);
        self.updated_at = chrono::Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentSummary {
    pub id: ExperimentId,
    pub name: String,
    pub status: ExperimentStatus,
    pub task_type: TaskType,
    pub tags: Vec<String>,
    pub dataset_id: Option<String>,
    pub dataset_version: Option<String>,
    pub group: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub metric_names: Vec<String>,
    pub best_metrics: HashMap<String, f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::TrainingConfig;
    use crate::types::{ComputeBackend, DataFormat, TaskType};

    fn make_test_config() -> TrainingConfig {
        TrainingConfig {
            session_name: "test".to_string(),
            task_type: TaskType::Classification,
            engine_id: "burn".to_string(),
            model_id: "mlp".to_string(),
            data_source_id: "test-ds".to_string(),
            data_path: "/tmp/test.csv".to_string(),
            epochs: 10,
            batch_size: 32,
            learning_rate: 0.001,
            optimizer: crate::core::config::OptimizerConfig::Adam {
                beta1: 0.9,
                beta2: 0.999,
                weight_decay: None,
            },
            loss_function: "cross_entropy".to_string(),
            compute_backend: ComputeBackend::Cpu,
            data_format: DataFormat::Csv,
            validation_split: 0.2,
            test_split: 0.1,
            shuffle: true,
            seed: Some(42),
            dataset_id: None,
            dataset_version: None,
            split_name: None,
            split_indices: None,
            checkpoint_interval: Some(5),
            early_stopping: None,
            lr_scheduler: crate::core::config::LrSchedulerConfig::Constant,
            target_columns: vec!["label".to_string()],
            feature_columns: vec!["x".to_string()],
            custom_params: HashMap::new(),
        }
    }

    #[test]
    fn test_experiment_create() {
        let config = make_test_config();
        let exp = Experiment::create("test-exp".to_string(), config);

        assert_eq!(exp.name, "test-exp");
        assert_eq!(exp.status, ExperimentStatus::Created);
        assert!(exp.metrics.is_empty());
        assert!(exp.tags.is_empty());
        assert!(exp.artifacts.is_empty());
        assert!(exp.model_id.is_none());
    }

    #[test]
    fn test_experiment_lifecycle() {
        let config = make_test_config();
        let mut exp = Experiment::create("test-exp".to_string(), config);

        assert!(exp.start().is_ok());
        assert_eq!(exp.status, ExperimentStatus::Running);

        assert!(exp.pause().is_ok());
        assert_eq!(exp.status, ExperimentStatus::Paused);

        assert!(exp.resume().is_ok());
        assert_eq!(exp.status, ExperimentStatus::Running);

        assert!(exp.complete(serde_json::json!({"accuracy": 0.95})).is_ok());
        assert_eq!(exp.status, ExperimentStatus::Completed);
        assert!(exp.completed_at.is_some());
        assert!(exp.final_metrics.is_some());
    }

    #[test]
    fn test_experiment_cannot_start_twice() {
        let config = make_test_config();
        let mut exp = Experiment::create("test-exp".to_string(), config);

        assert!(exp.start().is_ok());
        assert!(exp.start().is_err());
    }

    #[test]
    fn test_experiment_cannot_pause_when_created() {
        let config = make_test_config();
        let mut exp = Experiment::create("test-exp".to_string(), config);

        assert!(exp.pause().is_err());
    }

    #[test]
    fn test_experiment_cannot_resume_when_not_paused() {
        let config = make_test_config();
        let mut exp = Experiment::create("test-exp".to_string(), config);

        assert!(exp.resume().is_err());
    }

    #[test]
    fn test_experiment_fail() {
        let config = make_test_config();
        let mut exp = Experiment::create("test-exp".to_string(), config);
        exp.start().unwrap();
        exp.fail("OOM".to_string()).unwrap();

        assert_eq!(exp.status, ExperimentStatus::Failed);
        assert_eq!(exp.error_message, Some("OOM".to_string()));
        assert!(exp.completed_at.is_some());
    }

    #[test]
    fn test_experiment_cancel() {
        let config = make_test_config();
        let mut exp = Experiment::create("test-exp".to_string(), config);
        exp.start().unwrap();
        exp.cancel().unwrap();

        assert_eq!(exp.status, ExperimentStatus::Cancelled);
        assert!(exp.completed_at.is_some());
    }

    #[test]
    fn test_experiment_track_metric() {
        let config = make_test_config();
        let mut exp = Experiment::create("test-exp".to_string(), config);

        exp.track("loss", 0.5, 1);
        exp.track("loss", 0.3, 2);
        exp.track("accuracy", 0.8, 1);

        assert_eq!(exp.metrics.total_points(), 3);
        assert_eq!(exp.metrics.get_series("loss").unwrap().len(), 2);
    }

    #[test]
    fn test_experiment_tags() {
        let config = make_test_config();
        let mut exp = Experiment::create("test-exp".to_string(), config);

        exp.add_tag("v1".to_string());
        exp.add_tag("baseline".to_string());
        exp.add_tag("v1".to_string());

        assert_eq!(exp.tags.len(), 2);
        exp.remove_tag("baseline");
        assert_eq!(exp.tags.len(), 1);
    }

    #[test]
    fn test_experiment_params() {
        let config = make_test_config();
        let mut exp = Experiment::create("test-exp".to_string(), config);

        exp.set_param("lr".to_string(), serde_json::json!(0.001));
        exp.set_param("batch_size".to_string(), serde_json::json!(32));

        assert_eq!(exp.params.len(), 2);
        assert_eq!(exp.params["lr"], serde_json::json!(0.001));
    }

    #[test]
    fn test_experiment_add_artifact() {
        let config = make_test_config();
        let mut exp = Experiment::create("test-exp".to_string(), config);

        let artifact = crate::domain::experiment::ArtifactRef::new(
            "model".to_string(),
            "/tmp/model.mpk.gz".to_string(),
            1024,
        );
        exp.add_artifact(artifact);

        assert_eq!(exp.artifacts.len(), 1);
        assert_eq!(exp.artifacts[0].artifact_type, "model");
    }

    #[test]
    fn test_experiment_link_model() {
        let config = make_test_config();
        let mut exp = Experiment::create("test-exp".to_string(), config);

        exp.link_model(ModelId::from_str("model-123"));
        assert!(exp.model_id.is_some());
    }

    #[test]
    fn test_experiment_status_is_terminal() {
        assert!(!ExperimentStatus::Created.is_terminal());
        assert!(!ExperimentStatus::Running.is_terminal());
        assert!(!ExperimentStatus::Paused.is_terminal());
        assert!(ExperimentStatus::Completed.is_terminal());
        assert!(ExperimentStatus::Failed.is_terminal());
        assert!(ExperimentStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_experiment_status_is_active() {
        assert!(ExperimentStatus::Running.is_active());
        assert!(!ExperimentStatus::Created.is_active());
        assert!(!ExperimentStatus::Paused.is_active());
    }

    #[test]
    fn test_experiment_restart() {
        let config = make_test_config();
        let mut exp = Experiment::create("test-exp".to_string(), config);
        exp.start().unwrap();
        exp.fail("error".to_string()).unwrap();

        assert!(exp.restart().is_ok());
        assert_eq!(exp.status, ExperimentStatus::Running);
        assert!(exp.error_message.is_none());
        assert!(exp.completed_at.is_none());
    }

    #[test]
    fn test_experiment_id_uniqueness() {
        let id1 = ExperimentId::new();
        let id2 = ExperimentId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_experiment_id_from_str() {
        let id = ExperimentId::from_str("test-id-123");
        assert_eq!(id.as_str(), "test-id-123");
    }
}
