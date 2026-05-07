use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

use crate::core::{LabError, Result};
use crate::types::DataFormat;

use super::connector::{
    ConnectConfig, DataConnector, DiscoveredItem, ResolvedDataSource, ScanOptions,
    detect_format_from_path,
};

pub struct HttpConnector {
    client: reqwest::Client,
}

impl HttpConnector {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
    }
}

impl Default for HttpConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataConnector for HttpConnector {
    fn id(&self) -> &str {
        "http"
    }

    fn name(&self) -> &str {
        "HTTP/HTTPS"
    }

    fn description(&self) -> &str {
        "Connect to remote datasets via HTTP/HTTPS URLs"
    }

    fn connector_type(&self) -> &str {
        "http"
    }

    fn supported_formats(&self) -> Vec<DataFormat> {
        vec![
            DataFormat::Csv,
            DataFormat::Json,
            DataFormat::Parquet,
            DataFormat::Image,
            DataFormat::Text,
            DataFormat::Binary,
            DataFormat::Excel,
            DataFormat::TfRecord,
            DataFormat::HuggingFace,
        ]
    }

    fn can_handle(&self, uri: &str) -> bool {
        uri.starts_with("http://") || uri.starts_with("https://")
    }

    async fn scan(&self, config: &ConnectConfig, _options: &ScanOptions) -> Result<Vec<DiscoveredItem>> {
        let url = &config.uri;

        let response = match self.client.head(url).send().await {
            Ok(r) => r,
            Err(_) => self.client.get(url).send().await
                .map_err(|e| LabError::Custom(format!("HTTP request failed for {}: {}", url, e)))?,
        };

        let content_length = response.headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        let content_type = response.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let format = detect_format_from_path(url)
            .or_else(|| detect_format_from_content_type(content_type));

        let name = url
            .rsplit('/')
            .next()
            .unwrap_or("remote_data")
            .to_string();

        let mut metadata = HashMap::new();
        metadata.insert("content_type".into(), serde_json::Value::String(content_type.to_string()));
        metadata.insert("url".into(), serde_json::Value::String(url.clone()));
        metadata.insert("status_code".into(), serde_json::json!(response.status().as_u16()));

        if let Some(last_modified) = response.headers().get("last-modified").and_then(|v| v.to_str().ok()) {
            metadata.insert("last_modified".into(), serde_json::Value::String(last_modified.to_string()));
        }

        Ok(vec![DiscoveredItem {
            name,
            path: url.clone(),
            format: format.unwrap_or(DataFormat::Binary),
            size_bytes: content_length,
            connector_type: "http".to_string(),
            is_directory: false,
            children_count: None,
            metadata,
        }])
    }

    async fn test_connection(&self, config: &ConnectConfig) -> Result<bool> {
        let response = match self.client.head(&config.uri).send().await {
            Ok(r) => r,
            Err(_) => self.client.get(&config.uri).send().await
                .map_err(|e| LabError::Custom(format!("Connection test failed: {}", e)))?,
        };

        Ok(response.status().is_success())
    }

    async fn resolve_item(&self, item: &DiscoveredItem) -> Result<ResolvedDataSource> {
        let url = &item.path;
        let name = url
            .rsplit('/')
            .next()
            .unwrap_or("remote_data")
            .to_string();

        let local_cache_dir = std::env::temp_dir().join("biosphere_cache");
        if !local_cache_dir.exists() {
            let _ = std::fs::create_dir_all(&local_cache_dir);
        }

        let ext = Path::new(&item.path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("bin");
        let cached_filename = format!("http_{}.{}", 
            crc32fast::hash(url.as_bytes()),
            ext
        );
        let cached_path = local_cache_dir.join(&cached_filename);

        let local_path = if cached_path.exists() {
            cached_path.to_string_lossy().into_owned()
        } else {
            let response = self.client.get(url).send().await
                .map_err(|e| LabError::Custom(format!("Download failed: {}", e)))?;

            let bytes = response.bytes().await
                .map_err(|e| LabError::Custom(format!("Read response failed: {}", e)))?;

            std::fs::write(&cached_path, &bytes)
                .map_err(|e| LabError::Custom(format!("Cache write failed: {}", e)))?;

            cached_path.to_string_lossy().into_owned()
        };

        let mut load_params = HashMap::new();
        load_params.insert("original_url".into(), serde_json::Value::String(url.clone()));
        load_params.insert("cached_locally".into(), serde_json::json!(true));

        Ok(ResolvedDataSource {
            name,
            format: item.format,
            path: local_path,
            source_type: "http".to_string(),
            source_uri: url.clone(),
            size_bytes: item.size_bytes,
            load_config_params: load_params,
        })
    }
}

fn detect_format_from_content_type(content_type: &str) -> Option<DataFormat> {
    let ct = content_type.to_lowercase();
    if ct.contains("csv") || ct.contains("text/csv") {
        Some(DataFormat::Csv)
    } else if ct.contains("json") || ct.contains("application/json") {
        Some(DataFormat::Json)
    } else if ct.contains("parquet") || ct.contains("application/parquet") {
        Some(DataFormat::Parquet)
    } else if ct.contains("image/") {
        Some(DataFormat::Image)
    } else if ct.contains("text/plain") || ct.contains("text/html") {
        Some(DataFormat::Text)
    } else {
        None
    }
}
