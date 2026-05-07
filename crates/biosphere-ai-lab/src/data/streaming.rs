use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use crate::data::arrow_table::{ArrowTable, ArrowTableBuilder, infer_arrow_schema};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    pub chunk_size: usize,
    pub max_buffer_chunks: usize,
    pub prefetch_chunks: usize,
    pub drop_last: bool,
    pub shuffle_buffer_size: usize,
    pub seed: Option<u64>,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            chunk_size: 10000,
            max_buffer_chunks: 4,
            prefetch_chunks: 2,
            drop_last: false,
            shuffle_buffer_size: 0,
            seed: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingDatasetInfo {
    pub name: String,
    pub format: String,
    pub path: String,
    pub total_rows: Option<usize>,
    pub column_names: Vec<String>,
    pub column_types: Vec<String>,
    pub file_size_bytes: u64,
    pub estimated_chunks: Option<usize>,
    pub chunk_size: usize,
    pub supports_seek: bool,
}

#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub chunk_index: usize,
    pub start_row: usize,
    pub end_row: usize,
    pub data: ArrowTable,
    pub is_last: bool,
}

pub struct StreamingDataset {
    pub info: StreamingDatasetInfo,
    config: StreamingConfig,
    reader: Arc<Mutex<Box<dyn BufRead + Send>>>,
    current_position: Arc<Mutex<usize>>,
    current_chunk: Arc<Mutex<usize>>,
    exhausted: Arc<Mutex<bool>>,
}

impl StreamingDataset {
    pub fn open_csv(
        name: &str,
        path: &str,
        config: StreamingConfig,
    ) -> Result<Self, String> {
        let file = File::open(path)
            .map_err(|e| format!("Failed to open file: {}", e))?;
        let metadata = file.metadata()
            .map_err(|e| format!("Failed to read metadata: {}", e))?;
        let file_size = metadata.len();

        let mut buf_reader = BufReader::new(file);

        let mut header_line = String::new();
        buf_reader.read_line(&mut header_line)
            .map_err(|e| format!("Failed to read header: {}", e))?;
        let headers: Vec<String> = header_line.trim()
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .collect();

        let column_types: Vec<String> = headers.iter()
            .map(|_| "string".to_string())
            .collect();

        let info = StreamingDatasetInfo {
            name: name.to_string(),
            format: "csv".to_string(),
            path: path.to_string(),
            total_rows: None,
            column_names: headers,
            column_types,
            file_size_bytes: file_size,
            estimated_chunks: None,
            chunk_size: config.chunk_size,
            supports_seek: true,
        };

        Ok(Self {
            info,
            config,
            reader: Arc::new(Mutex::new(Box::new(buf_reader))),
            current_position: Arc::new(Mutex::new(0)),
            current_chunk: Arc::new(Mutex::new(0)),
            exhausted: Arc::new(Mutex::new(false)),
        })
    }

    pub fn open_jsonl(
        name: &str,
        path: &str,
        config: StreamingConfig,
    ) -> Result<Self, String> {
        let file = File::open(path)
            .map_err(|e| format!("Failed to open file: {}", e))?;
        let metadata = file.metadata()
            .map_err(|e| format!("Failed to read metadata: {}", e))?;
        let file_size = metadata.len();

        let mut buf_reader = BufReader::new(file);

        let mut first_line = String::new();
        buf_reader.read_line(&mut first_line)
            .map_err(|e| format!("Failed to read first line: {}", e))?;

        let column_names: Vec<String> = if let Ok(obj) = serde_json::from_str::<serde_json::Value>(first_line.trim()) {
            if let Some(map) = obj.as_object() {
                map.keys().cloned().collect()
            } else {
                vec!["value".to_string()]
            }
        } else {
            vec!["value".to_string()]
        };

        let column_types: Vec<String> = column_names.iter()
            .map(|_| "string".to_string())
            .collect();

        buf_reader.seek(SeekFrom::Start(0))
            .map_err(|e| format!("Seek failed: {}", e))?;

        let info = StreamingDatasetInfo {
            name: name.to_string(),
            format: "jsonl".to_string(),
            path: path.to_string(),
            total_rows: None,
            column_names,
            column_types,
            file_size_bytes: file_size,
            estimated_chunks: None,
            chunk_size: config.chunk_size,
            supports_seek: true,
        };

        Ok(Self {
            info,
            config,
            reader: Arc::new(Mutex::new(Box::new(buf_reader))),
            current_position: Arc::new(Mutex::new(0)),
            current_chunk: Arc::new(Mutex::new(0)),
            exhausted: Arc::new(Mutex::new(false)),
        })
    }

    pub fn open_text_lines(
        name: &str,
        path: &str,
        config: StreamingConfig,
    ) -> Result<Self, String> {
        let file = File::open(path)
            .map_err(|e| format!("Failed to open file: {}", e))?;
        let metadata = file.metadata()
            .map_err(|e| format!("Failed to read metadata: {}", e))?;
        let file_size = metadata.len();

        let info = StreamingDatasetInfo {
            name: name.to_string(),
            format: "text".to_string(),
            path: path.to_string(),
            total_rows: None,
            column_names: vec!["text".to_string()],
            column_types: vec!["string".to_string()],
            file_size_bytes: file_size,
            estimated_chunks: None,
            chunk_size: config.chunk_size,
            supports_seek: true,
        };

        Ok(Self {
            info,
            config,
            reader: Arc::new(Mutex::new(Box::new(BufReader::new(file)))),
            current_position: Arc::new(Mutex::new(0)),
            current_chunk: Arc::new(Mutex::new(0)),
            exhausted: Arc::new(Mutex::new(false)),
        })
    }

    pub fn next_chunk(&self) -> Result<Option<StreamChunk>, String> {
        let mut exhausted = self.exhausted.lock().unwrap();
        if *exhausted {
            return Ok(None);
        }

        let mut reader = self.reader.lock().unwrap();
        let mut chunk_idx = self.current_chunk.lock().unwrap();
        let mut position = self.current_position.lock().unwrap();

        let mut rows: Vec<Vec<String>> = Vec::with_capacity(self.config.chunk_size);
        let start_row = *position;

        let mut line = String::new();
        for _ in 0..self.config.chunk_size {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    *exhausted = true;
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match self.info.format.as_str() {
                        "csv" => {
                            let fields: Vec<String> = trimmed.split(',')
                                .map(|s| s.trim().trim_matches('"').to_string())
                                .collect();
                            rows.push(fields);
                        }
                        "jsonl" => {
                            rows.push(vec![trimmed.to_string()]);
                        }
                        "text" => {
                            rows.push(vec![trimmed.to_string()]);
                        }
                        _ => {
                            rows.push(vec![trimmed.to_string()]);
                        }
                    }
                    *position += 1;
                }
                Err(e) => {
                    return Err(format!("Read error: {}", e));
                }
            }
        }

        if rows.is_empty() {
            *exhausted = true;
            return Ok(None);
        }

        let is_last = *exhausted;
        let end_row = *position;
        let current = *chunk_idx;
        *chunk_idx += 1;

        drop(reader);
        drop(position);
        drop(chunk_idx);
        drop(exhausted);

        let table = self.build_arrow_table(&rows)?;

        Ok(Some(StreamChunk {
            chunk_index: current,
            start_row,
            end_row,
            data: table,
            is_last,
        }))
    }

    fn build_arrow_table(&self, rows: &[Vec<String>]) -> Result<ArrowTable, String> {
        let schema = infer_arrow_schema(&self.info.column_names, &[]);
        let mut builder = ArrowTableBuilder::new(&self.info.name, schema.clone());

        for row in rows {
            for (col_idx, field) in schema.fields().iter().enumerate() {
                let val = row.get(col_idx).map(|s| s.as_str()).unwrap_or("");
                if val.is_empty() {
                    builder.push_null(field.name());
                    continue;
                }
                match field.data_type() {
                    arrow::datatypes::DataType::Int64 => {
                        if let Ok(v) = val.parse::<i64>() {
                            builder.push_int(field.name(), v);
                        } else {
                            builder.push_null(field.name());
                        }
                    }
                    arrow::datatypes::DataType::Float64 => {
                        if let Ok(v) = val.parse::<f64>() {
                            builder.push_float(field.name(), v);
                        } else {
                            builder.push_null(field.name());
                        }
                    }
                    arrow::datatypes::DataType::Boolean => {
                        match val.to_lowercase().as_str() {
                            "true" | "1" => builder.push_bool(field.name(), true),
                            "false" | "0" => builder.push_bool(field.name(), false),
                            _ => builder.push_null(field.name()),
                        }
                    }
                    _ => {
                        builder.push_string(field.name(), val);
                    }
                }
            }
        }

        builder.build()
    }

    pub fn reset(&self) -> Result<(), String> {
        let mut position = self.current_position.lock().unwrap();
        let mut chunk_idx = self.current_chunk.lock().unwrap();
        let mut exhausted = self.exhausted.lock().unwrap();

        if self.info.supports_seek {
            let file = File::open(&self.info.path)
                .map_err(|e| format!("Failed to reopen file: {}", e))?;
            let buf_reader = BufReader::new(file);
            let mut reader = self.reader.lock().unwrap();
            *reader = Box::new(buf_reader);
        }

        *position = 0;
        *chunk_idx = 0;
        *exhausted = false;

        Ok(())
    }

    pub fn is_exhausted(&self) -> bool {
        *self.exhausted.lock().unwrap()
    }

    pub fn current_chunk_index(&self) -> usize {
        *self.current_chunk.lock().unwrap()
    }

    pub fn current_position(&self) -> usize {
        *self.current_position.lock().unwrap()
    }
}

pub struct StreamingDatasetBuilder {
    name: String,
    path: String,
    format: String,
    config: StreamingConfig,
}

impl StreamingDatasetBuilder {
    pub fn new(name: &str, path: &str) -> Self {
        let format = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("csv")
            .to_lowercase();

        Self {
            name: name.to_string(),
            path: path.to_string(),
            format,
            config: StreamingConfig::default(),
        }
    }

    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.config.chunk_size = size;
        self
    }

    pub fn with_shuffle(mut self, buffer_size: usize, seed: Option<u64>) -> Self {
        self.config.shuffle_buffer_size = buffer_size;
        self.config.seed = seed;
        self
    }

    pub fn build(self) -> Result<StreamingDataset, String> {
        match self.format.as_str() {
            "csv" | "tsv" => StreamingDataset::open_csv(&self.name, &self.path, self.config),
            "jsonl" | "json" => StreamingDataset::open_jsonl(&self.name, &self.path, self.config),
            "txt" => StreamingDataset::open_text_lines(&self.name, &self.path, self.config),
            _ => StreamingDataset::open_text_lines(&self.name, &self.path, self.config),
        }
    }
}

pub fn auto_recommend_chunk_size(_file_size_bytes: u64, target_memory_mb: f64) -> usize {
    let target_bytes = (target_memory_mb * 1024.0 * 1024.0) as u64;
    let estimated_row_size = 512u64;
    let rows_per_chunk = (target_bytes / estimated_row_size).max(1000).min(100000);
    rows_per_chunk as usize
}
