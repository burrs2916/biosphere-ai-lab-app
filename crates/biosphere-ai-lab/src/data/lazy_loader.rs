use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LazyLoadConfig {
    pub chunk_size: usize,
    pub max_memory_mb: f64,
    pub use_memory_map: bool,
    pub prefetch_chunks: usize,
    pub parallel_load: bool,
}

impl Default for LazyLoadConfig {
    fn default() -> Self {
        Self {
            chunk_size: 10000,
            max_memory_mb: 512.0,
            use_memory_map: true,
            prefetch_chunks: 2,
            parallel_load: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataChunk {
    pub chunk_index: usize,
    pub start_row: usize,
    pub end_row: usize,
    pub columns: Vec<String>,
    pub column_types: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub is_last: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LazyDatasetInfo {
    pub path: String,
    pub format: String,
    pub total_rows: usize,
    pub total_columns: usize,
    pub column_names: Vec<String>,
    pub column_types: Vec<String>,
    pub file_size_mb: f64,
    pub estimated_chunks: usize,
    pub chunk_size: usize,
    pub supports_memory_map: bool,
    pub supports_streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamProgress {
    pub chunks_read: usize,
    pub total_chunks: usize,
    pub rows_read: usize,
    pub total_rows: usize,
    pub progress_pct: f64,
    pub memory_used_mb: f64,
    pub is_complete: bool,
}

pub struct LazyDataLoader;

impl LazyDataLoader {
    pub fn inspect(path: &str, config: &LazyLoadConfig) -> Result<LazyDatasetInfo, String> {
        let path = Path::new(path);
        if !path.exists() {
            return Err(format!("File not found: {}", path.display()));
        }

        let metadata = std::fs::metadata(path)
            .map_err(|e| format!("Cannot read file metadata: {}", e))?;
        let file_size_mb = metadata.len() as f64 / (1024.0 * 1024.0);

        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let (format, supports_mm, supports_stream) = match extension.as_str() {
            "csv" => ("csv", true, true),
            "parquet" => ("parquet", true, true),
            "json" | "jsonl" => ("json", false, true),
            _ => ("unknown", false, false),
        };

        let (column_names, column_types, total_rows) = match format {
            "csv" => Self::inspect_csv(path)?,
            "parquet" => Self::inspect_parquet(path)?,
            _ => (Vec::new(), Vec::new(), 0),
        };

        let total_columns = column_names.len();
        let estimated_chunks = if total_rows > 0 {
            (total_rows + config.chunk_size - 1) / config.chunk_size
        } else {
            0
        };

        Ok(LazyDatasetInfo {
            path: path.to_string_lossy().to_string(),
            format: format.to_string(),
            total_rows,
            total_columns,
            column_names,
            column_types,
            file_size_mb,
            estimated_chunks,
            chunk_size: config.chunk_size,
            supports_memory_map: supports_mm,
            supports_streaming: supports_stream,
        })
    }

    fn inspect_csv(path: &Path) -> Result<(Vec<String>, Vec<String>, usize), String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("Cannot open CSV: {}", e))?;
        let reader = BufReader::new(file);

        let mut lines_iter = reader.lines();

        let header_line = lines_iter.next()
            .ok_or_else(|| "CSV file is empty".to_string())?
            .map_err(|e| format!("Cannot read CSV header: {}", e))?;

        let column_names: Vec<String> = header_line.trim()
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .collect();

        let mut sample_values: Vec<Vec<String>> = vec![Vec::new(); column_names.len()];
        let mut total_rows = 0;

        for line in lines_iter.by_ref().take(1000) {
            let line = line.map_err(|e| format!("CSV read error: {}", e))?;
            total_rows += 1;
            let fields: Vec<&str> = line.split(',').collect();
            for (i, field) in fields.iter().enumerate() {
                if i < sample_values.len() {
                    sample_values[i].push(field.trim().trim_matches('"').to_string());
                }
            }
        }

        let remaining = lines_iter.count();
        total_rows += remaining;

        let column_types: Vec<String> = sample_values.iter()
            .map(|vals| Self::infer_type(vals))
            .collect();

        Ok((column_names, column_types, total_rows))
    }

    fn inspect_parquet(path: &Path) -> Result<(Vec<String>, Vec<String>, usize), String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("Cannot open Parquet: {}", e))?;

        use parquet::file::reader::FileReader;
        let reader = parquet::file::reader::SerializedFileReader::new(file)
            .map_err(|e| format!("Parquet reader error: {}", e))?;

        let metadata = reader.metadata();
        let schema = metadata.file_metadata().schema_descr();

        let column_names: Vec<String> = schema.columns()
            .iter()
            .map(|c| c.name().to_string())
            .collect();

        let column_types: Vec<String> = schema.columns()
            .iter()
            .map(|c| format!("{:?}", c.physical_type()).to_lowercase())
            .collect();

        let total_rows = metadata.file_metadata().num_rows() as usize;

        Ok((column_names, column_types, total_rows))
    }

    fn infer_type(values: &[String]) -> String {
        if values.is_empty() {
            return "string".to_string();
        }

        let non_empty: Vec<&str> = values.iter()
            .map(|s| s.as_str())
            .filter(|s| !s.is_empty())
            .collect();

        if non_empty.is_empty() {
            return "string".to_string();
        }

        let int_count = non_empty.iter()
            .filter(|s| s.parse::<i64>().is_ok())
            .count();
        let float_count = non_empty.iter()
            .filter(|s| s.parse::<f64>().is_ok())
            .count();
        let bool_count = non_empty.iter()
            .filter(|s| {
                let val = **s;
                val == "true" || val == "false" || val == "TRUE" || val == "FALSE"
            })
            .count();

        let total = non_empty.len() as f64;
        if int_count as f64 / total > 0.8 {
            "integer".to_string()
        } else if float_count as f64 / total > 0.8 {
            "float".to_string()
        } else if bool_count as f64 / total > 0.8 {
            "boolean".to_string()
        } else {
            let distinct: std::collections::HashSet<&&str> = non_empty.iter().collect();
            if distinct.len() <= 20 && distinct.len() < non_empty.len() / 2 {
                "categorical".to_string()
            } else {
                "string".to_string()
            }
        }
    }

    pub fn read_chunk_csv(
        path: &str,
        chunk_index: usize,
        config: &LazyLoadConfig,
    ) -> Result<DataChunk, String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("Cannot open CSV: {}", e))?;
        let mut reader = BufReader::new(file);

        let mut header_line = String::new();
        reader.read_line(&mut header_line)
            .map_err(|e| format!("Cannot read CSV header: {}", e))?;

        let columns: Vec<String> = header_line.trim()
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .collect();

        let start_row = chunk_index * config.chunk_size;
        let mut rows = Vec::new();
        let mut current_row = 0;
        let mut end_row = start_row;

        for line in reader.lines() {
            let line = line.map_err(|e| format!("CSV read error: {}", e))?;

            if current_row >= start_row && rows.len() < config.chunk_size {
                let fields: Vec<serde_json::Value> = line.split(',')
                    .map(|f| {
                        let trimmed = f.trim().trim_matches('"');
                        if trimmed.is_empty() {
                            serde_json::Value::Null
                        } else if let Ok(n) = trimmed.parse::<i64>() {
                            serde_json::Value::from(n)
                        } else if let Ok(n) = trimmed.parse::<f64>() {
                            serde_json::json!(n)
                        } else {
                            serde_json::Value::String(trimmed.to_string())
                        }
                    })
                    .collect();
                rows.push(fields);
                end_row = current_row + 1;
            }

            current_row += 1;
            if rows.len() >= config.chunk_size {
                break;
            }
        }

        let is_last = rows.len() < config.chunk_size || current_row <= end_row;

        let column_types: Vec<String> = (0..columns.len())
            .map(|ci| {
                let vals: Vec<String> = rows.iter()
                    .filter_map(|r| r.get(ci))
                    .filter_map(|v| match v {
                        serde_json::Value::String(s) => Some(s.clone()),
                        serde_json::Value::Null => None,
                        other => Some(other.to_string()),
                    })
                    .collect();
                Self::infer_type(&vals)
            })
            .collect();

        Ok(DataChunk {
            chunk_index,
            start_row,
            end_row,
            columns,
            column_types,
            rows,
            is_last,
        })
    }

    pub fn read_chunk_parquet(
        path: &str,
        chunk_index: usize,
        config: &LazyLoadConfig,
    ) -> Result<DataChunk, String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("Cannot open Parquet: {}", e))?;

        use parquet::file::reader::FileReader;
        let reader = parquet::file::reader::SerializedFileReader::new(file)
            .map_err(|e| format!("Parquet reader error: {}", e))?;

        let metadata = reader.metadata();
        let schema = metadata.file_metadata().schema_descr();
        let total_rows = metadata.file_metadata().num_rows() as usize;

        let columns: Vec<String> = schema.columns()
            .iter()
            .map(|c| c.name().to_string())
            .collect();

        let column_types: Vec<String> = schema.columns()
            .iter()
            .map(|c| format!("{:?}", c.physical_type()).to_lowercase())
            .collect();

        let start_row = chunk_index * config.chunk_size;
        let end_row = (start_row + config.chunk_size).min(total_rows);

        let mut rows = Vec::new();

        let row_iter = reader.get_row_iter(None)
            .map_err(|e| format!("Parquet row iterator error: {}", e))?;

        let mut current_row = 0;
        for record in row_iter {
            let record = record.map_err(|e| format!("Parquet record error: {}", e))?;

            if current_row >= start_row && current_row < end_row {
                let row: Vec<serde_json::Value> = (0..columns.len())
                    .map(|ci| {
                        use parquet::record::RowAccessor;
                        match record.get_string(ci) {
                            Ok(s) => {
                                let trimmed = s.trim();
                                if trimmed.is_empty() {
                                    serde_json::Value::Null
                                } else if let Ok(n) = trimmed.parse::<i64>() {
                                    serde_json::Value::from(n)
                                } else if let Ok(n) = trimmed.parse::<f64>() {
                                    serde_json::json!(n)
                                } else {
                                    serde_json::Value::String(trimmed.to_string())
                                }
                            }
                            Err(_) => serde_json::Value::Null,
                        }
                    })
                    .collect();
                rows.push(row);
            }

            current_row += 1;
            if current_row >= end_row {
                break;
            }
        }

        let is_last = end_row >= total_rows;

        Ok(DataChunk {
            chunk_index,
            start_row,
            end_row,
            columns,
            column_types,
            rows,
            is_last,
        })
    }

    pub fn estimate_memory(chunk: &DataChunk) -> f64 {
        let row_count = chunk.rows.len();
        let col_count = chunk.columns.len();
        let avg_bytes_per_cell = 32.0;
        (row_count as f64 * col_count as f64 * avg_bytes_per_cell) / (1024.0 * 1024.0)
    }

    pub fn recommend_chunk_size(file_size_mb: f64, max_memory_mb: f64) -> usize {
        let safe_memory = max_memory_mb * 0.3;
        let rows_per_mb = 5000.0;
        let estimated_rows = file_size_mb * rows_per_mb;
        let chunks_for_memory = (safe_memory * rows_per_mb) as usize;
        let min_chunk = 1000;
        let max_chunk = 100000;

        chunks_for_memory.clamp(min_chunk, max_chunk).min(estimated_rows as usize)
    }
}
