use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

use crate::core::{LabError, Result};
use crate::types::DataFormat;

use super::connector::{
    ConnectConfig, DataConnector, DiscoveredItem, ResolvedDataSource, ScanOptions,
    detect_format_from_path,
};

pub struct LocalFsConnector;

impl LocalFsConnector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalFsConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataConnector for LocalFsConnector {
    fn id(&self) -> &str {
        "local_fs"
    }

    fn name(&self) -> &str {
        "Local Filesystem"
    }

    fn description(&self) -> &str {
        "Scan and discover datasets from local filesystem directories"
    }

    fn connector_type(&self) -> &str {
        "local"
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
        if uri.starts_with("http://") || uri.starts_with("https://") || uri.starts_with("s3://") {
            return false;
        }
        Path::new(uri).exists()
    }

    async fn scan(&self, config: &ConnectConfig, options: &ScanOptions) -> Result<Vec<DiscoveredItem>> {
        let root = Path::new(&config.uri);
        if !root.exists() {
            return Err(LabError::Custom(format!("Path does not exist: {}", config.uri)));
        }

        let mut items = Vec::new();
        self.scan_recursive(root, root, options, 0, &mut items)?;

        if let Some(max) = options.max_results {
            items.truncate(max);
        }

        items.sort_by(|a, b| {
            b.size_bytes.cmp(&a.size_bytes)
        });

        Ok(items)
    }

    async fn test_connection(&self, config: &ConnectConfig) -> Result<bool> {
        let path = Path::new(&config.uri);
        Ok(path.exists() && path.is_dir())
    }

    async fn resolve_item(&self, item: &DiscoveredItem) -> Result<ResolvedDataSource> {
        let path = Path::new(&item.path);
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(ResolvedDataSource {
            name,
            format: item.format,
            path: item.path.clone(),
            source_type: "local_fs".to_string(),
            source_uri: item.path.clone(),
            size_bytes: item.size_bytes,
            load_config_params: HashMap::new(),
        })
    }
}

impl LocalFsConnector {
    fn scan_recursive(
        &self,
        root: &Path,
        current: &Path,
        options: &ScanOptions,
        depth: usize,
        items: &mut Vec<DiscoveredItem>,
    ) -> Result<()> {
        if let Some(max_depth) = options.max_depth {
            if depth > max_depth {
                return Ok(());
            }
        }

        if let Some(max) = options.max_results {
            if items.len() >= max {
                return Ok(());
            }
        }

        let entries = std::fs::read_dir(current)
            .map_err(|e| LabError::Custom(format!("Cannot read dir {:?}: {}", current, e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| LabError::Custom(format!("Dir entry error: {}", e)))?;
            let path = entry.path();

            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");

            if options.exclude_patterns.iter().any(|p| file_name.contains(p.as_str()) || file_name == p.as_str()) {
                continue;
            }

            if path.is_dir() {
                if path.is_symlink() {
                    continue;
                }
                if options.recursive {
                    self.scan_recursive(root, &path, options, depth + 1, items)?;
                }

                let child_count = std::fs::read_dir(&path)
                    .ok()
                    .map(|mut rd| rd.by_ref().count())
                    .unwrap_or(0);

                if child_count > 0 {
                    let relative = path.strip_prefix(root).unwrap_or(&path);
                    let mut metadata = HashMap::new();
                    metadata.insert("child_count".into(), serde_json::json!(child_count));
                    metadata.insert("relative_path".into(), serde_json::Value::String(relative.to_string_lossy().into_owned()));

                    let has_data_files = self.dir_has_data_files(&path, options);
                    if has_data_files {
                        items.push(DiscoveredItem {
                            name: file_name.to_string(),
                            path: path.to_string_lossy().into_owned(),
                            format: DataFormat::Binary,
                            size_bytes: 0,
                            connector_type: "local_fs".to_string(),
                            is_directory: true,
                            children_count: Some(child_count),
                            metadata,
                        });
                    }
                }
            } else {
                let format = detect_format_from_path(&path.to_string_lossy());

                let should_include = if !options.extensions.is_empty() {
                    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                    options.extensions.iter().any(|e| e.to_lowercase() == ext.to_lowercase())
                } else {
                    format.is_some()
                };

                if !should_include {
                    continue;
                }

                let file_metadata = std::fs::metadata(&path)
                    .map_err(|e| LabError::Custom(format!("Cannot read metadata {:?}: {}", path, e)))?;

                let relative = path.strip_prefix(root).unwrap_or(&path);
                let mut item_metadata = HashMap::new();
                item_metadata.insert("relative_path".into(), serde_json::Value::String(relative.to_string_lossy().into_owned()));
                item_metadata.insert(
                    "extension".into(),
                    serde_json::Value::String(
                        path.extension().and_then(|s| s.to_str()).unwrap_or("").to_string(),
                    ),
                );
                item_metadata.insert("modified".into(), serde_json::json!(
                    file_metadata.modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                ));

                items.push(DiscoveredItem {
                    name: file_name.to_string(),
                    path: path.to_string_lossy().into_owned(),
                    format: format.unwrap_or(DataFormat::Binary),
                    size_bytes: file_metadata.len(),
                    connector_type: "local_fs".to_string(),
                    is_directory: false,
                    children_count: None,
                    metadata: item_metadata,
                });
            }
        }

        Ok(())
    }

    fn dir_has_data_files(&self, dir: &Path, options: &ScanOptions) -> bool {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let format = detect_format_from_path(&path.to_string_lossy());
                    if format.is_some() {
                        if !options.extensions.is_empty() {
                            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                            if options.extensions.iter().any(|e| e.to_lowercase() == ext.to_lowercase()) {
                                return true;
                            }
                        } else {
                            return true;
                        }
                    }
                } else if path.is_dir() && options.recursive {
                    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    if !options.exclude_patterns.iter().any(|p| file_name.contains(p.as_str())) {
                        if self.dir_has_data_files(&path, options) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}
