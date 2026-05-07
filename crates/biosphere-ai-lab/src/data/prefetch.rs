use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Instant;

use crate::data::arrow_table::ArrowTable;
use crate::data::streaming::{StreamChunk, StreamingConfig, StreamingDataset, StreamingDatasetInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionFormat {
    None,
    Gzip,
    Zstd,
}

impl CompressionFormat {
    pub fn from_extension(path: &str) -> Self {
        let lower = path.to_lowercase();
        if lower.ends_with(".gz") {
            CompressionFormat::Gzip
        } else if lower.ends_with(".zst") || lower.ends_with(".zstd") {
            CompressionFormat::Zstd
        } else {
            CompressionFormat::None
        }
    }

    pub fn decompress_reader(&self, file: File) -> Result<Box<dyn BufRead + Send>, String> {
        match self {
            CompressionFormat::None => Ok(Box::new(BufReader::new(file))),
            CompressionFormat::Gzip => {
                let decoder = flate2::read::GzDecoder::new(file);
                Ok(Box::new(BufReader::new(decoder)))
            }
            CompressionFormat::Zstd => {
                let decoder = zstd::stream::read::Decoder::new(file)
                    .map_err(|e| format!("Zstd decoder error: {}", e))?;
                Ok(Box::new(BufReader::new(decoder)))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefetchConfig {
    pub buffer_size: usize,
    pub num_prefetch_threads: usize,
    pub compression: CompressionFormat,
    pub use_mmap: bool,
    pub prefetch_ahead: usize,
    pub timeout_ms: u64,
}

impl Default for PrefetchConfig {
    fn default() -> Self {
        Self {
            buffer_size: 4,
            num_prefetch_threads: 2,
            compression: CompressionFormat::None,
            use_mmap: false,
            prefetch_ahead: 2,
            timeout_ms: 30000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefetchStats {
    pub total_chunks_produced: usize,
    pub total_chunks_consumed: usize,
    pub buffer_high_watermark: usize,
    pub buffer_low_watermark: usize,
    pub avg_produce_time_ms: f64,
    pub avg_consume_wait_ms: f64,
    pub prefetch_hits: usize,
    pub prefetch_misses: usize,
    pub is_active: bool,
}

impl Default for PrefetchStats {
    fn default() -> Self {
        Self {
            total_chunks_produced: 0,
            total_chunks_consumed: 0,
            buffer_high_watermark: 0,
            buffer_low_watermark: 0,
            avg_produce_time_ms: 0.0,
            avg_consume_wait_ms: 0.0,
            prefetch_hits: 0,
            prefetch_misses: 0,
            is_active: false,
        }
    }
}

struct PrefetchBuffer {
    chunks: VecDeque<StreamChunk>,
    max_size: usize,
    finished: bool,
    error: Option<String>,
}

impl PrefetchBuffer {
    fn new(max_size: usize) -> Self {
        Self {
            chunks: VecDeque::with_capacity(max_size),
            max_size,
            finished: false,
            error: None,
        }
    }

    fn is_full(&self) -> bool {
        self.chunks.len() >= self.max_size
    }

    fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }
}

pub struct PrefetchStreamingDataset {
    pub info: StreamingDatasetInfo,
    config: PrefetchConfig,
    streaming_config: StreamingConfig,
    buffer: Arc<(Mutex<PrefetchBuffer>, Condvar)>,
    producer_active: Arc<AtomicBool>,
    chunks_produced: Arc<AtomicUsize>,
    chunks_consumed: Arc<AtomicUsize>,
    produce_times: Arc<Mutex<Vec<f64>>>,
    consume_waits: Arc<Mutex<Vec<f64>>>,
    buffer_watermarks: Arc<Mutex<Vec<usize>>>,
    prefetch_hits: Arc<AtomicUsize>,
    prefetch_misses: Arc<AtomicUsize>,
    _producer_handle: Option<thread::JoinHandle<()>>,
}

impl PrefetchStreamingDataset {
    pub fn open_csv(
        name: &str,
        path: &str,
        streaming_config: StreamingConfig,
        prefetch_config: PrefetchConfig,
    ) -> Result<Self, String> {
        let compression = if matches!(prefetch_config.compression, CompressionFormat::None) {
            CompressionFormat::from_extension(path)
        } else {
            prefetch_config.compression.clone()
        };

        let file = File::open(path)
            .map_err(|e| format!("Failed to open file: {}", e))?;
        let metadata = file.metadata()
            .map_err(|e| format!("Failed to read metadata: {}", e))?;
        let file_size = metadata.len();

        let reader: Box<dyn BufRead + Send> = compression.decompress_reader(file)?;

        let mut buf_reader = BufReader::new(reader);
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
            chunk_size: streaming_config.chunk_size,
            supports_seek: false,
        };

        let buffer = Arc::new((
            Mutex::new(PrefetchBuffer::new(prefetch_config.buffer_size)),
            Condvar::new(),
        ));
        let producer_active = Arc::new(AtomicBool::new(true));
        let chunks_produced = Arc::new(AtomicUsize::new(0));
        let chunks_consumed = Arc::new(AtomicUsize::new(0));
        let produce_times = Arc::new(Mutex::new(Vec::new()));
        let consume_waits = Arc::new(Mutex::new(Vec::new()));
        let buffer_watermarks = Arc::new(Mutex::new(Vec::new()));
        let prefetch_hits = Arc::new(AtomicUsize::new(0));
        let prefetch_misses = Arc::new(AtomicUsize::new(0));

        let buffer_clone = buffer.clone();
        let producer_active_clone = producer_active.clone();
        let chunks_produced_clone = chunks_produced.clone();
        let produce_times_clone = produce_times.clone();
        let buffer_watermarks_clone = buffer_watermarks.clone();
        let chunk_size = streaming_config.chunk_size;
        let format = "csv".to_string();
        let col_names = info.column_names.clone();

        let producer_handle = thread::spawn(move || {
            let mut reader = buf_reader;
            let mut line = String::new();
            let mut chunk_idx: usize = 0;
            let mut row_offset: usize = 0;

            loop {
                if !producer_active_clone.load(Ordering::Relaxed) {
                    break;
                }

                let start = Instant::now();
                let mut rows: Vec<Vec<String>> = Vec::with_capacity(chunk_size);
                let start_row = row_offset;

                for _ in 0..chunk_size {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            let trimmed = line.trim();
                            if trimmed.is_empty() {
                                continue;
                            }
                            let fields: Vec<String> = trimmed.split(',')
                                .map(|s| s.trim().trim_matches('"').to_string())
                                .collect();
                            rows.push(fields);
                            row_offset += 1;
                        }
                        Err(_) => break,
                    }
                }

                let is_last = rows.is_empty();
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;

                if let Ok(mut times) = produce_times_clone.lock() {
                    times.push(elapsed);
                }

                let (lock, cvar) = &*buffer_clone;
                let mut buf = lock.lock().unwrap();

                while buf.is_full() && producer_active_clone.load(Ordering::Relaxed) {
                    buf = cvar.wait(buf).unwrap();
                }

                if !producer_active_clone.load(Ordering::Relaxed) {
                    break;
                }

                if is_last {
                    buf.finished = true;
                    cvar.notify_all();
                    break;
                }

                let table = build_arrow_table_from_rows(&col_names, &rows);
                match table {
                    Ok(t) => {
                        buf.chunks.push_back(StreamChunk {
                            chunk_index: chunk_idx,
                            start_row,
                            end_row: row_offset,
                            data: t,
                            is_last: false,
                        });
                        chunk_idx += 1;
                        chunks_produced_clone.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        buf.error = Some(e);
                        cvar.notify_all();
                        break;
                    }
                }

                if let Ok(mut wm) = buffer_watermarks_clone.lock() {
                    wm.push(buf.chunks.len());
                }

                cvar.notify_all();
            }
        });

        Ok(Self {
            info,
            config: PrefetchConfig {
                compression,
                ..prefetch_config
            },
            streaming_config,
            buffer,
            producer_active,
            chunks_produced,
            chunks_consumed,
            produce_times,
            consume_waits,
            buffer_watermarks,
            prefetch_hits,
            prefetch_misses,
            _producer_handle: Some(producer_handle),
        })
    }

    pub fn open_jsonl(
        name: &str,
        path: &str,
        streaming_config: StreamingConfig,
        prefetch_config: PrefetchConfig,
    ) -> Result<Self, String> {
        let compression = if matches!(prefetch_config.compression, CompressionFormat::None) {
            CompressionFormat::from_extension(path)
        } else {
            prefetch_config.compression.clone()
        };

        let file = File::open(path)
            .map_err(|e| format!("Failed to open file: {}", e))?;
        let metadata = file.metadata()
            .map_err(|e| format!("Failed to read metadata: {}", e))?;
        let file_size = metadata.len();

        let reader: Box<dyn BufRead + Send> = compression.decompress_reader(file)?;

        let mut buf_reader = BufReader::new(reader);
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

        let info = StreamingDatasetInfo {
            name: name.to_string(),
            format: "jsonl".to_string(),
            path: path.to_string(),
            total_rows: None,
            column_names: column_names.clone(),
            column_types,
            file_size_bytes: file_size,
            estimated_chunks: None,
            chunk_size: streaming_config.chunk_size,
            supports_seek: false,
        };

        let buffer = Arc::new((
            Mutex::new(PrefetchBuffer::new(prefetch_config.buffer_size)),
            Condvar::new(),
        ));
        let producer_active = Arc::new(AtomicBool::new(true));
        let chunks_produced = Arc::new(AtomicUsize::new(0));
        let chunks_consumed = Arc::new(AtomicUsize::new(0));
        let produce_times = Arc::new(Mutex::new(Vec::new()));
        let consume_waits = Arc::new(Mutex::new(Vec::new()));
        let buffer_watermarks = Arc::new(Mutex::new(Vec::new()));
        let prefetch_hits = Arc::new(AtomicUsize::new(0));
        let prefetch_misses = Arc::new(AtomicUsize::new(0));

        let buffer_clone = buffer.clone();
        let producer_active_clone = producer_active.clone();
        let chunks_produced_clone = chunks_produced.clone();
        let produce_times_clone = produce_times.clone();
        let buffer_watermarks_clone = buffer_watermarks.clone();
        let chunk_size = streaming_config.chunk_size;
        let col_names = column_names;

        let producer_handle = thread::spawn(move || {
            let mut reader = buf_reader;
            let mut line = String::new();
            let mut chunk_idx: usize = 0;
            let mut row_offset: usize = 0;

            loop {
                if !producer_active_clone.load(Ordering::Relaxed) {
                    break;
                }

                let start = Instant::now();
                let mut rows: Vec<Vec<String>> = Vec::with_capacity(chunk_size);
                let start_row = row_offset;

                for _ in 0..chunk_size {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            let trimmed = line.trim();
                            if trimmed.is_empty() {
                                continue;
                            }
                            rows.push(vec![trimmed.to_string()]);
                            row_offset += 1;
                        }
                        Err(_) => break,
                    }
                }

                let is_last = rows.is_empty();
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;

                if let Ok(mut times) = produce_times_clone.lock() {
                    times.push(elapsed);
                }

                let (lock, cvar) = &*buffer_clone;
                let mut buf = lock.lock().unwrap();

                while buf.is_full() && producer_active_clone.load(Ordering::Relaxed) {
                    buf = cvar.wait(buf).unwrap();
                }

                if !producer_active_clone.load(Ordering::Relaxed) {
                    break;
                }

                if is_last {
                    buf.finished = true;
                    cvar.notify_all();
                    break;
                }

                let table = build_arrow_table_from_rows(&col_names, &rows);
                match table {
                    Ok(t) => {
                        buf.chunks.push_back(StreamChunk {
                            chunk_index: chunk_idx,
                            start_row,
                            end_row: row_offset,
                            data: t,
                            is_last: false,
                        });
                        chunk_idx += 1;
                        chunks_produced_clone.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        buf.error = Some(e);
                        cvar.notify_all();
                        break;
                    }
                }

                if let Ok(mut wm) = buffer_watermarks_clone.lock() {
                    wm.push(buf.chunks.len());
                }

                cvar.notify_all();
            }
        });

        Ok(Self {
            info,
            config: PrefetchConfig {
                compression,
                ..prefetch_config
            },
            streaming_config,
            buffer,
            producer_active,
            chunks_produced,
            chunks_consumed,
            produce_times,
            consume_waits,
            buffer_watermarks,
            prefetch_hits,
            prefetch_misses,
            _producer_handle: Some(producer_handle),
        })
    }

    pub fn next_chunk(&self) -> Result<Option<StreamChunk>, String> {
        let (lock, cvar) = &*self.buffer;
        let wait_start = Instant::now();
        let mut buf = lock.lock().unwrap();

        if let Some(ref err) = buf.error {
            return Err(err.clone());
        }

        if buf.is_empty() && buf.finished {
            return Ok(None);
        }

        if buf.is_empty() {
            self.prefetch_misses.fetch_add(1, Ordering::Relaxed);
        } else {
            self.prefetch_hits.fetch_add(1, Ordering::Relaxed);
        }

        while buf.is_empty() && !buf.finished && buf.error.is_none() {
            let timeout = std::time::Duration::from_millis(self.config.timeout_ms);
            let result = cvar.wait_timeout(buf, timeout).unwrap();
            buf = result.0;

            if result.1.timed_out() {
                return Err("Timeout waiting for next chunk".to_string());
            }

            if let Some(ref err) = buf.error {
                return Err(err.clone());
            }
        }

        if buf.is_empty() && buf.finished {
            return Ok(None);
        }

        let chunk = buf.chunks.pop_front();
        let wait_elapsed = wait_start.elapsed().as_secs_f64() * 1000.0;

        if let Ok(mut waits) = self.consume_waits.lock() {
            waits.push(wait_elapsed);
        }

        self.chunks_consumed.fetch_add(1, Ordering::Relaxed);
        cvar.notify_all();

        Ok(chunk)
    }

    pub fn stats(&self) -> PrefetchStats {
        let total_produced = self.chunks_produced.load(Ordering::Relaxed);
        let total_consumed = self.chunks_consumed.load(Ordering::Relaxed);
        let hits = self.prefetch_hits.load(Ordering::Relaxed);
        let misses = self.prefetch_misses.load(Ordering::Relaxed);

        let avg_produce = if let Ok(times) = self.produce_times.lock() {
            if times.is_empty() {
                0.0
            } else {
                times.iter().sum::<f64>() / times.len() as f64
            }
        } else {
            0.0
        };

        let avg_wait = if let Ok(waits) = self.consume_waits.lock() {
            if waits.is_empty() {
                0.0
            } else {
                waits.iter().sum::<f64>() / waits.len() as f64
            }
        } else {
            0.0
        };

        let (high, low) = if let Ok(wm) = self.buffer_watermarks.lock() {
            (
                wm.iter().max().copied().unwrap_or(0),
                wm.iter().min().copied().unwrap_or(0),
            )
        } else {
            (0, 0)
        };

        PrefetchStats {
            total_chunks_produced: total_produced,
            total_chunks_consumed: total_consumed,
            buffer_high_watermark: high,
            buffer_low_watermark: low,
            avg_produce_time_ms: avg_produce,
            avg_consume_wait_ms: avg_wait,
            prefetch_hits: hits,
            prefetch_misses: misses,
            is_active: self.producer_active.load(Ordering::Relaxed),
        }
    }

    pub fn reset(&mut self) {
        self.producer_active.store(false, Ordering::Relaxed);
        let (lock, cvar) = &*self.buffer;
        {
            let mut buf = lock.lock().unwrap();
            buf.chunks.clear();
            buf.finished = false;
            buf.error = None;
        }
        cvar.notify_all();

        self.chunks_produced.store(0, Ordering::Relaxed);
        self.chunks_consumed.store(0, Ordering::Relaxed);
        self.prefetch_hits.store(0, Ordering::Relaxed);
        self.prefetch_misses.store(0, Ordering::Relaxed);

        if let Ok(mut times) = self.produce_times.lock() {
            times.clear();
        }
        if let Ok(mut waits) = self.consume_waits.lock() {
            waits.clear();
        }
        if let Ok(mut wm) = self.buffer_watermarks.lock() {
            wm.clear();
        }
    }
}

impl Drop for PrefetchStreamingDataset {
    fn drop(&mut self) {
        self.producer_active.store(false, Ordering::Relaxed);
        let (_, cvar) = &*self.buffer;
        cvar.notify_all();
    }
}

fn build_arrow_table_from_rows(
    column_names: &[String],
    rows: &[Vec<String>],
) -> Result<ArrowTable, String> {
    use arrow::array::StringBuilder;
    use arrow::datatypes::{DataType, Field, Schema};

    let fields: Vec<Field> = column_names
        .iter()
        .map(|name| Field::new(name.as_str(), DataType::Utf8, true))
        .collect();
    let schema = std::sync::Arc::new(Schema::new(fields));

    let mut builders: Vec<StringBuilder> = (0..column_names.len())
        .map(|_| StringBuilder::new())
        .collect();

    for row in rows {
        for (col_idx, builder) in builders.iter_mut().enumerate() {
            if col_idx < row.len() {
                builder.append_value(&row[col_idx]);
            } else {
                builder.append_null();
            }
        }
    }

    let arrays: Vec<std::sync::Arc<dyn arrow::array::Array>> = builders
        .into_iter()
        .map(|mut b| std::sync::Arc::new(b.finish()) as std::sync::Arc<dyn arrow::array::Array>)
        .collect();

    let batch = arrow::record_batch::RecordBatch::try_new(schema.clone(), arrays)
        .map_err(|e| format!("Failed to create record batch: {}", e))?;

    let mut table = ArrowTable::new("prefetch", schema);
    table.add_batch(batch)?;

    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_csv(path: &str, num_rows: usize) {
        let mut file = File::create(path).unwrap();
        writeln!(file, "col_a,col_b,col_c").unwrap();
        for i in 0..num_rows {
            writeln!(file, "val_{}_a,val_{}_b,val_{}_c", i, i, i).unwrap();
        }
    }

    #[test]
    fn test_compression_detection() {
        assert!(matches!(CompressionFormat::from_extension("data.csv"), CompressionFormat::None));
        assert!(matches!(CompressionFormat::from_extension("data.csv.gz"), CompressionFormat::Gzip));
        assert!(matches!(CompressionFormat::from_extension("data.jsonl.zst"), CompressionFormat::Zstd));
    }

    #[test]
    fn test_prefetch_streaming_csv() {
        let tmp = std::env::temp_dir().join("test_prefetch.csv");
        create_test_csv(tmp.to_str().unwrap(), 500);

        let streaming_config = StreamingConfig {
            chunk_size: 100,
            ..Default::default()
        };
        let prefetch_config = PrefetchConfig {
            buffer_size: 4,
            ..Default::default()
        };

        let dataset = PrefetchStreamingDataset::open_csv(
            "test",
            tmp.to_str().unwrap(),
            streaming_config,
            prefetch_config,
        )
        .unwrap();

        let mut total_rows = 0;
        let mut chunk_count = 0;
        while let Ok(Some(chunk)) = dataset.next_chunk() {
            chunk_count += 1;
            total_rows += chunk.end_row - chunk.start_row;
        }

        assert_eq!(chunk_count, 5);
        assert_eq!(total_rows, 500);

        let stats = dataset.stats();
        assert!(stats.total_chunks_produced >= 5);
        assert!(stats.total_chunks_consumed >= 5);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_prefetch_stats() {
        let tmp = std::env::temp_dir().join("test_prefetch_stats.csv");
        create_test_csv(tmp.to_str().unwrap(), 300);

        let streaming_config = StreamingConfig {
            chunk_size: 100,
            ..Default::default()
        };
        let prefetch_config = PrefetchConfig::default();

        let dataset = PrefetchStreamingDataset::open_csv(
            "test",
            tmp.to_str().unwrap(),
            streaming_config,
            prefetch_config,
        )
        .unwrap();

        while let Ok(Some(_)) = dataset.next_chunk() {}

        let stats = dataset.stats();
        assert!(stats.total_chunks_produced > 0);
        assert!(stats.total_chunks_consumed > 0);
        assert!(stats.avg_produce_time_ms >= 0.0);
        assert!(stats.avg_consume_wait_ms >= 0.0);

        let _ = std::fs::remove_file(&tmp);
    }
}
