use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::types::TaskType;

pub mod commands;
pub mod events;
pub mod aggregate;
pub mod repository;
pub mod handler;
pub mod metrics;

pub use aggregate::{Experiment, ExperimentId, ExperimentStatus, ExperimentSummary};
pub use commands::ExperimentCommand;
pub use events::ExperimentEvent;
pub use repository::ExperimentRepository;
pub use metrics::{MetricsTimeline, MetricSeries, MetricPoint};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub artifact_type: String,
    pub path: String,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub checksum: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl ArtifactRef {
    pub fn new(artifact_type: String, path: String, size_bytes: u64) -> Self {
        Self {
            artifact_type,
            path,
            size_bytes,
            created_at: Utc::now(),
            version: None,
            description: None,
            checksum: None,
            metadata: None,
        }
    }

    pub fn with_version(mut self, version: String) -> Self {
        self.version = Some(version);
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_checksum(mut self, checksum: String) -> Self {
        self.checksum = Some(checksum);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn compute_checksum(data: &[u8]) -> String {
        let hash = crc32fast::hash(data);
        format!("{:08x}", hash)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentFilter {
    pub status: Option<ExperimentStatus>,
    pub tags: Vec<String>,
    pub name_contains: Option<String>,
    pub task_type: Option<TaskType>,
    pub group: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl Default for ExperimentFilter {
    fn default() -> Self {
        Self {
            status: None,
            tags: Vec::new(),
            name_contains: None,
            task_type: None,
            group: None,
            created_after: None,
            limit: None,
            offset: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_ref_new() {
        let artifact = ArtifactRef::new("model".to_string(), "/tmp/model.mpk.gz".to_string(), 2048);
        assert_eq!(artifact.artifact_type, "model");
        assert_eq!(artifact.path, "/tmp/model.mpk.gz");
        assert_eq!(artifact.size_bytes, 2048);
        assert!(artifact.version.is_none());
        assert!(artifact.description.is_none());
        assert!(artifact.checksum.is_none());
        assert!(artifact.metadata.is_none());
    }

    #[test]
    fn test_artifact_ref_builder_pattern() {
        let artifact = ArtifactRef::new("model".to_string(), "/tmp/model.mpk.gz".to_string(), 2048)
            .with_version("1.0".to_string())
            .with_description("Best model".to_string())
            .with_checksum("abcd1234".to_string())
            .with_metadata(serde_json::json!({"framework": "burn"}));

        assert_eq!(artifact.version, Some("1.0".to_string()));
        assert_eq!(artifact.description, Some("Best model".to_string()));
        assert_eq!(artifact.checksum, Some("abcd1234".to_string()));
        assert!(artifact.metadata.is_some());
    }

    #[test]
    fn test_artifact_ref_compute_checksum() {
        let data1 = b"hello world";
        let data2 = b"hello world";
        let data3 = b"different data";

        let checksum1 = ArtifactRef::compute_checksum(data1);
        let checksum2 = ArtifactRef::compute_checksum(data2);
        let checksum3 = ArtifactRef::compute_checksum(data3);

        assert_eq!(checksum1, checksum2);
        assert_ne!(checksum1, checksum3);
        assert_eq!(checksum1.len(), 8);
    }

    #[test]
    fn test_experiment_filter_default() {
        let filter = ExperimentFilter::default();
        assert!(filter.status.is_none());
        assert!(filter.tags.is_empty());
        assert!(filter.name_contains.is_none());
        assert!(filter.task_type.is_none());
        assert!(filter.limit.is_none());
    }
}
