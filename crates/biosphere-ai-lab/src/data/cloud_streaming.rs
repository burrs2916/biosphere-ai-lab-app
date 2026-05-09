use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{BufRead, Read};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::data::arrow_table::ArrowTable;
use crate::data::streaming::{StreamChunk, StreamingConfig, StreamingDatasetInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudProvider {
    S3,
    GCS,
    OSS,
    MinIO,
    Custom(String),
}

impl CloudProvider {
    pub fn endpoint_pattern(&self) -> &str {
        match self {
            CloudProvider::S3 => "s3.{region}.amazonaws.com",
            CloudProvider::GCS => "storage.googleapis.com",
            CloudProvider::OSS => "oss-{region}.aliyuncs.com",
            CloudProvider::MinIO => "",
            CloudProvider::Custom(_) => "",
        }
    }

    pub fn default_region(&self) -> &str {
        match self {
            CloudProvider::S3 => "us-east-1",
            CloudProvider::GCS => "auto",
            CloudProvider::OSS => "oss-cn-hangzhou",
            CloudProvider::MinIO => "us-east-1",
            CloudProvider::Custom(_) => "auto",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudStorageConfig {
    pub provider: CloudProvider,
    pub endpoint: Option<String>,
    pub region: String,
    pub bucket: String,
    pub prefix: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    pub session_token: Option<String>,
    pub use_ssl: bool,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub retry_backoff_ms: u64,
    pub max_concurrent_requests: usize,
}

impl Default for CloudStorageConfig {
    fn default() -> Self {
        Self {
            provider: CloudProvider::S3,
            endpoint: None,
            region: "us-east-1".to_string(),
            bucket: String::new(),
            prefix: None,
            access_key: None,
            secret_key: None,
            session_token: None,
            use_ssl: true,
            timeout_secs: 30,
            max_retries: 3,
            retry_backoff_ms: 1000,
            max_concurrent_requests: 8,
        }
    }
}

impl CloudStorageConfig {
    pub fn from_env(provider: CloudProvider, bucket: &str) -> Self {
        let access_key = std::env::var("AWS_ACCESS_KEY_ID")
            .or_else(|_| std::env::var("OSS_ACCESS_KEY_ID"))
            .ok();
        let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
            .or_else(|_| std::env::var("OSS_ACCESS_KEY_SECRET"))
            .ok();
        let session_token = std::env::var("AWS_SESSION_TOKEN").ok();
        let region = std::env::var("AWS_REGION")
            .or_else(|_| std::env::var("OSS_REGION"))
            .unwrap_or_else(|_| provider.default_region().to_string());
        let endpoint = std::env::var("S3_ENDPOINT")
            .or_else(|_| std::env::var("OSS_ENDPOINT"))
            .ok();

        Self {
            provider,
            endpoint,
            region,
            bucket: bucket.to_string(),
            prefix: None,
            access_key,
            secret_key,
            session_token,
            ..Default::default()
        }
    }

    pub fn build_url(&self, key: &str) -> String {
        let scheme = if self.use_ssl { "https" } else { "http" };

        let host = match &self.endpoint {
            Some(ep) => ep.clone(),
            None => match &self.provider {
                CloudProvider::S3 => {
                    if self.region == "us-east-1" {
                        "s3.amazonaws.com".to_string()
                    } else {
                        format!("s3.{}.amazonaws.com", self.region)
                    }
                }
                CloudProvider::GCS => "storage.googleapis.com".to_string(),
                CloudProvider::OSS => format!("oss-{}.aliyuncs.com", self.region),
                CloudProvider::MinIO => "localhost:9000".to_string(),
                CloudProvider::Custom(ep) => ep.clone(),
            },
        };

        let full_key = match &self.prefix {
            Some(prefix) => format!("{}/{}", prefix.trim_end_matches('/'), key.trim_start_matches('/')),
            None => key.to_string(),
        };

        match &self.provider {
            CloudProvider::GCS => {
                format!("{}://{}/{}/{}", scheme, host, self.bucket, full_key)
            }
            _ => {
                format!("{}://{}.{}/{}", scheme, self.bucket, host, full_key)
            }
        }
    }

    fn sign_request(&self, _method: &str, _url: &str, headers: &mut HashMap<String, String>) {
        if let (Some(ak), Some(sk)) = (&self.access_key, &self.secret_key) {
            headers.insert("Authorization".to_string(), format!("Bearer {}", ak));
            headers.insert("X-Access-Key".to_string(), ak.clone());
            headers.insert("X-Secret-Key".to_string(), sk.clone());
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub cache_dir: PathBuf,
    pub max_cache_size_bytes: u64,
    pub max_cached_files: usize,
    pub eviction_policy: CacheEvictionPolicy,
    pub compress_cached: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: PathBuf::from("/tmp/biosphere_cloud_cache"),
            max_cache_size_bytes: 100 * 1024 * 1024 * 1024,
            max_cached_files: 1000,
            eviction_policy: CacheEvictionPolicy::LRU,
            compress_cached: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheEvictionPolicy {
    LRU,
    FIFO,
    LFU,
}

struct CacheEntry {
    key: String,
    local_path: PathBuf,
    size_bytes: u64,
    last_access: Instant,
    access_count: u64,
}

pub struct CloudCache {
    config: CacheConfig,
    entries: Arc<Mutex<HashMap<String, CacheEntry>>>,
    access_order: Arc<Mutex<VecDeque<String>>>,
    total_size: Arc<AtomicU64>,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl CloudCache {
    pub fn new(config: CacheConfig) -> Result<Self, String> {
        fs::create_dir_all(&config.cache_dir)
            .map_err(|e| format!("Failed to create cache dir: {}", e))?;

        Ok(Self {
            config,
            entries: Arc::new(Mutex::new(HashMap::new())),
            access_order: Arc::new(Mutex::new(VecDeque::new())),
            total_size: Arc::new(AtomicU64::new(0)),
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        })
    }

    pub fn get(&self, key: &str) -> Option<PathBuf> {
        let mut entries = self.entries.lock().unwrap();
        let mut order = self.access_order.lock().unwrap();

        if let Some(entry) = entries.get_mut(key) {
            if entry.local_path.exists() {
                entry.last_access = Instant::now();
                entry.access_count += 1;
                self.hits.fetch_add(1, Ordering::Relaxed);

                order.retain(|k| k != key);
                order.push_back(key.to_string());

                return Some(entry.local_path.clone());
            } else {
                entries.remove(key);
                order.retain(|k| k != key);
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    pub fn put(&self, key: &str, data: &[u8]) -> Result<PathBuf, String> {
        let local_path = self.cache_path(key);

        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create cache parent dir: {}", e))?;
        }

        fs::write(&local_path, data)
            .map_err(|e| format!("Failed to write cache file: {}", e))?;

        let size = data.len() as u64;

        self.evict_if_needed(size);

        let mut entries = self.entries.lock().unwrap();
        let mut order = self.access_order.lock().unwrap();

        entries.insert(key.to_string(), CacheEntry {
            key: key.to_string(),
            local_path: local_path.clone(),
            size_bytes: size,
            last_access: Instant::now(),
            access_count: 1,
        });

        order.push_back(key.to_string());
        self.total_size.fetch_add(size, Ordering::Relaxed);

        Ok(local_path)
    }

    fn evict_if_needed(&self, incoming_size: u64) {
        let current = self.total_size.load(Ordering::Relaxed);
        if current + incoming_size <= self.config.max_cache_size_bytes {
            return;
        }

        let mut entries = self.entries.lock().unwrap();
        let mut order = self.access_order.lock().unwrap();

        let mut to_evict = Vec::new();
        let mut freed = 0u64;

        match self.config.eviction_policy {
            CacheEvictionPolicy::LRU | CacheEvictionPolicy::FIFO => {
                while let Some(key) = order.pop_front() {
                    if let Some(entry) = entries.get(&key) {
                        to_evict.push(key.clone());
                        freed += entry.size_bytes;
                        if current + incoming_size - freed <= self.config.max_cache_size_bytes {
                            break;
                        }
                    }
                }
            }
            CacheEvictionPolicy::LFU => {
                let mut sorted: Vec<_> = entries.iter().collect();
                sorted.sort_by_key(|(_, e)| e.access_count);
                for (key, entry) in sorted {
                    to_evict.push(key.clone());
                    freed += entry.size_bytes;
                    if current + incoming_size - freed <= self.config.max_cache_size_bytes {
                        break;
                    }
                }
            }
        }

        for key in &to_evict {
            if let Some(entry) = entries.remove(key) {
                let _ = fs::remove_file(&entry.local_path);
                self.total_size.fetch_sub(entry.size_bytes, Ordering::Relaxed);
            }
            order.retain(|k| k != key);
        }
    }

    fn cache_path(&self, key: &str) -> PathBuf {
        let safe_key = key.replace('/', "_").replace('\\', "_").replace(':', "_");
        self.config.cache_dir.join(safe_key)
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            total_entries: self.entries.lock().unwrap().len(),
            total_size_bytes: self.total_size.load(Ordering::Relaxed),
            max_size_bytes: self.config.max_cache_size_bytes,
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            hit_rate: {
                let h = self.hits.load(Ordering::Relaxed) as f64;
                let m = self.misses.load(Ordering::Relaxed) as f64;
                if h + m > 0.0 { h / (h + m) } else { 0.0 }
            },
        }
    }

    pub fn clear(&self) -> Result<(), String> {
        let mut entries = self.entries.lock().unwrap();
        let mut order = self.access_order.lock().unwrap();

        for (_, entry) in entries.drain() {
            let _ = fs::remove_file(&entry.local_path);
        }
        order.clear();
        self.total_size.store(0, Ordering::Relaxed);

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size_bytes: u64,
    pub max_size_bytes: u64,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

pub struct CloudStreamingReader {
    config: CloudStorageConfig,
    key: String,
    url: String,
    file_size: u64,
    position: u64,
    cache: Option<Arc<CloudCache>>,
    client: reqwest::blocking::Client,
    stats: CloudReadStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CloudReadStats {
    pub bytes_read: u64,
    pub requests_made: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub avg_latency_ms: f64,
    pub total_latency_ms: f64,
}

impl CloudStreamingReader {
    pub fn new(
        config: CloudStorageConfig,
        key: &str,
        cache: Option<Arc<CloudCache>>,
    ) -> Result<Self, String> {
        let url = config.build_url(key);

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let file_size = Self::head_object(&client, &url, &config)
            .unwrap_or(0);

        Ok(Self {
            config,
            key: key.to_string(),
            url,
            file_size,
            position: 0,
            cache,
            client,
            stats: CloudReadStats::default(),
        })
    }

    fn head_object(
        client: &reqwest::blocking::Client,
        url: &str,
        config: &CloudStorageConfig,
    ) -> Result<u64, String> {
        let mut headers = HashMap::new();
        config.sign_request("HEAD", url, &mut headers);

        let mut req = client.head(url);
        for (k, v) in &headers {
            req = req.header(k.as_str(), v.as_str());
        }

        let resp = req.send()
            .map_err(|e| format!("HEAD request failed: {}", e))?;

        resp.headers()
            .get("content-length")
            .and_then(|v: &reqwest::header::HeaderValue| v.to_str().ok())
            .and_then(|v: &str| v.parse().ok())
            .ok_or_else(|| "Could not determine file size".to_string())
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn read_range(&mut self, start: u64, end: u64) -> Result<Vec<u8>, String> {
        let cache_key = format!("{}:{}:{}", self.key, start, end);

        if let Some(ref cache) = self.cache {
            if let Some(cached_path) = cache.get(&cache_key) {
                self.stats.cache_hits += 1;
                return fs::read(&cached_path)
                    .map_err(|e| format!("Failed to read cached file: {}", e));
            }
            self.stats.cache_misses += 1;
        }

        let start_time = Instant::now();
        let data = self.download_range_with_retry(start, end)?;
        let latency = start_time.elapsed();

        self.stats.requests_made += 1;
        self.stats.bytes_read += data.len() as u64;
        self.stats.total_latency_ms += latency.as_secs_f64() * 1000.0;
        self.stats.avg_latency_ms = if self.stats.requests_made > 0 {
            self.stats.total_latency_ms / self.stats.requests_made as f64
        } else {
            0.0
        };

        if let Some(ref cache) = self.cache {
            let _ = cache.put(&cache_key, &data);
        }

        Ok(data)
    }

    fn download_range_with_retry(&self, start: u64, end: u64) -> Result<Vec<u8>, String> {
        let mut last_error = String::new();

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                let backoff = self.config.retry_backoff_ms * (1u64 << (attempt - 1));
                std::thread::sleep(Duration::from_millis(backoff));
            }

            match self.download_range(start, end) {
                Ok(data) => return Ok(data),
                Err(e) => {
                    last_error = e;
                }
            }
        }

        Err(format!("Failed after {} retries: {}", self.config.max_retries, last_error))
    }

    fn download_range(&self, start: u64, end: u64) -> Result<Vec<u8>, String> {
        let mut headers = HashMap::new();
        self.config.sign_request("GET", &self.url, &mut headers);

        let range_header = format!("bytes={}-{}", start, end);
        headers.insert("Range".to_string(), range_header);

        let mut req = self.client.get(&self.url);
        for (k, v) in &headers {
            req = req.header(k.as_str(), v.as_str());
        }

        let resp = req.send()
            .map_err(|e| format!("Range request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("HTTP error: {} for range {}-{}", resp.status(), start, end));
        }

        let data: Vec<u8> = resp.bytes()
            .map_err(|e| format!("Failed to read response body: {}", e))?
            .to_vec();

        Ok(data)
    }

    pub fn stats(&self) -> &CloudReadStats {
        &self.stats
    }
}

pub struct CloudStreamingDataset {
    pub info: StreamingDatasetInfo,
    config: StreamingConfig,
    storage_config: CloudStorageConfig,
    reader: Option<CloudStreamingReader>,
    cache: Option<Arc<CloudCache>>,
    current_chunk: usize,
    exhausted: bool,
    total_chunks: Option<usize>,
}

impl CloudStreamingDataset {
    pub fn open_csv(
        name: &str,
        key: &str,
        storage_config: CloudStorageConfig,
        streaming_config: StreamingConfig,
        cache: Option<Arc<CloudCache>>,
    ) -> Result<Self, String> {
        let mut reader = CloudStreamingReader::new(
            storage_config.clone(),
            key,
            cache.clone(),
        )?;

        let header_data = reader.read_range(0, 4096.min(reader.file_size().saturating_sub(1)))?;
        let header_str = String::from_utf8_lossy(&header_data);
        let first_line = header_str.lines().next().unwrap_or("");

        let column_names: Vec<String> = first_line
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .collect();

        let column_types: Vec<String> = column_names.iter()
            .map(|_| "string".to_string())
            .collect();

        let total_chunks = if reader.file_size() > 0 {
            Some((reader.file_size() as usize / streaming_config.chunk_size) + 1)
        } else {
            None
        };

        let info = StreamingDatasetInfo {
            name: name.to_string(),
            format: "csv".to_string(),
            path: storage_config.build_url(key),
            total_rows: None,
            column_names,
            column_types,
            file_size_bytes: reader.file_size(),
            estimated_chunks: total_chunks,
            chunk_size: streaming_config.chunk_size,
            supports_seek: true,
        };

        Ok(Self {
            info,
            config: streaming_config,
            storage_config,
            reader: Some(reader),
            cache,
            current_chunk: 0,
            exhausted: false,
            total_chunks,
        })
    }

    pub fn open_jsonl(
        name: &str,
        key: &str,
        storage_config: CloudStorageConfig,
        streaming_config: StreamingConfig,
        cache: Option<Arc<CloudCache>>,
    ) -> Result<Self, String> {
        let mut reader = CloudStreamingReader::new(
            storage_config.clone(),
            key,
            cache.clone(),
        )?;

        let header_data = reader.read_range(0, 4096.min(reader.file_size().saturating_sub(1)))?;
        let header_str = String::from_utf8_lossy(&header_data);
        let first_line = header_str.lines().next().unwrap_or("{}");

        let column_names: Vec<String> = if let Ok(obj) = serde_json::from_str::<serde_json::Value>(first_line) {
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

        let total_chunks = if reader.file_size() > 0 {
            Some((reader.file_size() as usize / streaming_config.chunk_size) + 1)
        } else {
            None
        };

        let info = StreamingDatasetInfo {
            name: name.to_string(),
            format: "jsonl".to_string(),
            path: storage_config.build_url(key),
            total_rows: None,
            column_names,
            column_types,
            file_size_bytes: reader.file_size(),
            estimated_chunks: total_chunks,
            chunk_size: streaming_config.chunk_size,
            supports_seek: true,
        };

        Ok(Self {
            info,
            config: streaming_config,
            storage_config,
            reader: Some(reader),
            cache,
            current_chunk: 0,
            exhausted: false,
            total_chunks,
        })
    }

    pub fn open_parquet(
        name: &str,
        key: &str,
        storage_config: CloudStorageConfig,
        streaming_config: StreamingConfig,
        cache: Option<Arc<CloudCache>>,
    ) -> Result<Self, String> {
        let reader = CloudStreamingReader::new(
            storage_config.clone(),
            key,
            cache.clone(),
        )?;

        let total_chunks = if reader.file_size() > 0 {
            Some((reader.file_size() as usize / streaming_config.chunk_size) + 1)
        } else {
            None
        };

        let info = StreamingDatasetInfo {
            name: name.to_string(),
            format: "parquet".to_string(),
            path: storage_config.build_url(key),
            total_rows: None,
            column_names: vec![],
            column_types: vec![],
            file_size_bytes: reader.file_size(),
            estimated_chunks: total_chunks,
            chunk_size: streaming_config.chunk_size,
            supports_seek: true,
        };

        Ok(Self {
            info,
            config: streaming_config,
            storage_config,
            reader: Some(reader),
            cache,
            current_chunk: 0,
            exhausted: false,
            total_chunks,
        })
    }

    pub fn next_chunk(&mut self) -> Result<Option<StreamChunk>, String> {
        if self.exhausted {
            return Ok(None);
        }

        let reader = match &mut self.reader {
            Some(r) => r,
            None => return Ok(None),
        };

        let chunk_size = self.config.chunk_size as u64;
        let start = self.current_chunk as u64 * chunk_size;
        let end = (start + chunk_size - 1).min(reader.file_size().saturating_sub(1));

        if start >= reader.file_size() {
            self.exhausted = true;
            return Ok(None);
        }

        let data = reader.read_range(start, end)?;
        let is_last = end >= reader.file_size().saturating_sub(1);

        let _rows: Vec<Vec<String>> = match self.info.format.as_str() {
            "csv" => {
                let text = String::from_utf8_lossy(&data);
                let mut rows = Vec::new();
                for (i, line) in text.lines().enumerate() {
                    if i == 0 && self.current_chunk == 0 {
                        continue;
                    }
                    if line.trim().is_empty() {
                        continue;
                    }
                    let fields: Vec<String> = line.split(',')
                        .map(|s| s.trim().trim_matches('"').to_string())
                        .collect();
                    rows.push(fields);
                }
                rows
            }
            "jsonl" => {
                let text = String::from_utf8_lossy(&data);
                text.lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| vec![l.to_string()])
                    .collect()
            }
            _ => {
                vec![vec![String::from_utf8_lossy(&data).to_string()]]
            }
        };

        let schema = std::sync::Arc::new(arrow::datatypes::Schema::new(
            self.info.column_names.iter().map(|name| {
                arrow::datatypes::Field::new(name, arrow::datatypes::DataType::Utf8, true)
            }).collect::<Vec<_>>()
        ));

        let chunk = StreamChunk {
            chunk_index: self.current_chunk,
            start_row: self.current_chunk * self.config.chunk_size,
            end_row: (self.current_chunk + 1) * self.config.chunk_size,
            data: ArrowTable::new(&format!("cloud_chunk_{}", self.current_chunk), schema),
            is_last,
        };

        self.current_chunk += 1;

        if is_last {
            self.exhausted = true;
        }

        Ok(Some(chunk))
    }

    pub fn progress(&self) -> CloudStreamProgress {
        CloudStreamProgress {
            chunks_read: self.current_chunk,
            total_chunks: self.total_chunks,
            exhausted: self.exhausted,
            completion_pct: match self.total_chunks {
                Some(total) if total > 0 => {
                    (self.current_chunk as f64 / total as f64) * 100.0
                }
                _ => 0.0,
            },
        }
    }

    pub fn cache_stats(&self) -> Option<CacheStats> {
        self.cache.as_ref().map(|c| c.stats())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudStreamProgress {
    pub chunks_read: usize,
    pub total_chunks: Option<usize>,
    pub exhausted: bool,
    pub completion_pct: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_storage_config_url() {
        let config = CloudStorageConfig {
            provider: CloudProvider::S3,
            region: "us-west-2".to_string(),
            bucket: "my-bucket".to_string(),
            ..Default::default()
        };

        let url = config.build_url("data/train.csv");
        assert!(url.contains("my-bucket"));
        assert!(url.contains("s3.us-west-2.amazonaws.com"));
        assert!(url.contains("data/train.csv"));
    }

    #[test]
    fn test_cloud_storage_config_gcs_url() {
        let config = CloudStorageConfig {
            provider: CloudProvider::GCS,
            bucket: "my-bucket".to_string(),
            ..Default::default()
        };

        let url = config.build_url("data/train.jsonl");
        assert!(url.contains("storage.googleapis.com"));
        assert!(url.contains("my-bucket"));
        assert!(url.contains("data/train.jsonl"));
    }

    #[test]
    fn test_cloud_storage_config_with_prefix() {
        let config = CloudStorageConfig {
            provider: CloudProvider::S3,
            bucket: "my-bucket".to_string(),
            prefix: Some("datasets/v1".to_string()),
            ..Default::default()
        };

        let url = config.build_url("train.csv");
        assert!(url.contains("datasets/v1/train.csv"));
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.max_cache_size_bytes, 100 * 1024 * 1024 * 1024);
        assert_eq!(config.max_cached_files, 1000);
    }

    #[test]
    fn test_cloud_cache_new() {
        let config = CacheConfig {
            cache_dir: PathBuf::from("/tmp/biosphere_test_cache"),
            max_cache_size_bytes: 1024 * 1024,
            ..Default::default()
        };

        let cache = CloudCache::new(config);
        assert!(cache.is_ok());

        let cache = cache.unwrap();
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cloud_cache_put_and_get() {
        let config = CacheConfig {
            cache_dir: PathBuf::from("/tmp/biosphere_test_cache2"),
            max_cache_size_bytes: 1024 * 1024,
            ..Default::default()
        };

        let cache = CloudCache::new(config).unwrap();

        let data = b"hello cloud cache";
        let path = cache.put("test_key", data);
        assert!(path.is_ok());

        let cached = cache.get("test_key");
        assert!(cached.is_some());

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);

        let _ = cache.clear();
    }

    #[test]
    fn test_cloud_cache_miss() {
        let config = CacheConfig {
            cache_dir: PathBuf::from("/tmp/biosphere_test_cache3"),
            max_cache_size_bytes: 1024 * 1024,
            ..Default::default()
        };

        let cache = CloudCache::new(config).unwrap();

        let cached = cache.get("nonexistent_key");
        assert!(cached.is_none());

        let stats = cache.stats();
        assert_eq!(stats.misses, 1);

        let _ = cache.clear();
    }

    #[test]
    fn test_cloud_stream_progress() {
        let progress = CloudStreamProgress {
            chunks_read: 50,
            total_chunks: Some(100),
            exhausted: false,
            completion_pct: 50.0,
        };

        assert_eq!(progress.completion_pct, 50.0);
        assert!(!progress.exhausted);
    }
}
