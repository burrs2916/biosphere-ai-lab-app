use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::Result;
use crate::types::DataFormat;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredItem {
    pub name: String,
    pub path: String,
    pub format: DataFormat,
    pub size_bytes: u64,
    pub connector_type: String,
    pub is_directory: bool,
    pub children_count: Option<usize>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub connector_type: String,
    pub supported_formats: Vec<DataFormat>,
    pub requires_auth: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanOptions {
    pub recursive: bool,
    pub max_depth: Option<usize>,
    pub extensions: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub max_results: Option<usize>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            recursive: true,
            max_depth: Some(3),
            extensions: Vec::new(),
            exclude_patterns: vec![
                ".git".into(),
                "node_modules".into(),
                ".DS_Store".into(),
                "__pycache__".into(),
                ".venv".into(),
                "target".into(),
            ],
            max_results: Some(500),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectConfig {
    pub uri: String,
    pub params: HashMap<String, serde_json::Value>,
}

impl ConnectConfig {
    pub fn local_path(path: impl Into<String>) -> Self {
        Self {
            uri: path.into(),
            params: HashMap::new(),
        }
    }

    pub fn http_url(url: impl Into<String>) -> Self {
        Self {
            uri: url.into(),
            params: HashMap::new(),
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.params.insert(key.into(), value);
        self
    }
}

#[async_trait]
pub trait DataConnector: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn connector_type(&self) -> &str;
    fn supported_formats(&self) -> Vec<DataFormat>;
    fn requires_auth(&self) -> bool {
        false
    }

    fn info(&self) -> ConnectorInfo {
        ConnectorInfo {
            id: self.id().to_string(),
            name: self.name().to_string(),
            description: self.description().to_string(),
            connector_type: self.connector_type().to_string(),
            supported_formats: self.supported_formats(),
            requires_auth: self.requires_auth(),
        }
    }

    fn can_handle(&self, uri: &str) -> bool;

    async fn scan(&self, config: &ConnectConfig, options: &ScanOptions) -> Result<Vec<DiscoveredItem>>;

    async fn test_connection(&self, config: &ConnectConfig) -> Result<bool>;

    async fn resolve_item(&self, item: &DiscoveredItem) -> Result<ResolvedDataSource>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedDataSource {
    pub name: String,
    pub format: DataFormat,
    pub path: String,
    pub source_type: String,
    pub source_uri: String,
    pub size_bytes: u64,
    pub load_config_params: HashMap<String, serde_json::Value>,
}

pub fn detect_format_from_path(path: &str) -> Option<DataFormat> {
    let lower = path.to_lowercase();
    if lower.ends_with(".csv") || lower.ends_with(".tsv") {
        Some(DataFormat::Csv)
    } else if lower.ends_with(".json") || lower.ends_with(".jsonl") || lower.ends_with(".ndjson") {
        Some(DataFormat::Json)
    } else if lower.ends_with(".parquet") {
        Some(DataFormat::Parquet)
    } else if lower.ends_with(".xlsx") || lower.ends_with(".xls") {
        Some(DataFormat::Excel)
    } else if lower.ends_with(".tfrecord") || lower.ends_with(".tfrecords") {
        Some(DataFormat::TfRecord)
    } else if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")
        || lower.ends_with(".gif") || lower.ends_with(".bmp") || lower.ends_with(".webp")
        || lower.ends_with(".tiff") || lower.ends_with(".svg")
    {
        Some(DataFormat::Image)
    } else if lower.ends_with(".txt") || lower.ends_with(".md") || lower.ends_with(".rst")
        || lower.ends_with(".html") || lower.ends_with(".xml")
    {
        Some(DataFormat::Text)
    } else if lower.ends_with(".bin") || lower.ends_with(".dat") {
        Some(DataFormat::Binary)
    } else {
        None
    }
}

pub fn detect_format_from_content(data: &[u8]) -> Option<DataFormat> {
    if data.len() < 2 {
        return None;
    }

    if data.starts_with(b"PAR1") {
        return Some(DataFormat::Parquet);
    }

    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return Some(DataFormat::Image);
    }
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(DataFormat::Image);
    }
    if data.starts_with(b"GIF8") {
        return Some(DataFormat::Image);
    }

    let head = std::str::from_utf8(&data[..data.len().min(1024)]).ok()?;
    let trimmed = head.trim_start();

    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if serde_json::from_slice::<serde_json::Value>(data).is_ok() {
            return Some(DataFormat::Json);
        }
    }

    if trimmed.contains(',') && trimmed.contains('\n') {
        let first_line = trimmed.lines().next()?;
        let comma_count = first_line.matches(',').count();
        if comma_count >= 1 {
            return Some(DataFormat::Csv);
        }
    }

    None
}

pub fn format_file_size(size_bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if size_bytes >= GB {
        format!("{:.2} GB", size_bytes as f64 / GB as f64)
    } else if size_bytes >= MB {
        format!("{:.2} MB", size_bytes as f64 / MB as f64)
    } else if size_bytes >= KB {
        format!("{:.2} KB", size_bytes as f64 / KB as f64)
    } else {
        format!("{} B", size_bytes)
    }
}
