use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Read;

const HF_API_BASE: &str = "https://huggingface.co/api";
const HF_DATASETS_BASE: &str = "https://huggingface.co/datasets";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfHubConfig {
    pub cache_dir: PathBuf,
    pub token: Option<String>,
    pub endpoint: String,
    pub max_retries: u32,
    pub timeout_secs: u64,
    pub use_streaming: bool,
    pub revision: String,
}

impl Default for HfHubConfig {
    fn default() -> Self {
        Self {
            cache_dir: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("biosphere")
                .join("hf_cache"),
            token: std::env::var("HF_TOKEN").ok(),
            endpoint: HF_API_BASE.to_string(),
            max_retries: 3,
            timeout_secs: 30,
            use_streaming: true,
            revision: "main".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfDatasetInfo {
    pub dataset_id: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub languages: Vec<String>,
    pub license: Option<String>,
    pub size_categories: Vec<String>,
    pub configs: Vec<HfDatasetConfig>,
    pub splits: HashMap<String, Vec<HfSplitInfo>>,
    pub downloads: Option<u64>,
    pub likes: Option<u64>,
    pub card_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfDatasetConfig {
    pub config: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfSplitInfo {
    pub split: String,
    pub num_rows: Option<u64>,
    pub num_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfParquetFile {
    pub filename: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfParquetResponse {
    pub parquet_files: Vec<HfParquetFile>,
    pub pending: Vec<HashMap<String, serde_json::Value>>,
    pub failed: Vec<HashMap<String, serde_json::Value>>,
    pub partial: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfSearchResult {
    pub dataset_id: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub downloads: Option<u64>,
    pub likes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfSearchResponse {
    pub datasets: Vec<HfSearchResult>,
    pub num_items_per_page: usize,
    pub num_total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfDownloadProgress {
    pub dataset_id: String,
    pub config: String,
    pub split: String,
    pub total_files: usize,
    pub downloaded_files: usize,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub status: HfDownloadStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HfDownloadStatus {
    Pending,
    Downloading,
    Completed,
    Failed(String),
}

pub struct HfHubClient {
    config: HfHubConfig,
    client: reqwest::blocking::Client,
}

impl HfHubClient {
    pub fn new(config: HfHubConfig) -> Result<Self, String> {
        fs::create_dir_all(&config.cache_dir)
            .map_err(|e| format!("Failed to create cache dir: {}", e))?;

        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(ref token) = config.token {
            let auth_value = format!("Bearer {}", token);
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&auth_value)
                    .map_err(|e| format!("Invalid token: {}", e))?,
            );
        }

        let client = reqwest::blocking::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self { config, client })
    }

    pub fn dataset_info(&self, dataset_id: &str) -> Result<HfDatasetInfo, String> {
        let url = format!("{}/datasets/{}", self.config.endpoint, dataset_id);
        let response = self.get_with_retry(&url)?;
        let info: HfDatasetInfo = serde_json::from_str(&response)
            .map_err(|e| format!("Failed to parse dataset info: {}", e))?;
        Ok(info)
    }

    pub fn search_datasets(
        &self,
        query: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<HfSearchResponse, String> {
        let limit = limit.unwrap_or(20);
        let offset = offset.unwrap_or(0);
        let url = format!(
            "{}/datasets?search={}&limit={}&offset={}&full=false",
            self.config.endpoint, query, limit, offset
        );
        let response = self.get_with_retry(&url)?;
        let results: HfSearchResponse = serde_json::from_str(&response)
            .map_err(|e| format!("Failed to parse search results: {}", e))?;
        Ok(results)
    }

    pub fn list_parquet_files(
        &self,
        dataset_id: &str,
        config: &str,
        split: &str,
    ) -> Result<Vec<HfParquetFile>, String> {
        let url = format!(
            "{}/datasets/{}/parquet/{}/{}",
            self.config.endpoint, dataset_id, config, split
        );
        let response = self.get_with_retry(&url)?;
        let parquet_resp: HfParquetResponse = serde_json::from_str(&response)
            .map_err(|e| format!("Failed to parse parquet files: {}", e))?;

        if parquet_resp.partial {
            return Err(format!(
                "Parquet files are still being generated for {}/{}/{}",
                dataset_id, config, split
            ));
        }

        Ok(parquet_resp.parquet_files)
    }

    pub fn download_file(
        &self,
        dataset_id: &str,
        filename: &str,
    ) -> Result<PathBuf, String> {
        let revision = &self.config.revision;
        let url = format!(
            "{}/{}/resolve/{}/{}",
            HF_DATASETS_BASE, dataset_id, revision, filename
        );

        let cache_path = self.cache_path(dataset_id, filename);

        if cache_path.exists() {
            return Ok(cache_path);
        }

        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create cache dir: {}", e))?;
        }

        let response = self.client
            .get(&url)
            .send()
            .map_err(|e| format!("Download failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Download failed with status {}: {}",
                response.status(),
                url
            ));
        }

        let bytes = response
            .bytes()
            .map_err(|e| format!("Failed to read response: {}", e))?;

        fs::write(&cache_path, &bytes)
            .map_err(|e| format!("Failed to write cache file: {}", e))?;

        Ok(cache_path)
    }

    pub fn download_dataset(
        &self,
        dataset_id: &str,
        config: &str,
        split: &str,
        progress_callback: Option<Box<dyn Fn(&HfDownloadProgress)>>,
    ) -> Result<Vec<PathBuf>, String> {
        let parquet_files = self.list_parquet_files(dataset_id, config, split)?;

        let total_files = parquet_files.len();
        let total_bytes: u64 = parquet_files.iter().map(|f| f.size).sum();
        let mut downloaded = Vec::new();
        let mut downloaded_bytes = 0u64;

        let mut progress = HfDownloadProgress {
            dataset_id: dataset_id.to_string(),
            config: config.to_string(),
            split: split.to_string(),
            total_files,
            downloaded_files: 0,
            total_bytes,
            downloaded_bytes: 0,
            status: HfDownloadStatus::Downloading,
        };

        if let Some(ref cb) = progress_callback {
            cb(&progress);
        }

        for file in &parquet_files {
            match self.download_file(dataset_id, &file.filename) {
                Ok(path) => {
                    downloaded.push(path);
                    downloaded_bytes += file.size;
                    progress.downloaded_files = downloaded.len();
                    progress.downloaded_bytes = downloaded_bytes;

                    if let Some(ref cb) = progress_callback {
                        cb(&progress);
                    }
                }
                Err(e) => {
                    progress.status = HfDownloadStatus::Failed(e.clone());
                    if let Some(ref cb) = progress_callback {
                        cb(&progress);
                    }
                    return Err(e);
                }
            }
        }

        progress.status = HfDownloadStatus::Completed;
        if let Some(ref cb) = progress_callback {
            cb(&progress);
        }

        Ok(downloaded)
    }

    pub fn load_dataset(
        &self,
        dataset_id: &str,
        config: &str,
        split: &str,
    ) -> Result<Vec<PathBuf>, String> {
        self.download_dataset(dataset_id, config, split, None)
    }

    pub fn load_dataset_streaming(
        &self,
        dataset_id: &str,
        config: &str,
        split: &str,
    ) -> Result<HfStreamingLoader, String> {
        let parquet_files = self.list_parquet_files(dataset_id, config, split)?;
        HfStreamingLoader::new(self.config.clone(), dataset_id, parquet_files)
    }

    pub fn cache_path(&self, dataset_id: &str, filename: &str) -> PathBuf {
        self.config.cache_dir
            .join(dataset_id.replace('/', "__"))
            .join(filename)
    }

    pub fn clear_cache(&self, dataset_id: Option<&str>) -> Result<(), String> {
        if let Some(id) = dataset_id {
            let dir = self.config.cache_dir.join(id.replace('/', "__"));
            if dir.exists() {
                fs::remove_dir_all(&dir)
                    .map_err(|e| format!("Failed to clear cache: {}", e))?;
            }
        } else {
            if self.config.cache_dir.exists() {
                fs::remove_dir_all(&self.config.cache_dir)
                    .map_err(|e| format!("Failed to clear cache: {}", e))?;
            }
            fs::create_dir_all(&self.config.cache_dir)
                .map_err(|e| format!("Failed to recreate cache dir: {}", e))?;
        }
        Ok(())
    }

    pub fn cache_size(&self) -> Result<u64, String> {
        dir_size(&self.config.cache_dir)
    }

    pub fn cached_datasets(&self) -> Result<Vec<String>, String> {
        let mut datasets = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.config.cache_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    if let Some(name) = entry.file_name().to_str() {
                        datasets.push(name.replace("__", "/"));
                    }
                }
            }
        }
        Ok(datasets)
    }

    fn get_with_retry(&self, url: &str) -> Result<String, String> {
        let mut last_error = String::new();

        for attempt in 0..self.config.max_retries {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(500 * attempt as u64));
            }

            match self.client.get(url).send() {
                Ok(response) => {
                    if response.status().is_success() {
                        return response
                            .text()
                            .map_err(|e| format!("Failed to read response: {}", e));
                    }
                    last_error = format!("HTTP {}", response.status());
                }
                Err(e) => {
                    last_error = format!("Request error: {}", e);
                }
            }
        }

        Err(format!("Failed after {} retries: {}", self.config.max_retries, last_error))
    }
}

pub struct HfStreamingLoader {
    config: HfHubConfig,
    dataset_id: String,
    parquet_files: Vec<HfParquetFile>,
    current_file: usize,
    current_row: usize,
    current_batch: Option<Vec<u8>>,
    client: reqwest::blocking::Client,
}

impl HfStreamingLoader {
    fn new(
        config: HfHubConfig,
        dataset_id: &str,
        parquet_files: Vec<HfParquetFile>,
    ) -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            config,
            dataset_id: dataset_id.to_string(),
            parquet_files,
            current_file: 0,
            current_row: 0,
            current_batch: None,
            client,
        })
    }

    pub fn next_batch(&mut self, batch_size: usize) -> Result<Option<Vec<Vec<u8>>>, String> {
        if self.current_file >= self.parquet_files.len() {
            return Ok(None);
        }

        let file = &self.parquet_files[self.current_file];

        if self.current_batch.is_none() {
            let url = &file.url;
            let response = self.client
                .get(url)
                .send()
                .map_err(|e| format!("Streaming download failed: {}", e))?;

            if !response.status().is_success() {
                return Err(format!("HTTP {} for {}", response.status(), url));
            }

            let bytes = response
                .bytes()
                .map_err(|e| format!("Failed to read streaming data: {}", e))?;

            self.current_batch = Some(bytes.to_vec());
            self.current_row = 0;
        }

        let data = self.current_batch.as_ref().unwrap();
        let mut rows = Vec::new();

        for _ in 0..batch_size {
            if self.current_row >= data.len() {
                self.current_file += 1;
                self.current_batch = None;
                self.current_row = 0;
                break;
            }

            let end = (self.current_row + 1024).min(data.len());
            rows.push(data[self.current_row..end].to_vec());
            self.current_row = end;
        }

        if rows.is_empty() && self.current_file < self.parquet_files.len() {
            return self.next_batch(batch_size);
        }

        Ok(Some(rows))
    }

    pub fn total_files(&self) -> usize {
        self.parquet_files.len()
    }

    pub fn current_file_index(&self) -> usize {
        self.current_file
    }

    pub fn progress(&self) -> f64 {
        if self.parquet_files.is_empty() {
            return 1.0;
        }
        self.current_file as f64 / self.parquet_files.len() as f64
    }

    pub fn reset(&mut self) {
        self.current_file = 0;
        self.current_row = 0;
        self.current_batch = None;
    }
}

pub struct HfDatasetLoader {
    client: HfHubClient,
    dataset_id: String,
    config: String,
    split: String,
    local_files: Vec<PathBuf>,
    current_file: usize,
}

impl HfDatasetLoader {
    pub fn new(
        hub_config: HfHubConfig,
        dataset_id: &str,
        config: &str,
        split: &str,
    ) -> Result<Self, String> {
        let client = HfHubClient::new(hub_config)?;
        let local_files = client.load_dataset(dataset_id, config, split)?;

        Ok(Self {
            client,
            dataset_id: dataset_id.to_string(),
            config: config.to_string(),
            split: split.to_string(),
            local_files,
            current_file: 0,
        })
    }

    pub fn files(&self) -> &[PathBuf] {
        &self.local_files
    }

    pub fn file_count(&self) -> usize {
        self.local_files.len()
    }

    pub fn dataset_id(&self) -> &str {
        &self.dataset_id
    }

    pub fn config_name(&self) -> &str {
        &self.config
    }

    pub fn split_name(&self) -> &str {
        &self.split
    }

    pub fn read_file(&self, index: usize) -> Result<Vec<u8>, String> {
        if index >= self.local_files.len() {
            return Err(format!("File index {} out of range", index));
        }

        let path = &self.local_files[index];
        let mut file = fs::File::open(path)
            .map_err(|e| format!("Failed to open file: {}", e))?;

        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        Ok(data)
    }

    pub fn iter_files(&self) -> impl Iterator<Item = &PathBuf> {
        self.local_files.iter()
    }
}

impl Iterator for HfDatasetLoader {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_file >= self.local_files.len() {
            return None;
        }

        match self.read_file(self.current_file) {
            Ok(data) => {
                self.current_file += 1;
                Some(data)
            }
            Err(_) => {
                self.current_file += 1;
                self.next()
            }
        }
    }
}

fn dir_size(path: &Path) -> Result<u64, String> {
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                total += dir_size(&path).unwrap_or(0);
            } else if let Ok(meta) = path.metadata() {
                total += meta.len();
            }
        }
    }
    Ok(total)
}

pub fn list_popular_datasets() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        ("c4", "en", "train", "Colossal Clean Crawled Corpus - web text"),
        ("wikipedia", "20220301.en", "train", "Wikipedia articles"),
        ("the_pile", "all", "train", "The Pile - diverse text corpus"),
        ("bookcorpus", "plain_text", "train", "BookCorpus - books"),
        ("openwebtext", "plain_text", "train", "OpenWebText - Reddit links"),
        ("starcoderdata", "python", "train", "StarCoder - code data"),
        ("math_dataset", "algebra__linear_1d", "train", "Math dataset"),
        ("gsm8k", "main", "train", "Grade school math 8K"),
        ("databricks/databricks-dolly-15k", "default", "train", "Dolly instruction dataset"),
        ("tatsu-lab/alpaca", "default", "train", "Alpaca instruction dataset"),
        ("Open-Orca/OpenOrca", "default", "train", "OpenOrca instruction dataset"),
        ("HuggingFaceH4/ultrachat_200k", "default", "train_sft", "UltraChat conversation"),
        ("Anthropic/hh-rlhf", "default", "train", "HH-RLHF preference data"),
        ("openai/summarize_from_feedback", "comparisons", "train", "RLHF summarization"),
        ("bigcode/the-stack", "default", "train", "The Stack - code corpus"),
        ("togethercomputer/RedPajama-Data-1T", "default", "train", "RedPajama 1T tokens"),
        ("allenai/c4", "en", "train", "C4 cleaned text"),
        ("EleutherAI/pile", "all", "train", "The Pile"),
        ("bigscience/xP3", "en", "train", "xP3 multilingual instructions"),
        ("laion/laion2B-en", "default", "train", "LAION-2B image-text pairs"),
    ].into_iter().map(|(id, cfg, split, desc)| (id, cfg, split, desc)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hf_config_default() {
        let config = HfHubConfig::default();
        assert!(config.cache_dir.to_string_lossy().contains("hf_cache"));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout_secs, 30);
    }

    #[test]
    fn test_cache_path() {
        let config = HfHubConfig {
            cache_dir: PathBuf::from("/tmp/test_cache"),
            ..Default::default()
        };
        let client = HfHubClient::new(config).unwrap();
        let path = client.cache_path("org/dataset", "data.parquet");
        assert!(path.to_string_lossy().contains("org__dataset"));
        assert!(path.to_string_lossy().contains("data.parquet"));
    }

    #[test]
    fn test_popular_datasets() {
        let datasets = list_popular_datasets();
        assert!(datasets.len() >= 10);
        assert!(datasets.iter().any(|(id, _, _, _)| *id == "c4"));
        assert!(datasets.iter().any(|(id, _, _, _)| *id == "wikipedia"));
    }

    #[test]
    fn test_hf_download_status() {
        let status = HfDownloadStatus::Pending;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("pending"));

        let status = HfDownloadStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("completed"));

        let status = HfDownloadStatus::Failed("error".to_string());
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("failed"));
    }

    #[test]
    fn test_hf_dataset_info_serialization() {
        let info = HfDatasetInfo {
            dataset_id: "test/dataset".to_string(),
            description: Some("Test dataset".to_string()),
            tags: vec!["nlp".to_string()],
            languages: vec!["en".to_string()],
            license: Some("mit".to_string()),
            size_categories: vec!["1M<n<10M".to_string()],
            configs: vec![HfDatasetConfig {
                config: "default".to_string(),
                description: Some("Default config".to_string()),
            }],
            splits: {
                let mut m = HashMap::new();
                m.insert("default".to_string(), vec![HfSplitInfo {
                    split: "train".to_string(),
                    num_rows: Some(1000),
                    num_bytes: Some(50000),
                }]);
                m
            },
            downloads: Some(100),
            likes: Some(50),
            card_data: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: HfDatasetInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.dataset_id, "test/dataset");
        assert_eq!(parsed.configs.len(), 1);
    }

    #[test]
    fn test_hf_parquet_response() {
        let resp = HfParquetResponse {
            parquet_files: vec![
                HfParquetFile {
                    filename: "train-00000.parquet".to_string(),
                    size: 1024,
                    url: "https://example.com/file.parquet".to_string(),
                },
            ],
            pending: vec![],
            failed: vec![],
            partial: false,
        };

        let json = serde_json::to_string(&resp).unwrap();
        let parsed: HfParquetResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.parquet_files.len(), 1);
        assert!(!parsed.partial);
    }

    #[test]
    fn test_hf_search_response() {
        let resp = HfSearchResponse {
            datasets: vec![
                HfSearchResult {
                    dataset_id: "test/dataset".to_string(),
                    description: Some("Test".to_string()),
                    tags: vec!["nlp".to_string()],
                    downloads: Some(100),
                    likes: Some(50),
                },
            ],
            num_items_per_page: 20,
            num_total: 1,
        };

        let json = serde_json::to_string(&resp).unwrap();
        let parsed: HfSearchResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.datasets.len(), 1);
        assert_eq!(parsed.num_total, 1);
    }
}
