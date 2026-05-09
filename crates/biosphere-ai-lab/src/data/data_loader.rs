use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use crate::data::data_collator::DataCollator;
use crate::data::sampler::{BatchSampler, DistributedSampler, DistributedSamplerConfig};
use crate::data::tokenizer::{BatchEncoding, TokenizerPipeline};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLoaderConfig {
    pub batch_size: usize,
    pub num_workers: usize,
    pub prefetch_factor: usize,
    pub drop_last: bool,
    pub timeout_ms: u64,
    pub pin_memory: bool,
    pub dynamic_batching: Option<DynamicBatchingConfig>,
    pub shuffle: bool,
    pub seed: u64,
}

impl Default for DataLoaderConfig {
    fn default() -> Self {
        Self {
            batch_size: 32,
            num_workers: 0,
            prefetch_factor: 2,
            drop_last: false,
            timeout_ms: 0,
            pin_memory: false,
            dynamic_batching: None,
            shuffle: true,
            seed: 42,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicBatchingConfig {
    pub max_tokens_per_batch: usize,
    pub bucket_boundaries: Vec<usize>,
    pub batch_size_tolerance: f64,
    pub pad_to_multiple_of: Option<usize>,
}

impl Default for DynamicBatchingConfig {
    fn default() -> Self {
        Self {
            max_tokens_per_batch: 16384,
            bucket_boundaries: vec![64, 128, 256, 512, 1024, 2048, 4096],
            batch_size_tolerance: 0.25,
            pad_to_multiple_of: Some(8),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSample {
    pub index: usize,
    pub text: Option<String>,
    pub tokens: Option<Vec<u32>>,
    pub label: Option<i64>,
    pub weight: Option<f64>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl DataSample {
    pub fn from_text(index: usize, text: String) -> Self {
        Self {
            index,
            text: Some(text),
            tokens: None,
            label: None,
            weight: None,
            metadata: HashMap::new(),
        }
    }

    pub fn from_tokens(index: usize, tokens: Vec<u32>) -> Self {
        Self {
            index,
            text: None,
            tokens: Some(tokens),
            label: None,
            weight: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_label(mut self, label: i64) -> Self {
        self.label = Some(label);
        self
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
        self
    }

    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataBatch {
    pub batch_index: usize,
    pub size: usize,
    pub input_ids: Vec<Vec<u32>>,
    pub attention_mask: Vec<Vec<u8>>,
    pub labels: Option<Vec<Vec<i64>>>,
    pub weights: Option<Vec<f64>>,
    pub sample_indices: Vec<usize>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub load_time_ms: f64,
    pub tokenize_time_ms: f64,
    pub collate_time_ms: f64,
}

impl DataBatch {
    pub fn num_tokens(&self) -> usize {
        self.input_ids.iter().map(|ids| ids.len()).sum()
    }

    pub fn max_sequence_length(&self) -> usize {
        self.input_ids.iter().map(|ids| ids.len()).max().unwrap_or(0)
    }

    pub fn padding_ratio(&self) -> f64 {
        let max_len = self.max_sequence_length();
        if max_len == 0 {
            return 0.0;
        }
        let total_capacity = self.size * max_len;
        let total_tokens = self.num_tokens();
        1.0 - (total_tokens as f64 / total_capacity as f64)
    }
}

pub trait SampleDataset: Send + Sync {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn get(&self, index: usize) -> Result<DataSample, String>;
    fn column_names(&self) -> Vec<String>;
    fn dataset_name(&self) -> &str;
}

pub struct TextFileDataset {
    name: String,
    lines: Vec<String>,
}

impl TextFileDataset {
    pub fn new(name: &str, lines: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            lines,
        }
    }
}

impl SampleDataset for TextFileDataset {
    fn len(&self) -> usize {
        self.lines.len()
    }

    fn get(&self, index: usize) -> Result<DataSample, String> {
        if index >= self.lines.len() {
            return Err(format!("Index {} out of bounds (len={})", index, self.lines.len()));
        }
        Ok(DataSample::from_text(index, self.lines[index].clone()))
    }

    fn column_names(&self) -> Vec<String> {
        vec!["text".to_string()]
    }

    fn dataset_name(&self) -> &str {
        &self.name
    }
}

pub struct TokenizedDataset {
    name: String,
    input_ids: Vec<Vec<u32>>,
    attention_mask: Vec<Vec<u8>>,
    labels: Option<Vec<Vec<i64>>>,
}

impl TokenizedDataset {
    pub fn new(
        name: &str,
        input_ids: Vec<Vec<u32>>,
        attention_mask: Vec<Vec<u8>>,
        labels: Option<Vec<Vec<i64>>>,
    ) -> Self {
        Self {
            name: name.to_string(),
            input_ids,
            attention_mask,
            labels,
        }
    }

    pub fn from_batch_encoding(name: &str, encoding: &BatchEncoding) -> Self {
        let labels = if let Some(ref type_ids) = encoding.token_type_ids {
            Some(type_ids.iter().map(|v| v.iter().map(|&x| x as i64).collect()).collect())
        } else {
            None
        };

        Self {
            name: name.to_string(),
            input_ids: encoding.input_ids.clone(),
            attention_mask: encoding.attention_mask.clone(),
            labels,
        }
    }
}

impl SampleDataset for TokenizedDataset {
    fn len(&self) -> usize {
        self.input_ids.len()
    }

    fn get(&self, index: usize) -> Result<DataSample, String> {
        if index >= self.input_ids.len() {
            return Err(format!("Index {} out of bounds (len={})", index, self.input_ids.len()));
        }

        let mut sample = DataSample::from_tokens(index, self.input_ids[index].clone());

        if let Some(ref labels) = self.labels {
            if index < labels.len() {
                sample = sample.with_label(labels[index].first().copied().unwrap_or(-100));
            }
        }

        Ok(sample)
    }

    fn column_names(&self) -> Vec<String> {
        vec!["input_ids".to_string(), "attention_mask".to_string()]
    }

    fn dataset_name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerStatus {
    Idle,
    Loading,
    Tokenizing,
    Done,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    pub worker_id: usize,
    pub status: WorkerStatus,
    pub samples_loaded: usize,
    pub batches_produced: usize,
    pub avg_load_time_ms: f64,
    pub avg_tokenize_time_ms: f64,
    pub last_error: Option<String>,
}

enum WorkerCommand {
    LoadBatch {
        indices: Vec<usize>,
        batch_index: usize,
    },
    Shutdown,
}

enum WorkerResult {
    BatchReady(DataBatch),
    BatchError {
        batch_index: usize,
        error: String,
    },
    WorkerStopped(usize),
}

struct WorkerState {
    worker_id: usize,
    dataset: Arc<dyn SampleDataset>,
    tokenizer: Option<Arc<TokenizerPipeline>>,
    collator: Option<Arc<DataCollator>>,
    dynamic_batching: Option<DynamicBatchingConfig>,
    pin_memory: bool,
}

pub struct DataLoader {
    config: DataLoaderConfig,
    dataset: Arc<dyn SampleDataset>,
    tokenizer: Option<Arc<TokenizerPipeline>>,
    collator: Option<Arc<DataCollator>>,
    batch_sampler: Arc<Mutex<BatchSampler>>,
    distributed_sampler: Option<Arc<Mutex<DistributedSampler>>>,
    command_senders: Vec<mpsc::Sender<WorkerCommand>>,
    result_receiver: mpsc::Receiver<WorkerResult>,
    worker_handles: Vec<thread::JoinHandle<()>>,
    worker_infos: Arc<Mutex<Vec<WorkerInfo>>>,
    is_active: Arc<AtomicBool>,
    epoch: Arc<AtomicUsize>,
    batches_yielded: Arc<AtomicUsize>,
    samples_yielded: Arc<AtomicUsize>,
    total_load_time_ms: Arc<AtomicUsize>,
    total_tokenize_time_ms: Arc<AtomicUsize>,
    total_collate_time_ms: Arc<AtomicUsize>,
    start_time: Instant,
}

impl DataLoader {
    pub fn new(
        dataset: Arc<dyn SampleDataset>,
        config: DataLoaderConfig,
    ) -> Self {
        let total_samples = dataset.len();
        let _num_batches = if config.drop_last {
            total_samples / config.batch_size
        } else {
            (total_samples + config.batch_size - 1) / config.batch_size
        };

        let indices: Vec<usize> = (0..total_samples).collect();
        let batch_sampler = Arc::new(Mutex::new(BatchSampler::new(
            indices,
            config.batch_size,
            config.drop_last,
        )));

        let (cmd_senders, result_receiver, worker_handles, worker_infos) =
            Self::spawn_workers(config.num_workers);

        Self {
            config,
            dataset,
            tokenizer: None,
            collator: None,
            batch_sampler,
            distributed_sampler: None,
            command_senders: cmd_senders,
            result_receiver,
            worker_handles,
            worker_infos,
            is_active: Arc::new(AtomicBool::new(false)),
            epoch: Arc::new(AtomicUsize::new(0)),
            batches_yielded: Arc::new(AtomicUsize::new(0)),
            samples_yielded: Arc::new(AtomicUsize::new(0)),
            total_load_time_ms: Arc::new(AtomicUsize::new(0)),
            total_tokenize_time_ms: Arc::new(AtomicUsize::new(0)),
            total_collate_time_ms: Arc::new(AtomicUsize::new(0)),
            start_time: Instant::now(),
        }
    }

    pub fn with_tokenizer(mut self, tokenizer: TokenizerPipeline) -> Self {
        self.tokenizer = Some(Arc::new(tokenizer));
        self
    }

    pub fn with_collator(mut self, collator: DataCollator) -> Self {
        self.collator = Some(Arc::new(collator));
        self
    }

    pub fn with_distributed_sampler(
        mut self,
        num_replicas: usize,
        rank: usize,
    ) -> Self {
        let sampler_config = DistributedSamplerConfig {
            num_replicas,
            rank,
            shuffle: self.config.shuffle,
            seed: self.config.seed,
            drop_last: self.config.drop_last,
        };
        let sampler = DistributedSampler::new(self.dataset.len(), sampler_config);
        self.distributed_sampler = Some(Arc::new(Mutex::new(sampler)));
        self
    }

    fn spawn_workers(
        num_workers: usize,
    ) -> (
        Vec<mpsc::Sender<WorkerCommand>>,
        mpsc::Receiver<WorkerResult>,
        Vec<thread::JoinHandle<()>>,
        Arc<Mutex<Vec<WorkerInfo>>>,
    ) {
        let (result_tx, result_rx) = mpsc::channel();
        let mut cmd_senders = Vec::new();
        let mut handles = Vec::new();
        let worker_infos = Arc::new(Mutex::new(Vec::new()));

        for worker_id in 0..num_workers {
            let (cmd_tx, cmd_rx) = mpsc::channel();
            cmd_senders.push(cmd_tx);

            let result_tx_clone = result_tx.clone();
            let worker_infos_clone = worker_infos.clone();

            let info = WorkerInfo {
                worker_id,
                status: WorkerStatus::Idle,
                samples_loaded: 0,
                batches_produced: 0,
                avg_load_time_ms: 0.0,
                avg_tokenize_time_ms: 0.0,
                last_error: None,
            };
            worker_infos.lock().unwrap().push(info);

            let handle = thread::spawn(move || {
                Self::worker_loop(worker_id, cmd_rx, result_tx_clone, worker_infos_clone);
            });
            handles.push(handle);
        }

        (cmd_senders, result_rx, handles, worker_infos)
    }

    fn worker_loop(
        worker_id: usize,
        cmd_rx: mpsc::Receiver<WorkerCommand>,
        result_tx: mpsc::Sender<WorkerResult>,
        worker_infos: Arc<Mutex<Vec<WorkerInfo>>>,
    ) {
        let dataset: Option<Arc<dyn SampleDataset>> = None;
        let tokenizer: Option<Arc<TokenizerPipeline>> = None;
        let _collator: Option<Arc<DataCollator>> = None;
        let dynamic_batching: Option<DynamicBatchingConfig> = None;
        let _pin_memory = false;
        let mut samples_loaded = 0usize;
        let mut batches_produced = 0usize;
        let mut total_load_time = Duration::ZERO;
        let mut total_tokenize_time = Duration::ZERO;

        for cmd in cmd_rx {
            match cmd {
                WorkerCommand::LoadBatch { indices, batch_index } => {
                    if let Some(ref ds) = dataset {
                        let load_start = Instant::now();

                        let mut samples = Vec::with_capacity(indices.len());
                        for &idx in &indices {
                            match ds.get(idx) {
                                Ok(sample) => samples.push(sample),
                                Err(e) => {
                                    let _ = result_tx.send(WorkerResult::BatchError {
                                        batch_index,
                                        error: format!("Worker {}: {}", worker_id, e),
                                    });
                                    break;
                                }
                            }
                        }

                        if samples.len() != indices.len() {
                            continue;
                        }

                        let load_time = load_start.elapsed();
                        total_load_time += load_time;
                        samples_loaded += samples.len();

                        let tokenize_start = Instant::now();
                        let mut tokenize_time = Duration::ZERO;

                        let batch = if let Some(ref tok) = tokenizer {
                            let texts: Vec<String> = samples.iter()
                            .filter_map(|s| s.text.clone())
                            .collect();

                        if !texts.is_empty() && texts.len() == samples.len() {
                            match tok.encode_batch(&texts, None) {
                                    Ok(encoding) => {
                                        tokenize_time = tokenize_start.elapsed();
                                        total_tokenize_time += tokenize_time;

                                        let mut input_ids = encoding.input_ids.clone();
                                        let mut attention_mask = encoding.attention_mask.clone();

                                        if let Some(ref db) = dynamic_batching {
                                            let (ids, mask) = Self::apply_dynamic_batching(
                                                &input_ids, &attention_mask, db,
                                            );
                                            input_ids = ids;
                                            attention_mask = mask;
                                        }

                                        let labels: Option<Vec<Vec<i64>>> = samples.iter()
                                            .map(|s| s.label.map(|l| vec![l]))
                                            .collect::<Option<Vec<_>>>();

                                        let weights: Option<Vec<f64>> = samples.iter()
                                            .map(|s| s.weight)
                                            .collect::<Option<Vec<_>>>();

                                        let sample_indices: Vec<usize> = samples.iter()
                                            .map(|s| s.index)
                                            .collect();

                                        DataBatch {
                                            batch_index,
                                            size: input_ids.len(),
                                            input_ids,
                                            attention_mask,
                                            labels,
                                            weights,
                                            sample_indices,
                                            metadata: HashMap::new(),
                                            load_time_ms: load_time.as_secs_f64() * 1000.0,
                                            tokenize_time_ms: tokenize_time.as_secs_f64() * 1000.0,
                                            collate_time_ms: 0.0,
                                        }
                                    }
                                    Err(e) => {
                                        let _ = result_tx.send(WorkerResult::BatchError {
                                            batch_index,
                                            error: format!("Tokenize error: {}", e),
                                        });
                                        continue;
                                    }
                                }
                            } else {
                                let mut input_ids: Vec<Vec<u32>> = samples.iter()
                                    .filter_map(|s| s.tokens.clone())
                                    .collect();
                                let attention_mask: Vec<Vec<u8>> = input_ids.iter()
                                    .map(|ids| vec![1u8; ids.len()])
                                    .collect();

                                if let Some(ref db) = dynamic_batching {
                                    let (ids, _mask) = Self::apply_dynamic_batching(
                                        &input_ids, &attention_mask, db,
                                    );
                                    input_ids = ids;
                                }

                                let labels: Option<Vec<Vec<i64>>> = samples.iter()
                                    .map(|s| s.label.map(|l| vec![l]))
                                    .collect::<Option<Vec<_>>>();

                                let weights: Option<Vec<f64>> = samples.iter()
                                    .map(|s| s.weight)
                                    .collect::<Option<Vec<_>>>();

                                let sample_indices: Vec<usize> = samples.iter()
                                    .map(|s| s.index)
                                    .collect();

                                DataBatch {
                                    batch_index,
                                    size: input_ids.len(),
                                    input_ids,
                                    attention_mask,
                                    labels,
                                    weights,
                                    sample_indices,
                                    metadata: HashMap::new(),
                                    load_time_ms: load_time.as_secs_f64() * 1000.0,
                                    tokenize_time_ms: tokenize_time.as_secs_f64() * 1000.0,
                                    collate_time_ms: 0.0,
                                }
                            }
                        } else {
                            let input_ids: Vec<Vec<u32>> = samples.iter()
                                .filter_map(|s| s.tokens.clone())
                                .collect();
                            let attention_mask: Vec<Vec<u8>> = input_ids.iter()
                                .map(|ids| vec![1u8; ids.len()])
                                .collect();

                            let labels: Option<Vec<Vec<i64>>> = samples.iter()
                                .map(|s| s.label.map(|l| vec![l]))
                                .collect::<Option<Vec<_>>>();

                            let weights: Option<Vec<f64>> = samples.iter()
                                .map(|s| s.weight)
                                .collect::<Option<Vec<_>>>();

                            let sample_indices: Vec<usize> = samples.iter()
                                .map(|s| s.index)
                                .collect();

                            DataBatch {
                                batch_index,
                                size: input_ids.len(),
                                input_ids,
                                attention_mask,
                                labels,
                                weights,
                                sample_indices,
                                metadata: HashMap::new(),
                                load_time_ms: load_time.as_secs_f64() * 1000.0,
                                tokenize_time_ms: 0.0,
                                collate_time_ms: 0.0,
                            }
                        };

                        batches_produced += 1;
                        let _ = result_tx.send(WorkerResult::BatchReady(batch));
                    }
                }
                WorkerCommand::Shutdown => {
                    let _ = result_tx.send(WorkerResult::WorkerStopped(worker_id));
                    break;
                }
            }
        }

        if let Ok(mut infos) = worker_infos.lock() {
            if worker_id < infos.len() {
                infos[worker_id].samples_loaded = samples_loaded;
                infos[worker_id].batches_produced = batches_produced;
                infos[worker_id].avg_load_time_ms = if samples_loaded > 0 {
                    total_load_time.as_secs_f64() * 1000.0 / samples_loaded as f64
                } else {
                    0.0
                };
                infos[worker_id].avg_tokenize_time_ms = if batches_produced > 0 {
                    total_tokenize_time.as_secs_f64() * 1000.0 / batches_produced as f64
                } else {
                    0.0
                };
            }
        }
    }

    fn apply_dynamic_batching(
        input_ids: &[Vec<u32>],
        attention_mask: &[Vec<u8>],
        config: &DynamicBatchingConfig,
    ) -> (Vec<Vec<u32>>, Vec<Vec<u8>>) {
        let mut bucketed: HashMap<usize, Vec<usize>> = HashMap::new();

        for (i, ids) in input_ids.iter().enumerate() {
            let len = ids.len();
            let mut bucket = config.bucket_boundaries[config.bucket_boundaries.len() - 1];
            for &boundary in &config.bucket_boundaries {
                if len <= boundary {
                    bucket = boundary;
                    break;
                }
            }
            bucketed.entry(bucket).or_default().push(i);
        }

        let mut result_ids = Vec::new();
        let mut result_mask = Vec::new();

        for (&bucket_size, indices) in &bucketed {
            let max_samples_per_batch = (config.max_tokens_per_batch / bucket_size).max(1);
            let tolerance = (max_samples_per_batch as f64 * config.batch_size_tolerance) as usize;
            let effective_max = max_samples_per_batch + tolerance;

            for chunk in indices.chunks(effective_max) {
                let max_len = chunk.iter()
                    .map(|&i| input_ids[i].len())
                    .max()
                    .unwrap_or(bucket_size);

                let padded_len = match config.pad_to_multiple_of {
                    Some(m) if m > 0 => ((max_len + m - 1) / m) * m,
                    _ => max_len,
                };

                for &i in chunk {
                    let mut ids = input_ids[i].clone();
                    let mut mask = attention_mask[i].clone();

                    ids.resize(padded_len, 0);
                    mask.resize(padded_len, 0);

                    result_ids.push(ids);
                    result_mask.push(mask);
                }
            }
        }

        (result_ids, result_mask)
    }

    pub fn set_epoch(&self, epoch: usize) {
        self.epoch.store(epoch, Ordering::Relaxed);
        if let Some(ref sampler) = self.distributed_sampler {
            if let Ok(mut s) = sampler.lock() {
                s.set_epoch(epoch);
            }
        }
    }

    pub fn len(&self) -> usize {
        if let Some(ref sampler) = self.distributed_sampler {
            if let Ok(s) = sampler.lock() {
                let total = s.len();
                if self.config.drop_last {
                    total / self.config.batch_size
                } else {
                    (total + self.config.batch_size - 1) / self.config.batch_size
                }
            } else {
                0
            }
        } else if let Ok(sampler) = self.batch_sampler.lock() {
            sampler.len()
        } else {
            0
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn stats(&self) -> DataLoaderStats {
        let batches = self.batches_yielded.load(Ordering::Relaxed);
        let samples = self.samples_yielded.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed();

        DataLoaderStats {
            epoch: self.epoch.load(Ordering::Relaxed),
            batches_yielded: batches,
            samples_yielded: samples,
            total_load_time_ms: self.total_load_time_ms.load(Ordering::Relaxed) as f64,
            total_tokenize_time_ms: self.total_tokenize_time_ms.load(Ordering::Relaxed) as f64,
            total_collate_time_ms: self.total_collate_time_ms.load(Ordering::Relaxed) as f64,
            throughput_samples_per_sec: if elapsed.as_secs_f64() > 0.0 {
                samples as f64 / elapsed.as_secs_f64()
            } else {
                0.0
            },
            throughput_batches_per_sec: if elapsed.as_secs_f64() > 0.0 {
                batches as f64 / elapsed.as_secs_f64()
            } else {
                0.0
            },
            num_workers: self.config.num_workers,
            worker_infos: self.worker_infos.lock().unwrap().clone(),
        }
    }

    pub fn worker_infos(&self) -> Vec<WorkerInfo> {
        self.worker_infos.lock().unwrap().clone()
    }

    pub fn shutdown(self) {
        for sender in &self.command_senders {
            let _ = sender.send(WorkerCommand::Shutdown);
        }
    }
}

impl Iterator for DataLoader {
    type Item = DataBatch;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.is_active.load(Ordering::Relaxed) {
            self.is_active.store(true, Ordering::Relaxed);
        }

        if self.config.num_workers > 0 {
            self.next_with_workers()
        } else {
            self.next_single_threaded()
        }
    }
}

impl DataLoader {
    fn next_single_threaded(&mut self) -> Option<DataBatch> {
        let indices: Vec<usize> = if let Some(ref sampler) = self.distributed_sampler {
            if let Ok(s) = sampler.lock() {
                s.iter().collect()
            } else {
                return None;
            }
        } else if let Ok(sampler) = self.batch_sampler.lock() {
            let mut iter = sampler.iter();
            match iter.next() {
                Some(indices) => indices,
                None => return None,
            }
        } else {
            return None;
        };

        if indices.is_empty() {
            return None;
        }

        let load_start = Instant::now();
        let mut samples = Vec::with_capacity(indices.len());
        for &idx in &indices {
            match self.dataset.get(idx) {
                Ok(sample) => samples.push(sample),
                Err(_) => continue,
            }
        }

        if samples.is_empty() {
            return None;
        }

        let load_time = load_start.elapsed();

        let tokenize_start = Instant::now();
        let mut tokenize_time = Duration::ZERO;

        let batch = if let Some(ref tokenizer) = self.tokenizer {
            let texts: Vec<String> = samples.iter()
                .filter_map(|s| s.text.clone())
                .collect();

            if !texts.is_empty() && texts.len() == samples.len() {
                match tokenizer.encode_batch(&texts, None) {
                    Ok(encoding) => {
                        tokenize_time = tokenize_start.elapsed();

                        let mut input_ids = encoding.input_ids.clone();
                        let mut attention_mask = encoding.attention_mask.clone();

                        if let Some(ref db) = self.config.dynamic_batching {
                            let (ids, mask) = Self::apply_dynamic_batching(
                                &input_ids, &attention_mask, db,
                            );
                            input_ids = ids;
                            attention_mask = mask;
                        }

                        let labels: Option<Vec<Vec<i64>>> = samples.iter()
                            .map(|s| s.label.map(|l| vec![l]))
                            .collect::<Option<Vec<_>>>();

                        let weights: Option<Vec<f64>> = samples.iter()
                            .map(|s| s.weight)
                            .collect::<Option<Vec<_>>>();

                        let sample_indices: Vec<usize> = samples.iter()
                            .map(|s| s.index)
                            .collect();

                        DataBatch {
                            batch_index: self.batches_yielded.load(Ordering::Relaxed),
                            size: input_ids.len(),
                            input_ids,
                            attention_mask,
                            labels,
                            weights,
                            sample_indices,
                            metadata: HashMap::new(),
                            load_time_ms: load_time.as_secs_f64() * 1000.0,
                            tokenize_time_ms: tokenize_time.as_secs_f64() * 1000.0,
                            collate_time_ms: 0.0,
                        }
                    }
                    Err(_) => return None,
                }
            } else {
                let input_ids: Vec<Vec<u32>> = samples.iter()
                    .filter_map(|s| s.tokens.clone())
                    .collect();
                let attention_mask: Vec<Vec<u8>> = input_ids.iter()
                    .map(|ids| vec![1u8; ids.len()])
                    .collect();

                let labels: Option<Vec<Vec<i64>>> = samples.iter()
                    .map(|s| s.label.map(|l| vec![l]))
                    .collect::<Option<Vec<_>>>();

                let weights: Option<Vec<f64>> = samples.iter()
                    .map(|s| s.weight)
                    .collect::<Option<Vec<_>>>();

                let sample_indices: Vec<usize> = samples.iter()
                    .map(|s| s.index)
                    .collect();

                DataBatch {
                    batch_index: self.batches_yielded.load(Ordering::Relaxed),
                    size: input_ids.len(),
                    input_ids,
                    attention_mask,
                    labels,
                    weights,
                    sample_indices,
                    metadata: HashMap::new(),
                    load_time_ms: load_time.as_secs_f64() * 1000.0,
                    tokenize_time_ms: 0.0,
                    collate_time_ms: 0.0,
                }
            }
        } else {
            let input_ids: Vec<Vec<u32>> = samples.iter()
                .filter_map(|s| s.tokens.clone())
                .collect();
            let attention_mask: Vec<Vec<u8>> = input_ids.iter()
                .map(|ids| vec![1u8; ids.len()])
                .collect();

            let labels: Option<Vec<Vec<i64>>> = samples.iter()
                .map(|s| s.label.map(|l| vec![l]))
                .collect::<Option<Vec<_>>>();

            let weights: Option<Vec<f64>> = samples.iter()
                .map(|s| s.weight)
                .collect::<Option<Vec<_>>>();

            let sample_indices: Vec<usize> = samples.iter()
                .map(|s| s.index)
                .collect();

            DataBatch {
                batch_index: self.batches_yielded.load(Ordering::Relaxed),
                size: input_ids.len(),
                input_ids,
                attention_mask,
                labels,
                weights,
                sample_indices,
                metadata: HashMap::new(),
                load_time_ms: load_time.as_secs_f64() * 1000.0,
                tokenize_time_ms: 0.0,
                collate_time_ms: 0.0,
            }
        };

        self.batches_yielded.fetch_add(1, Ordering::Relaxed);
        self.samples_yielded.fetch_add(batch.size, Ordering::Relaxed);
        self.total_load_time_ms.fetch_add(batch.load_time_ms as usize, Ordering::Relaxed);
        self.total_tokenize_time_ms.fetch_add(batch.tokenize_time_ms as usize, Ordering::Relaxed);

        Some(batch)
    }

    fn next_with_workers(&mut self) -> Option<DataBatch> {
        let indices: Vec<usize> = if let Some(ref sampler) = self.distributed_sampler {
            if let Ok(s) = sampler.lock() {
                s.iter().collect()
            } else {
                return None;
            }
        } else if let Ok(sampler) = self.batch_sampler.lock() {
            let mut iter = sampler.iter();
            match iter.next() {
                Some(indices) => indices,
                None => return None,
            }
        } else {
            return None;
        };

        if indices.is_empty() {
            return None;
        }

        let batch_index = self.batches_yielded.load(Ordering::Relaxed);
        let worker_idx = batch_index % self.config.num_workers;

        if worker_idx < self.command_senders.len() {
            let _ = self.command_senders[worker_idx].send(WorkerCommand::LoadBatch {
                indices,
                batch_index,
            });
        }

        match self.result_receiver.recv_timeout(
            Duration::from_millis(if self.config.timeout_ms > 0 {
                self.config.timeout_ms
            } else {
                60000
            }),
        ) {
            Ok(WorkerResult::BatchReady(batch)) => {
                self.batches_yielded.fetch_add(1, Ordering::Relaxed);
                self.samples_yielded.fetch_add(batch.size, Ordering::Relaxed);
                self.total_load_time_ms.fetch_add(batch.load_time_ms as usize, Ordering::Relaxed);
                self.total_tokenize_time_ms.fetch_add(batch.tokenize_time_ms as usize, Ordering::Relaxed);
                self.total_collate_time_ms.fetch_add(batch.collate_time_ms as usize, Ordering::Relaxed);
                Some(batch)
            }
            Ok(WorkerResult::BatchError { error, .. }) => {
                eprintln!("DataLoader worker error: {}", error);
                None
            }
            Ok(WorkerResult::WorkerStopped(_)) => None,
            Err(_) => None,
        }
    }
}

impl Drop for DataLoader {
    fn drop(&mut self) {
        for sender in &self.command_senders {
            let _ = sender.send(WorkerCommand::Shutdown);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLoaderStats {
    pub epoch: usize,
    pub batches_yielded: usize,
    pub samples_yielded: usize,
    pub total_load_time_ms: f64,
    pub total_tokenize_time_ms: f64,
    pub total_collate_time_ms: f64,
    pub throughput_samples_per_sec: f64,
    pub throughput_batches_per_sec: f64,
    pub num_workers: usize,
    pub worker_infos: Vec<WorkerInfo>,
}

impl DataLoaderStats {
    pub fn utilization(&self) -> f64 {
        if self.total_load_time_ms + self.total_tokenize_time_ms + self.total_collate_time_ms > 0.0 {
            let total = self.total_load_time_ms + self.total_tokenize_time_ms + self.total_collate_time_ms;
            self.total_tokenize_time_ms / total
        } else {
            0.0
        }
    }

    pub fn bottleneck(&self) -> &str {
        if self.total_load_time_ms > self.total_tokenize_time_ms
            && self.total_load_time_ms > self.total_collate_time_ms
        {
            "data_loading"
        } else if self.total_tokenize_time_ms > self.total_collate_time_ms {
            "tokenization"
        } else {
            "collation"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_sample_creation() {
        let sample = DataSample::from_text(0, "hello world".to_string());
        assert_eq!(sample.index, 0);
        assert_eq!(sample.text, Some("hello world".to_string()));
        assert!(sample.tokens.is_none());

        let sample = DataSample::from_tokens(1, vec![1, 2, 3]);
        assert_eq!(sample.index, 1);
        assert_eq!(sample.tokens, Some(vec![1, 2, 3]));
        assert!(sample.text.is_none());
    }

    #[test]
    fn test_data_sample_with_label() {
        let sample = DataSample::from_text(0, "test".to_string())
            .with_label(1)
            .with_weight(0.5);
        assert_eq!(sample.label, Some(1));
        assert_eq!(sample.weight, Some(0.5));
    }

    #[test]
    fn test_text_file_dataset() {
        let lines = vec![
            "line one".to_string(),
            "line two".to_string(),
            "line three".to_string(),
        ];
        let dataset = TextFileDataset::new("test", lines);

        assert_eq!(dataset.len(), 3);
        assert!(!dataset.is_empty());
        assert_eq!(dataset.dataset_name(), "test");
        assert_eq!(dataset.column_names(), vec!["text"]);

        let sample = dataset.get(0).unwrap();
        assert_eq!(sample.text, Some("line one".to_string()));

        let sample = dataset.get(2).unwrap();
        assert_eq!(sample.text, Some("line three".to_string()));

        assert!(dataset.get(3).is_err());
    }

    #[test]
    fn test_tokenized_dataset() {
        let input_ids = vec![
            vec![1, 2, 3],
            vec![4, 5],
            vec![6, 7, 8, 9],
        ];
        let attention_mask = vec![
            vec![1, 1, 1],
            vec![1, 1],
            vec![1, 1, 1, 1],
        ];
        let labels = Some(vec![
            vec![0],
            vec![1],
            vec![0],
        ]);

        let dataset = TokenizedDataset::new("tokens", input_ids, attention_mask, labels);

        assert_eq!(dataset.len(), 3);
        assert_eq!(dataset.dataset_name(), "tokens");

        let sample = dataset.get(0).unwrap();
        assert_eq!(sample.tokens, Some(vec![1, 2, 3]));
        assert_eq!(sample.label, Some(0));

        let sample = dataset.get(1).unwrap();
        assert_eq!(sample.tokens, Some(vec![4, 5]));
        assert_eq!(sample.label, Some(1));
    }

    #[test]
    fn test_data_loader_single_threaded() {
        let lines: Vec<String> = (0..100).map(|i| format!("sample {}", i)).collect();
        let dataset = Arc::new(TextFileDataset::new("test", lines));

        let config = DataLoaderConfig {
            batch_size: 10,
            num_workers: 0,
            prefetch_factor: 1,
            drop_last: false,
            timeout_ms: 0,
            pin_memory: false,
            dynamic_batching: None,
            shuffle: false,
            seed: 42,
        };

        let loader = DataLoader::new(dataset, config);
        assert_eq!(loader.len(), 10);

        let batches: Vec<DataBatch> = loader.collect();
        assert_eq!(batches.len(), 10);
        assert_eq!(batches[0].size, 10);
        assert_eq!(batches[9].size, 10);
    }

    #[test]
    fn test_data_loader_drop_last() {
        let lines: Vec<String> = (0..95).map(|i| format!("sample {}", i)).collect();
        let dataset = Arc::new(TextFileDataset::new("test", lines));

        let config = DataLoaderConfig {
            batch_size: 10,
            num_workers: 0,
            prefetch_factor: 1,
            drop_last: true,
            timeout_ms: 0,
            pin_memory: false,
            dynamic_batching: None,
            shuffle: false,
            seed: 42,
        };

        let loader = DataLoader::new(dataset, config);
        assert_eq!(loader.len(), 9);

        let batches: Vec<DataBatch> = loader.collect();
        assert_eq!(batches.len(), 9);
    }

    #[test]
    fn test_data_batch_stats() {
        let batch = DataBatch {
            batch_index: 0,
            size: 3,
            input_ids: vec![vec![1, 2, 3], vec![4, 5], vec![6, 7, 8, 9]],
            attention_mask: vec![vec![1, 1, 1], vec![1, 1], vec![1, 1, 1, 1]],
            labels: None,
            weights: None,
            sample_indices: vec![0, 1, 2],
            metadata: HashMap::new(),
            load_time_ms: 1.0,
            tokenize_time_ms: 2.0,
            collate_time_ms: 0.5,
        };

        assert_eq!(batch.num_tokens(), 9);
        assert_eq!(batch.max_sequence_length(), 4);
        assert!(batch.padding_ratio() > 0.0);
    }

    #[test]
    fn test_dynamic_batching() {
        let input_ids = vec![
            vec![1; 50],
            vec![2; 60],
            vec![3; 120],
            vec![4; 130],
            vec![5; 500],
            vec![6; 510],
        ];
        let attention_mask: Vec<Vec<u8>> = input_ids.iter()
            .map(|ids| vec![1u8; ids.len()])
            .collect();

        let config = DynamicBatchingConfig::default();
        let (result_ids, result_mask) = DataLoader::apply_dynamic_batching(
            &input_ids, &attention_mask, &config,
        );

        assert!(!result_ids.is_empty());
        assert_eq!(result_ids.len(), result_mask.len());
    }

    #[test]
    fn test_data_loader_stats() {
        let stats = DataLoaderStats {
            epoch: 0,
            batches_yielded: 100,
            samples_yielded: 3200,
            total_load_time_ms: 5000.0,
            total_tokenize_time_ms: 3000.0,
            total_collate_time_ms: 1000.0,
            throughput_samples_per_sec: 640.0,
            throughput_batches_per_sec: 20.0,
            num_workers: 4,
            worker_infos: vec![],
        };

        assert_eq!(stats.bottleneck(), "data_loading");
        assert!(stats.utilization() > 0.0);
    }
}
