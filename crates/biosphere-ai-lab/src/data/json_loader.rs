use async_trait::async_trait;
use serde_json::Value;

use crate::core::{LabError, Result};
use crate::core::config::DataLoadConfig;
use crate::types::{DataFormat, PluginId};
use super::data_trait::{DataPreview, DatasetInfo, DataSource, PreprocessType};

pub struct JsonLoader {
    id: PluginId,
}

impl JsonLoader {
    pub fn new() -> Self {
        Self {
            id: PluginId::new("json"),
        }
    }
}

impl Default for JsonLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataSource for JsonLoader {
    fn id(&self) -> &PluginId {
        &self.id
    }

    fn name(&self) -> &str {
        "JSON Data Loader"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Load and parse JSON data files (array of objects format)"
    }

    fn data_format(&self) -> DataFormat {
        DataFormat::Json
    }

    fn supported_preprocessing(&self) -> Vec<PreprocessType> {
        vec![
            PreprocessType::Normalize,
            PreprocessType::Standardize,
            PreprocessType::OneHotEncode,
            PreprocessType::LabelEncode,
            PreprocessType::FillMissing,
            PreprocessType::DropMissing,
        ]
    }

    async fn load(&self, config: &DataLoadConfig) -> Result<DatasetInfo> {
        let path = std::path::Path::new(&config.path);
        if !path.exists() {
            return Err(LabError::DataLoadFailed(format!("File not found: {}", config.path)));
        }

        let metadata = std::fs::metadata(&config.path)
            .map_err(|e| LabError::DataLoadFailed(format!("Cannot read file metadata: {}", e)))?;

        let content = std::fs::read_to_string(&config.path)
            .map_err(|e| LabError::DataLoadFailed(format!("Cannot read file: {}", e)))?;

        let json_value: Value = serde_json::from_str(&content)
            .map_err(|e| LabError::DataLoadFailed(format!("JSON parse error: {}", e)))?;

        let array = json_value.as_array()
            .ok_or_else(|| LabError::DataLoadFailed("JSON root must be an array".to_string()))?;

        if array.is_empty() {
            return Err(LabError::DataLoadFailed("JSON array is empty".to_string()));
        }

        let first_obj = array[0].as_object()
            .ok_or_else(|| LabError::DataLoadFailed("JSON items must be objects".to_string()))?;

        let column_names: Vec<String> = first_obj.keys().cloned().collect();
        let columns = column_names.len();

        let mut rows = 0usize;
        let mut has_missing = false;
        for item in array {
            rows += 1;
            if let Some(max) = config.max_rows {
                if rows >= max {
                    break;
                }
            }
            if !has_missing {
                if let Some(obj) = item.as_object() {
                    for key in &column_names {
                        if !obj.contains_key(key) || obj[key].is_null() {
                            has_missing = true;
                            break;
                        }
                    }
                }
            }
        }

        let column_types: Vec<String> = column_names.iter().map(|key| {
            let mut type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for item in array.iter().take(100) {
                if let Some(obj) = item.as_object() {
                    if let Some(val) = obj.get(key) {
                        let t = if val.is_number() { "number" } else if val.is_string() { "string" } else if val.is_boolean() { "boolean" } else if val.is_null() { "null" } else { "unknown" };
                        *type_counts.entry(t.to_string()).or_insert(0) += 1;
                    }
                }
            }
            type_counts.into_iter().max_by_key(|(_, c)| *c).map(|(t, _)| t).unwrap_or_else(|| "unknown".to_string())
        }).collect();

        Ok(DatasetInfo {
            name: path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            format: DataFormat::Json,
            rows,
            columns,
            column_names,
            column_types,
            has_missing_values: has_missing,
            memory_size_mb: metadata.len() as f64 / (1024.0 * 1024.0),
        })
    }

    async fn preview(&self, config: &DataLoadConfig, offset: usize, limit: usize) -> Result<DataPreview> {
        let content = std::fs::read_to_string(&config.path)
            .map_err(|e| LabError::DataLoadFailed(format!("Cannot read file: {}", e)))?;

        let json_value: Value = serde_json::from_str(&content)
            .map_err(|e| LabError::DataLoadFailed(format!("JSON parse error: {}", e)))?;

        let array = json_value.as_array()
            .ok_or_else(|| LabError::DataLoadFailed("JSON root must be an array".to_string()))?;

        if array.is_empty() {
            return Err(LabError::DataLoadFailed("JSON array is empty".to_string()));
        }

        let first_obj = array[0].as_object()
            .ok_or_else(|| LabError::DataLoadFailed("JSON items must be objects".to_string()))?;

        let columns: Vec<String> = first_obj.keys().cloned().collect();

        let column_types: Vec<String> = columns.iter().map(|key| {
            let mut type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for item in array.iter().take(100) {
                if let Some(obj) = item.as_object() {
                    if let Some(val) = obj.get(key) {
                        let t = if val.is_i64() || val.is_u64() { "integer" } else if val.is_f64() { "float" } else if val.is_string() { "string" } else if val.is_boolean() { "boolean" } else if val.is_null() { "null" } else { "unknown" };
                        *type_counts.entry(t.to_string()).or_insert(0) += 1;
                    }
                }
            }
            type_counts.into_iter()
                .filter(|(t, _)| t != "null")
                .max_by_key(|(_, c)| *c)
                .map(|(t, _)| t)
                .unwrap_or_else(|| "unknown".to_string())
        }).collect();

        let mut rows = Vec::new();
        for item in array.iter().skip(offset).take(limit) {
            if let Some(obj) = item.as_object() {
                let row: Vec<Value> = columns.iter().map(|key| {
                    obj.get(key).cloned().unwrap_or(Value::Null)
                }).collect();
                rows.push(row);
            }
        }

        let total_rows = array.len();

        Ok(DataPreview {
            columns,
            column_types,
            rows,
            total_rows,
            offset,
        })
    }
}
