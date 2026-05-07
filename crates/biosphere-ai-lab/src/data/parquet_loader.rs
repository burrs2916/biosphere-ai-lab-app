use async_trait::async_trait;
use serde_json::Value;

use crate::core::{LabError, Result};
use crate::core::config::DataLoadConfig;
use crate::types::{DataFormat, PluginId};
use super::data_trait::{DataPreview, DatasetInfo, DataSource, PreprocessType};

pub struct ParquetLoader {
    id: PluginId,
}

impl ParquetLoader {
    pub fn new() -> Self {
        Self {
            id: PluginId::new("parquet"),
        }
    }
}

impl Default for ParquetLoader {
    fn default() -> Self {
        Self::new()
    }
}

fn arrow_type_to_string(data_type: &arrow::datatypes::DataType) -> String {
    use arrow::datatypes::DataType;
    match data_type {
        DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 |
        DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64 => "integer".to_string(),
        DataType::Float16 | DataType::Float32 | DataType::Float64 => "float".to_string(),
        DataType::Boolean => "boolean".to_string(),
        DataType::Utf8 | DataType::LargeUtf8 => "string".to_string(),
        DataType::Date32 | DataType::Date64 | DataType::Timestamp(_, _) => "datetime".to_string(),
        DataType::List(_) | DataType::LargeList(_) => "list".to_string(),
        _ => "unknown".to_string(),
    }
}

fn arrow_value_to_json(row_idx: usize, array: &dyn arrow::array::Array) -> Value {
    use arrow::array::*;
    if row_idx >= array.len() || array.is_null(row_idx) {
        return Value::Null;
    }
    if let Some(arr) = array.as_any().downcast_ref::<Int32Array>() {
        Value::from(arr.value(row_idx))
    } else if let Some(arr) = array.as_any().downcast_ref::<Int64Array>() {
        Value::from(arr.value(row_idx))
    } else if let Some(arr) = array.as_any().downcast_ref::<UInt32Array>() {
        Value::from(arr.value(row_idx) as i64)
    } else if let Some(arr) = array.as_any().downcast_ref::<UInt64Array>() {
        Value::from(arr.value(row_idx) as i64)
    } else if let Some(arr) = array.as_any().downcast_ref::<Float32Array>() {
        Value::from(arr.value(row_idx) as f64)
    } else if let Some(arr) = array.as_any().downcast_ref::<Float64Array>() {
        Value::from(arr.value(row_idx))
    } else if let Some(arr) = array.as_any().downcast_ref::<BooleanArray>() {
        Value::from(arr.value(row_idx))
    } else if let Some(arr) = array.as_any().downcast_ref::<StringArray>() {
        Value::from(arr.value(row_idx))
    } else if let Some(arr) = array.as_any().downcast_ref::<LargeStringArray>() {
        Value::from(arr.value(row_idx))
    } else if let Some(arr) = array.as_any().downcast_ref::<Int8Array>() {
        Value::from(arr.value(row_idx) as i64)
    } else if let Some(arr) = array.as_any().downcast_ref::<Int16Array>() {
        Value::from(arr.value(row_idx) as i64)
    } else if let Some(arr) = array.as_any().downcast_ref::<UInt8Array>() {
        Value::from(arr.value(row_idx) as i64)
    } else if let Some(arr) = array.as_any().downcast_ref::<UInt16Array>() {
        Value::from(arr.value(row_idx) as i64)
    } else {
        Value::Null
    }
}

#[async_trait]
impl DataSource for ParquetLoader {
    fn id(&self) -> &PluginId {
        &self.id
    }

    fn name(&self) -> &str {
        "Parquet Data Loader"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Load and parse Apache Parquet data files"
    }

    fn data_format(&self) -> DataFormat {
        DataFormat::Parquet
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

        let file = std::fs::File::open(&config.path)
            .map_err(|e| LabError::DataLoadFailed(format!("Cannot open parquet file: {}", e)))?;

        let builder = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(file)
            .map_err(|e| LabError::DataLoadFailed(format!("Cannot create parquet reader: {}", e)))?;

        let parquet_metadata = builder.metadata();
        let file_metadata = parquet_metadata.file_metadata();
        let rows = file_metadata.num_rows() as usize;

        let arrow_schema = builder.schema();
        let column_names: Vec<String> = arrow_schema.fields().iter()
            .map(|f: &std::sync::Arc<arrow::datatypes::Field>| f.name().clone())
            .collect();
        let column_types: Vec<String> = arrow_schema.fields().iter()
            .map(|f: &std::sync::Arc<arrow::datatypes::Field>| arrow_type_to_string(f.data_type()))
            .collect();
        let columns = column_names.len();

        let file2 = std::fs::File::open(&config.path)
            .map_err(|e| LabError::DataLoadFailed(format!("Cannot open parquet file: {}", e)))?;
        let arrow_reader = parquet::arrow::arrow_reader::ParquetRecordBatchReader::try_new(file2, 1024)
            .map_err(|e| LabError::DataLoadFailed(format!("Cannot create arrow reader: {}", e)))?;

        let mut has_missing = false;
        for batch_result in arrow_reader {
            if has_missing { break; }
            if let Ok(batch) = batch_result {
                for col in 0..batch.num_columns() {
                    let array = batch.column(col);
                    if array.null_count() > 0 {
                        has_missing = true;
                        break;
                    }
                }
            }
        }

        Ok(DatasetInfo {
            name: path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            format: DataFormat::Parquet,
            rows,
            columns,
            column_names,
            column_types,
            has_missing_values: has_missing,
            memory_size_mb: metadata.len() as f64 / (1024.0 * 1024.0),
        })
    }

    async fn preview(&self, config: &DataLoadConfig, offset: usize, limit: usize) -> Result<DataPreview> {
        let file = std::fs::File::open(&config.path)
            .map_err(|e| LabError::DataLoadFailed(format!("Cannot open parquet file: {}", e)))?;

        let builder = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(file)
            .map_err(|e| LabError::DataLoadFailed(format!("Cannot create parquet reader: {}", e)))?;

        let total_rows = builder.metadata().file_metadata().num_rows() as usize;

        let arrow_schema = builder.schema();
        let column_names: Vec<String> = arrow_schema.fields().iter()
            .map(|f: &std::sync::Arc<arrow::datatypes::Field>| f.name().clone())
            .collect();
        let column_types: Vec<String> = arrow_schema.fields().iter()
            .map(|f: &std::sync::Arc<arrow::datatypes::Field>| arrow_type_to_string(f.data_type()))
            .collect();

        let file2 = std::fs::File::open(&config.path)
            .map_err(|e| LabError::DataLoadFailed(format!("Cannot open parquet file: {}", e)))?;

        let arrow_reader = parquet::arrow::arrow_reader::ParquetRecordBatchReader::try_new(file2, 1024)
            .map_err(|e| LabError::DataLoadFailed(format!("Cannot build parquet reader: {}", e)))?;

        let mut all_rows: Vec<Vec<Value>> = Vec::new();
        let mut current_offset = 0usize;

        for batch_result in arrow_reader {
            let batch = batch_result
                .map_err(|e| LabError::DataLoadFailed(format!("Cannot read batch: {}", e)))?;
            let batch_rows = batch.num_rows();

            if current_offset + batch_rows <= offset {
                current_offset += batch_rows;
                continue;
            }

            let start_in_batch = if current_offset < offset { offset - current_offset } else { 0 };
            let remaining = limit - all_rows.len();
            let end_in_batch = std::cmp::min(start_in_batch + remaining, batch_rows);

            for row_idx in start_in_batch..end_in_batch {
                let row: Vec<Value> = (0..batch.num_columns())
                    .map(|col_idx| arrow_value_to_json(row_idx, batch.column(col_idx)))
                    .collect();
                all_rows.push(row);
            }

            current_offset += batch_rows;

            if all_rows.len() >= limit {
                break;
            }
        }

        Ok(DataPreview {
            columns: column_names,
            column_types,
            rows: all_rows,
            total_rows,
            offset,
        })
    }
}
