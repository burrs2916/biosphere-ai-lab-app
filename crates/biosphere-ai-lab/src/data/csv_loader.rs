use async_trait::async_trait;
use serde_json::Value;

use crate::core::{LabError, Result};
use crate::core::config::DataLoadConfig;
use crate::types::{DataFormat, PluginId};
use super::data_trait::{DataPreview, DatasetInfo, DataSource, PreprocessType};

pub struct CsvLoader {
    id: PluginId,
}

impl CsvLoader {
    pub fn new() -> Self {
        Self {
            id: PluginId::new("csv"),
        }
    }
}

impl Default for CsvLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataSource for CsvLoader {
    fn id(&self) -> &PluginId {
        &self.id
    }

    fn name(&self) -> &str {
        "CSV Data Loader"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Load and parse CSV/TSV data files"
    }

    fn data_format(&self) -> DataFormat {
        DataFormat::Csv
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

        let delimiter = config.delimiter.unwrap_or(',');
        let has_header = config.has_header;

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(delimiter as u8)
            .has_headers(has_header)
            .from_reader(content.as_bytes());

        let headers = reader.headers()
            .map_err(|e| LabError::DataLoadFailed(format!("Cannot parse CSV headers: {}", e)))?
            .clone();

        let column_names: Vec<String> = headers.iter().map(|h| h.to_string()).collect();
        let columns = column_names.len();

        let mut rows = 0usize;
        for result in reader.records() {
            let _ = result.map_err(|e| LabError::DataLoadFailed(format!("CSV parse error at row {}: {}", rows + 1, e)))?;
            rows += 1;
            if let Some(max) = config.max_rows {
                if rows >= max {
                    break;
                }
            }
        }

        let column_types = vec!["unknown".to_string(); columns];

        let mut has_missing = false;
        let mut reader2 = csv::ReaderBuilder::new()
            .delimiter(delimiter as u8)
            .has_headers(has_header)
            .from_reader(content.as_bytes());
        for result in reader2.records() {
            if let Ok(record) = result {
                for field in record.iter() {
                    if field.trim().is_empty() {
                        has_missing = true;
                        break;
                    }
                }
                if has_missing {
                    break;
                }
            }
        }

        Ok(DatasetInfo {
            name: path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            format: DataFormat::Csv,
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

        let delimiter = config.delimiter.unwrap_or(',');
        let has_header = config.has_header;

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(delimiter as u8)
            .has_headers(has_header)
            .from_reader(content.as_bytes());

        let headers = reader.headers()
            .map_err(|e| LabError::DataLoadFailed(format!("Cannot parse CSV headers: {}", e)))?
            .clone();

        let columns: Vec<String> = headers.iter().map(|h| h.to_string()).collect();

        let all_records: Vec<csv::StringRecord> = reader.records()
            .filter_map(|r| r.ok())
            .collect();

        let total_rows = all_records.len();

        let sliced: Vec<&csv::StringRecord> = all_records.iter()
            .skip(offset)
            .take(limit)
            .collect();

        let mut column_types: Vec<String> = vec!["unknown".to_string(); columns.len()];
        {
            let sample: Vec<&csv::StringRecord> = all_records.iter().take(100).collect();
            for (col_idx, _) in columns.iter().enumerate() {
                let mut int_count = 0usize;
                let mut float_count = 0usize;
                let mut bool_count = 0usize;
                let mut null_count = 0usize;
                let mut total_counted = 0usize;
                for record in &sample {
                    if let Some(field) = record.get(col_idx) {
                        let trimmed = field.trim();
                        if trimmed.is_empty() {
                            null_count += 1;
                        } else if trimmed == "true" || trimmed == "false" || trimmed == "TRUE" || trimmed == "FALSE" {
                            bool_count += 1;
                        } else if trimmed.parse::<i64>().is_ok() {
                            int_count += 1;
                        } else if trimmed.parse::<f64>().is_ok() {
                            float_count += 1;
                        }
                        total_counted += 1;
                    }
                }
                let non_null = total_counted - null_count;
                if non_null == 0 {
                    column_types[col_idx] = "unknown".to_string();
                } else if int_count as f64 / non_null as f64 > 0.8 {
                    column_types[col_idx] = "integer".to_string();
                } else if (int_count + float_count) as f64 / non_null as f64 > 0.8 {
                    column_types[col_idx] = "float".to_string();
                } else if bool_count as f64 / non_null as f64 > 0.8 {
                    column_types[col_idx] = "boolean".to_string();
                } else {
                    let distinct: std::collections::HashSet<&str> = sample.iter()
                        .filter_map(|r| r.get(col_idx))
                        .map(|f| f.trim())
                        .filter(|f| !f.is_empty())
                        .collect();
                    if distinct.len() <= 10 && distinct.len() < non_null / 2 {
                        column_types[col_idx] = "categorical".to_string();
                    } else {
                        column_types[col_idx] = "string".to_string();
                    }
                }
            }
        }

        let rows: Vec<Vec<Value>> = sliced.iter().map(|record| {
            record.iter().enumerate().map(|(_, field)| {
                let trimmed = field.trim();
                if trimmed.is_empty() {
                    Value::Null
                } else if let Ok(n) = trimmed.parse::<i64>() {
                    Value::from(n)
                } else if let Ok(f) = trimmed.parse::<f64>() {
                    Value::from(f)
                } else if trimmed == "true" || trimmed == "TRUE" {
                    Value::from(true)
                } else if trimmed == "false" || trimmed == "FALSE" {
                    Value::from(false)
                } else {
                    Value::from(trimmed)
                }
            }).collect()
        }).collect();

        Ok(DataPreview {
            columns,
            column_types,
            rows,
            total_rows,
            offset,
        })
    }
}
