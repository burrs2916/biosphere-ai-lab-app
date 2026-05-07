use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteStorageConfig {
    pub storage_type: StorageType,
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub bucket: String,
    pub prefix: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    pub session_token: Option<String>,
    pub use_ssl: bool,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub chunk_size_bytes: usize,
}

impl Default for RemoteStorageConfig {
    fn default() -> Self {
        Self {
            storage_type: StorageType::S3,
            endpoint: None,
            region: Some("us-east-1".to_string()),
            bucket: String::new(),
            prefix: None,
            access_key: None,
            secret_key: None,
            session_token: None,
            use_ssl: true,
            timeout_secs: 30,
            max_retries: 3,
            chunk_size_bytes: 8 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageType {
    S3,
    GCS,
    AzureBlob,
    HDFS,
    MinIO,
    Custom,
}

impl std::fmt::Display for StorageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::S3 => write!(f, "s3"),
            Self::GCS => write!(f, "gcs"),
            Self::AzureBlob => write!(f, "azure_blob"),
            Self::HDFS => write!(f, "hdfs"),
            Self::MinIO => write!(f, "minio"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteObjectInfo {
    pub key: String,
    pub size_bytes: u64,
    pub last_modified: String,
    pub etag: Option<String>,
    pub storage_class: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteListResult {
    pub objects: Vec<RemoteObjectInfo>,
    pub total_count: usize,
    pub total_size_bytes: u64,
    pub prefix: String,
    pub truncated: bool,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSyncReport {
    pub source: String,
    pub destination: String,
    pub files_synced: usize,
    pub files_skipped: usize,
    pub files_failed: usize,
    pub bytes_transferred: u64,
    pub duration_secs: f64,
    pub errors: Vec<String>,
    pub status: SyncStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Success,
    Partial,
    Failed,
}

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::Partial => write!(f, "partial"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteStorageStats {
    pub storage_type: StorageType,
    pub bucket: String,
    pub total_objects: usize,
    pub total_size_bytes: u64,
    pub by_extension: Vec<ExtensionStats>,
    pub by_storage_class: Vec<StorageClassStats>,
    pub largest_object: Option<RemoteObjectInfo>,
    pub smallest_object: Option<RemoteObjectInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionStats {
    pub extension: String,
    pub count: usize,
    pub total_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageClassStats {
    pub storage_class: String,
    pub count: usize,
    pub total_size_bytes: u64,
}

pub struct RemoteStorageManager;

impl RemoteStorageManager {
    pub fn validate_config(config: &RemoteStorageConfig) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if config.bucket.is_empty() {
            errors.push("bucket name is required".to_string());
        }

        match config.storage_type {
            StorageType::S3 | StorageType::MinIO => {
                if config.region.is_none() && config.endpoint.is_none() {
                    errors.push("region or endpoint is required for S3/MinIO".to_string());
                }
            }
            StorageType::GCS => {
                if config.access_key.is_none() {
                    errors.push("service account credentials required for GCS".to_string());
                }
            }
            StorageType::AzureBlob => {
                if config.access_key.is_none() {
                    errors.push("connection string or access key required for Azure Blob".to_string());
                }
            }
            StorageType::HDFS => {
                if config.endpoint.is_none() {
                    errors.push("namenode endpoint required for HDFS".to_string());
                }
            }
            StorageType::Custom => {}
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn build_object_url(config: &RemoteStorageConfig, key: &str) -> String {
        let scheme = if config.use_ssl { "https" } else { "http" };

        match config.storage_type {
            StorageType::S3 => {
                if let Some(ref endpoint) = config.endpoint {
                    format!("{}://{}/{}/{}", scheme, endpoint, config.bucket, key)
                } else {
                    format!(
                        "{}://{}.s3.{}.amazonaws.com/{}",
                        scheme, config.bucket,
                        config.region.as_deref().unwrap_or("us-east-1"),
                        key
                    )
                }
            }
            StorageType::GCS => {
                format!("{}://storage.googleapis.com/{}/{}", scheme, config.bucket, key)
            }
            StorageType::AzureBlob => {
                format!(
                    "{}://{}.blob.core.windows.net/{}",
                    scheme, config.bucket, key
                )
            }
            StorageType::MinIO => {
                let endpoint = config.endpoint.as_deref().unwrap_or("localhost:9000");
                format!("{}://{}/{}/{}", scheme, endpoint, config.bucket, key)
            }
            StorageType::HDFS => {
                let endpoint = config.endpoint.as_deref().unwrap_or("localhost:9000");
                format!("hdfs://{}/{}/{}", endpoint, config.bucket, key)
            }
            StorageType::Custom => {
                let endpoint = config.endpoint.as_deref().unwrap_or("localhost");
                format!("{}://{}/{}/{}", scheme, endpoint, config.bucket, key)
            }
        }
    }

    pub fn estimate_transfer_time(
        total_bytes: u64,
        bandwidth_mbps: f64,
    ) -> TransferEstimate {
        if bandwidth_mbps <= 0.0 {
            return TransferEstimate {
                estimated_secs: f64::MAX,
                bandwidth_mbps: 0.0,
                parallel_recommendation: 1,
            };
        }

        let bytes_per_sec = bandwidth_mbps * 1_000_000.0 / 8.0;
        let estimated_secs = total_bytes as f64 / bytes_per_sec;

        let parallel = if estimated_secs > 3600.0 {
            8
        } else if estimated_secs > 600.0 {
            4
        } else if estimated_secs > 60.0 {
            2
        } else {
            1
        };

        TransferEstimate {
            estimated_secs,
            bandwidth_mbps,
            parallel_recommendation: parallel,
        }
    }

    pub fn recommend_storage_class(
        access_pattern: AccessPattern,
        total_size_bytes: u64,
    ) -> Vec<StorageClassRecommendation> {
        let mut recs = Vec::new();

        match access_pattern {
            AccessPattern::Frequent => {
                recs.push(StorageClassRecommendation {
                    storage_type: StorageType::S3,
                    class_name: "STANDARD".to_string(),
                    monthly_cost_estimate: total_size_bytes as f64 / 1_000_000_000.0 * 0.023,
                    retrieval_cost: 0.0,
                    description: "频繁访问，最低延迟".to_string(),
                });
            }
            AccessPattern::Infrequent => {
                recs.push(StorageClassRecommendation {
                    storage_type: StorageType::S3,
                    class_name: "STANDARD_IA".to_string(),
                    monthly_cost_estimate: total_size_bytes as f64 / 1_000_000_000.0 * 0.0125,
                    retrieval_cost: 0.01,
                    description: "不频繁访问，较低存储成本".to_string(),
                });
            }
            AccessPattern::Archive => {
                recs.push(StorageClassRecommendation {
                    storage_type: StorageType::S3,
                    class_name: "GLACIER".to_string(),
                    monthly_cost_estimate: total_size_bytes as f64 / 1_000_000_000.0 * 0.004,
                    retrieval_cost: 0.01,
                    description: "归档存储，最低成本，检索需数小时".to_string(),
                });
            }
            AccessPattern::Training => {
                recs.push(StorageClassRecommendation {
                    storage_type: StorageType::S3,
                    class_name: "STANDARD".to_string(),
                    monthly_cost_estimate: total_size_bytes as f64 / 1_000_000_000.0 * 0.023,
                    retrieval_cost: 0.0,
                    description: "训练数据需要频繁读取，推荐标准存储".to_string(),
                });
            }
        }

        recs
    }

    pub fn generate_sync_plan(
        local_files: &[LocalFileInfo],
        remote_objects: &[RemoteObjectInfo],
    ) -> SyncPlan {
        let mut to_upload = Vec::new();
        let mut to_download = Vec::new();
        let mut unchanged = Vec::new();
        let mut conflicts = Vec::new();

        let remote_map: HashMap<&str, &RemoteObjectInfo> = remote_objects.iter()
            .map(|o| (o.key.as_str(), o))
            .collect();

        for local in local_files {
            let remote_key = &local.relative_path;
            match remote_map.get(remote_key.as_str()) {
                Some(remote) => {
                    if local.size_bytes != remote.size_bytes {
                        conflicts.push(SyncConflict {
                            path: remote_key.clone(),
                            local_size: local.size_bytes,
                            remote_size: remote.size_bytes,
                            local_modified: local.modified_at.clone(),
                            remote_modified: remote.last_modified.clone(),
                            resolution: ConflictResolution::UploadNewer,
                        });
                    } else {
                        unchanged.push(remote_key.clone());
                    }
                }
                None => {
                    to_upload.push(remote_key.clone());
                }
            }
        }

        for remote in remote_objects {
            let local_exists = local_files.iter()
                .any(|l| l.relative_path == remote.key);
            if !local_exists {
                to_download.push(remote.key.clone());
            }
        }

        SyncPlan {
            to_upload,
            to_download,
            unchanged,
            conflicts,
            total_operations: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferEstimate {
    pub estimated_secs: f64,
    pub bandwidth_mbps: f64,
    pub parallel_recommendation: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessPattern {
    Frequent,
    Infrequent,
    Archive,
    Training,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageClassRecommendation {
    pub storage_type: StorageType,
    pub class_name: String,
    pub monthly_cost_estimate: f64,
    pub retrieval_cost: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalFileInfo {
    pub relative_path: String,
    pub size_bytes: u64,
    pub modified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPlan {
    pub to_upload: Vec<String>,
    pub to_download: Vec<String>,
    pub unchanged: Vec<String>,
    pub conflicts: Vec<SyncConflict>,
    pub total_operations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflict {
    pub path: String,
    pub local_size: u64,
    pub remote_size: u64,
    pub local_modified: String,
    pub remote_modified: String,
    pub resolution: ConflictResolution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    UploadNewer,
    DownloadNewer,
    KeepBoth,
    Skip,
}

impl std::fmt::Display for ConflictResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UploadNewer => write!(f, "upload_newer"),
            Self::DownloadNewer => write!(f, "download_newer"),
            Self::KeepBoth => write!(f, "keep_both"),
            Self::Skip => write!(f, "skip"),
        }
    }
}
