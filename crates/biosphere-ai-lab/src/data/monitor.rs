use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    pub window_size: usize,
    pub alert_thresholds: AlertThresholds,
    pub sampling_rate: f64,
    pub enable_token_stats: bool,
    pub enable_sequence_stats: bool,
    pub enable_label_stats: bool,
    pub enable_performance_stats: bool,
    pub log_interval_seconds: u64,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            window_size: 1000,
            alert_thresholds: AlertThresholds::default(),
            sampling_rate: 1.0,
            enable_token_stats: true,
            enable_sequence_stats: true,
            enable_label_stats: true,
            enable_performance_stats: true,
            log_interval_seconds: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub max_empty_ratio: f64,
    pub max_oov_ratio: f64,
    pub max_truncation_ratio: f64,
    pub min_avg_sequence_length: usize,
    pub max_avg_sequence_length: usize,
    pub max_label_imbalance_ratio: f64,
    pub max_throughput_drop_ratio: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            max_empty_ratio: 0.05,
            max_oov_ratio: 0.1,
            max_truncation_ratio: 0.3,
            min_avg_sequence_length: 10,
            max_avg_sequence_length: 4096,
            max_label_imbalance_ratio: 10.0,
            max_throughput_drop_ratio: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStats {
    pub total_tokens: usize,
    pub padding_tokens: usize,
    pub oov_tokens: usize,
    pub special_tokens: usize,
    pub avg_tokens_per_sample: f64,
    pub max_tokens_in_batch: usize,
    pub min_tokens_in_batch: usize,
    pub oov_ratio: f64,
    pub padding_ratio: f64,
}

impl Default for TokenStats {
    fn default() -> Self {
        Self {
            total_tokens: 0,
            padding_tokens: 0,
            oov_tokens: 0,
            special_tokens: 0,
            avg_tokens_per_sample: 0.0,
            max_tokens_in_batch: 0,
            min_tokens_in_batch: usize::MAX,
            oov_ratio: 0.0,
            padding_ratio: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceStats {
    pub total_sequences: usize,
    pub empty_sequences: usize,
    pub truncated_sequences: usize,
    pub avg_sequence_length: f64,
    pub median_sequence_length: f64,
    pub p95_sequence_length: usize,
    pub min_sequence_length: usize,
    pub max_sequence_length: usize,
    pub empty_ratio: f64,
    pub truncation_ratio: f64,
}

impl Default for SequenceStats {
    fn default() -> Self {
        Self {
            total_sequences: 0,
            empty_sequences: 0,
            truncated_sequences: 0,
            avg_sequence_length: 0.0,
            median_sequence_length: 0.0,
            p95_sequence_length: 0,
            min_sequence_length: usize::MAX,
            max_sequence_length: 0,
            empty_ratio: 0.0,
            truncation_ratio: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelStats {
    pub total_samples: usize,
    pub class_distribution: HashMap<usize, usize>,
    pub majority_class: Option<usize>,
    pub minority_class: Option<usize>,
    pub majority_count: usize,
    pub minority_count: usize,
    pub num_classes: usize,
    pub imbalance_ratio: f64,
    pub entropy: f64,
}

impl Default for LabelStats {
    fn default() -> Self {
        Self {
            total_samples: 0,
            class_distribution: HashMap::new(),
            majority_class: None,
            minority_class: None,
            majority_count: 0,
            minority_count: 0,
            num_classes: 0,
            imbalance_ratio: 0.0,
            entropy: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub samples_per_second: f64,
    pub batches_per_second: f64,
    pub tokens_per_second: f64,
    pub avg_batch_load_time_ms: f64,
    pub avg_batch_process_time_ms: f64,
    pub total_samples_processed: usize,
    pub total_batches_processed: usize,
    pub elapsed_seconds: f64,
    pub peak_memory_mb: f64,
    pub current_memory_mb: f64,
}

impl Default for PerformanceStats {
    fn default() -> Self {
        Self {
            samples_per_second: 0.0,
            batches_per_second: 0.0,
            tokens_per_second: 0.0,
            avg_batch_load_time_ms: 0.0,
            avg_batch_process_time_ms: 0.0,
            total_samples_processed: 0,
            total_batches_processed: 0,
            elapsed_seconds: 0.0,
            peak_memory_mb: 0.0,
            current_memory_mb: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQualityAlert {
    pub timestamp: DateTime<Utc>,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub current_value: f64,
    pub threshold_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertType {
    HighEmptyRatio,
    HighOovRatio,
    HighTruncationRatio,
    LowAvgSequenceLength,
    HighAvgSequenceLength,
    HighLabelImbalance,
    ThroughputDrop,
    DataDrift,
    AnomalyDetected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorSnapshot {
    pub timestamp: DateTime<Utc>,
    pub token_stats: TokenStats,
    pub sequence_stats: SequenceStats,
    pub label_stats: LabelStats,
    pub performance_stats: PerformanceStats,
    pub active_alerts: Vec<DataQualityAlert>,
    pub alert_history: Vec<DataQualityAlert>,
    pub window_sample_count: usize,
}

pub struct DataMonitor {
    config: MonitorConfig,
    token_stats: Arc<Mutex<TokenStats>>,
    sequence_stats: Arc<Mutex<SequenceStats>>,
    label_stats: Arc<Mutex<LabelStats>>,
    performance_stats: Arc<Mutex<PerformanceStats>>,
    alerts: Arc<Mutex<Vec<DataQualityAlert>>>,
    alert_history: Arc<Mutex<VecDeque<DataQualityAlert>>>,
    sequence_lengths: Arc<Mutex<VecDeque<usize>>>,
    batch_times: Arc<Mutex<VecDeque<Duration>>>,
    load_times: Arc<Mutex<VecDeque<Duration>>>,
    start_time: Arc<Mutex<Option<Instant>>>,
    is_active: Arc<AtomicBool>,
    total_samples: Arc<AtomicUsize>,
    total_batches: Arc<AtomicUsize>,
    total_tokens: Arc<AtomicUsize>,
    last_log_time: Arc<Mutex<Instant>>,
}

impl DataMonitor {
    pub fn new(config: MonitorConfig) -> Self {
        let window_size = config.window_size;
        Self {
            config,
            token_stats: Arc::new(Mutex::new(TokenStats::default())),
            sequence_stats: Arc::new(Mutex::new(SequenceStats::default())),
            label_stats: Arc::new(Mutex::new(LabelStats::default())),
            performance_stats: Arc::new(Mutex::new(PerformanceStats::default())),
            alerts: Arc::new(Mutex::new(Vec::new())),
            alert_history: Arc::new(Mutex::new(VecDeque::with_capacity(1000))),
            sequence_lengths: Arc::new(Mutex::new(VecDeque::with_capacity(window_size))),
            batch_times: Arc::new(Mutex::new(VecDeque::with_capacity(window_size))),
            load_times: Arc::new(Mutex::new(VecDeque::with_capacity(window_size))),
            start_time: Arc::new(Mutex::new(None)),
            is_active: Arc::new(AtomicBool::new(false)),
            total_samples: Arc::new(AtomicUsize::new(0)),
            total_batches: Arc::new(AtomicUsize::new(0)),
            total_tokens: Arc::new(AtomicUsize::new(0)),
            last_log_time: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub fn start(&self) {
        self.is_active.store(true, Ordering::SeqCst);
        if let Ok(mut start) = self.start_time.lock() {
            *start = Some(Instant::now());
        }
    }

    pub fn stop(&self) {
        self.is_active.store(false, Ordering::SeqCst);
    }

    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::SeqCst)
    }

    pub fn record_batch(
        &self,
        input_ids: &[Vec<u32>],
        attention_mask: &[Vec<u8>],
        labels: Option<&[usize]>,
        load_time: Duration,
        process_time: Duration,
    ) {
        if !self.is_active() {
            return;
        }

        let batch_size = input_ids.len();
        self.total_samples.fetch_add(batch_size, Ordering::Relaxed);
        self.total_batches.fetch_add(1, Ordering::Relaxed);

        let mut total_tokens_in_batch = 0usize;
        let mut padding_count = 0usize;
        let mut max_seq_len = 0usize;
        let mut min_seq_len = usize::MAX;

        for (ids, mask) in input_ids.iter().zip(attention_mask.iter()) {
            let seq_len = ids.len();
            total_tokens_in_batch += seq_len;
            max_seq_len = max_seq_len.max(seq_len);
            min_seq_len = min_seq_len.min(seq_len);

            let active_tokens = mask.iter().filter(|&&m| m == 1).count();
            padding_count += seq_len - active_tokens;

            if active_tokens == 0 {
                if let Ok(mut stats) = self.sequence_stats.lock() {
                    stats.empty_sequences += 1;
                }
            }

            if let Ok(mut lengths) = self.sequence_lengths.lock() {
                if lengths.len() >= self.config.window_size {
                    lengths.pop_front();
                }
                lengths.push_back(active_tokens);
            }
        }

        self.total_tokens.fetch_add(total_tokens_in_batch, Ordering::Relaxed);

        if let Ok(mut stats) = self.token_stats.lock() {
            stats.total_tokens += total_tokens_in_batch;
            stats.padding_tokens += padding_count;
            stats.max_tokens_in_batch = stats.max_tokens_in_batch.max(max_seq_len);
            stats.min_tokens_in_batch = stats.min_tokens_in_batch.min(min_seq_len);
            if stats.total_tokens > 0 {
                stats.padding_ratio = stats.padding_tokens as f64 / stats.total_tokens as f64;
            }
        }

        if let Ok(mut stats) = self.sequence_stats.lock() {
            stats.total_sequences += batch_size;
            stats.max_sequence_length = stats.max_sequence_length.max(max_seq_len);
            stats.min_sequence_length = stats.min_sequence_length.min(min_seq_len);
            if stats.total_sequences > 0 {
                stats.empty_ratio = stats.empty_sequences as f64 / stats.total_sequences as f64;
            }
        }

        if let Some(lbls) = labels {
            if let Ok(mut stats) = self.label_stats.lock() {
                stats.total_samples += batch_size;
                for &label in lbls {
                    *stats.class_distribution.entry(label).or_insert(0) += 1;
                }
                stats.num_classes = stats.class_distribution.len();

                let mut max_count = 0usize;
                let mut min_count = usize::MAX;
                let mut majority_cls = None;
                let mut minority_cls = None;
                for (&cls, &count) in &stats.class_distribution {
                    if count > max_count {
                        max_count = count;
                        majority_cls = Some(cls);
                    }
                    if count < min_count {
                        min_count = count;
                        minority_cls = Some(cls);
                    }
                }
                stats.majority_class = majority_cls;
                stats.minority_class = minority_cls;
                stats.majority_count = max_count;
                stats.minority_count = min_count;
                if min_count > 0 && min_count != usize::MAX {
                    stats.imbalance_ratio = max_count as f64 / min_count as f64;
                }

                let total: f64 = stats.class_distribution.values().sum::<usize>() as f64;
                stats.entropy = stats.class_distribution.values()
                    .map(|&c| {
                        let p = c as f64 / total;
                        if p > 0.0 { -p * p.log2() } else { 0.0 }
                    })
                    .sum();
            }
        }

        {
            if let Ok(mut times) = self.batch_times.lock() {
                if times.len() >= self.config.window_size {
                    times.pop_front();
                }
                times.push_back(process_time);
            }
            if let Ok(mut times) = self.load_times.lock() {
                if times.len() >= self.config.window_size {
                    times.pop_front();
                }
                times.push_back(load_time);
            }
        }

        self.update_performance_stats();
        self.check_alerts();
        self.maybe_log();
    }

    fn update_performance_stats(&self) {
        if let Ok(mut stats) = self.performance_stats.lock() {
            stats.total_samples_processed = self.total_samples.load(Ordering::Relaxed);
            stats.total_batches_processed = self.total_batches.load(Ordering::Relaxed);

            if let Ok(start) = self.start_time.lock() {
                if let Some(start_time) = *start {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    stats.elapsed_seconds = elapsed;
                    if elapsed > 0.0 {
                        stats.samples_per_second = stats.total_samples_processed as f64 / elapsed;
                        stats.batches_per_second = stats.total_batches_processed as f64 / elapsed;
                        stats.tokens_per_second = self.total_tokens.load(Ordering::Relaxed) as f64 / elapsed;
                    }
                }
            }

            if let Ok(times) = self.batch_times.lock() {
                if !times.is_empty() {
                    stats.avg_batch_process_time_ms = times.iter()
                        .map(|d| d.as_secs_f64() * 1000.0)
                        .sum::<f64>() / times.len() as f64;
                }
            }
            if let Ok(times) = self.load_times.lock() {
                if !times.is_empty() {
                    stats.avg_batch_load_time_ms = times.iter()
                        .map(|d| d.as_secs_f64() * 1000.0)
                        .sum::<f64>() / times.len() as f64;
                }
            }
        }
    }

    fn check_alerts(&self) {
        let mut new_alerts = Vec::new();

        if let Ok(seq_stats) = self.sequence_stats.lock() {
            if seq_stats.empty_ratio > self.config.alert_thresholds.max_empty_ratio {
                new_alerts.push(DataQualityAlert {
                    timestamp: Utc::now(),
                    alert_type: AlertType::HighEmptyRatio,
                    severity: AlertSeverity::Warning,
                    message: format!("Empty sequence ratio {:.4} exceeds threshold {:.4}",
                        seq_stats.empty_ratio, self.config.alert_thresholds.max_empty_ratio),
                    current_value: seq_stats.empty_ratio,
                    threshold_value: self.config.alert_thresholds.max_empty_ratio,
                });
            }

            if seq_stats.truncation_ratio > self.config.alert_thresholds.max_truncation_ratio {
                new_alerts.push(DataQualityAlert {
                    timestamp: Utc::now(),
                    alert_type: AlertType::HighTruncationRatio,
                    severity: AlertSeverity::Warning,
                    message: format!("Truncation ratio {:.4} exceeds threshold {:.4}",
                        seq_stats.truncation_ratio, self.config.alert_thresholds.max_truncation_ratio),
                    current_value: seq_stats.truncation_ratio,
                    threshold_value: self.config.alert_thresholds.max_truncation_ratio,
                });
            }
        }

        if let Ok(token_stats) = self.token_stats.lock() {
            if token_stats.oov_ratio > self.config.alert_thresholds.max_oov_ratio {
                new_alerts.push(DataQualityAlert {
                    timestamp: Utc::now(),
                    alert_type: AlertType::HighOovRatio,
                    severity: AlertSeverity::Warning,
                    message: format!("OOV ratio {:.4} exceeds threshold {:.4}",
                        token_stats.oov_ratio, self.config.alert_thresholds.max_oov_ratio),
                    current_value: token_stats.oov_ratio,
                    threshold_value: self.config.alert_thresholds.max_oov_ratio,
                });
            }
        }

        if let Ok(label_stats) = self.label_stats.lock() {
            if label_stats.imbalance_ratio > self.config.alert_thresholds.max_label_imbalance_ratio {
                new_alerts.push(DataQualityAlert {
                    timestamp: Utc::now(),
                    alert_type: AlertType::HighLabelImbalance,
                    severity: AlertSeverity::Warning,
                    message: format!("Label imbalance ratio {:.2} exceeds threshold {:.2}",
                        label_stats.imbalance_ratio, self.config.alert_thresholds.max_label_imbalance_ratio),
                    current_value: label_stats.imbalance_ratio,
                    threshold_value: self.config.alert_thresholds.max_label_imbalance_ratio,
                });
            }
        }

        if !new_alerts.is_empty() {
            if let Ok(mut alerts) = self.alerts.lock() {
                *alerts = new_alerts.clone();
            }
            if let Ok(mut history) = self.alert_history.lock() {
                for alert in new_alerts {
                    if history.len() >= 1000 {
                        history.pop_front();
                    }
                    history.push_back(alert);
                }
            }
        }
    }

    fn maybe_log(&self) {
        if let Ok(mut last_log) = self.last_log_time.lock() {
            if last_log.elapsed().as_secs() >= self.config.log_interval_seconds {
                let snapshot = self.snapshot();
                tracing::info!(
                    "[DataMonitor] samples={} batches={} tokens={} samples/s={:.1} empty_ratio={:.4} alerts={}",
                    snapshot.performance_stats.total_samples_processed,
                    snapshot.performance_stats.total_batches_processed,
                    snapshot.token_stats.total_tokens,
                    snapshot.performance_stats.samples_per_second,
                    snapshot.sequence_stats.empty_ratio,
                    snapshot.active_alerts.len(),
                );
                *last_log = Instant::now();
            }
        }
    }

    pub fn snapshot(&self) -> MonitorSnapshot {
        let token_stats = self.token_stats.lock().unwrap().clone();
        let sequence_stats = self.sequence_stats.lock().unwrap().clone();
        let label_stats = self.label_stats.lock().unwrap().clone();
        let performance_stats = self.performance_stats.lock().unwrap().clone();
        let active_alerts = self.alerts.lock().unwrap().clone();
        let alert_history: Vec<DataQualityAlert> = self.alert_history.lock().unwrap().iter().cloned().collect();

        let window_count = self.sequence_lengths.lock().unwrap().len();

        MonitorSnapshot {
            timestamp: Utc::now(),
            token_stats,
            sequence_stats,
            label_stats,
            performance_stats,
            active_alerts,
            alert_history,
            window_sample_count: window_count,
        }
    }

    pub fn reset(&self) {
        if let Ok(mut stats) = self.token_stats.lock() { *stats = TokenStats::default(); }
        if let Ok(mut stats) = self.sequence_stats.lock() { *stats = SequenceStats::default(); }
        if let Ok(mut stats) = self.label_stats.lock() { *stats = LabelStats::default(); }
        if let Ok(mut stats) = self.performance_stats.lock() { *stats = PerformanceStats::default(); }
        if let Ok(mut alerts) = self.alerts.lock() { alerts.clear(); }
        if let Ok(mut lengths) = self.sequence_lengths.lock() { lengths.clear(); }
        if let Ok(mut times) = self.batch_times.lock() { times.clear(); }
        if let Ok(mut times) = self.load_times.lock() { times.clear(); }
        if let Ok(mut start) = self.start_time.lock() { *start = None; }
        self.total_samples.store(0, Ordering::Relaxed);
        self.total_batches.store(0, Ordering::Relaxed);
        self.total_tokens.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_start_stop() {
        let config = MonitorConfig::default();
        let monitor = DataMonitor::new(config);
        assert!(!monitor.is_active());

        monitor.start();
        assert!(monitor.is_active());

        monitor.stop();
        assert!(!monitor.is_active());
    }

    #[test]
    fn test_record_batch() {
        let config = MonitorConfig::default();
        let monitor = DataMonitor::new(config);
        monitor.start();

        let input_ids = vec![
            vec![1, 2, 3, 4, 5],
            vec![1, 2, 3],
            vec![1, 2, 3, 4, 5, 6, 7],
        ];
        let attention_mask = vec![
            vec![1, 1, 1, 1, 1],
            vec![1, 1, 1],
            vec![1, 1, 1, 1, 1, 1, 1],
        ];
        let labels = vec![0usize, 1, 0];

        monitor.record_batch(
            &input_ids,
            &attention_mask,
            Some(&labels),
            Duration::from_millis(10),
            Duration::from_millis(50),
        );

        let snapshot = monitor.snapshot();
        assert_eq!(snapshot.performance_stats.total_samples_processed, 3);
        assert_eq!(snapshot.performance_stats.total_batches_processed, 1);
        assert_eq!(snapshot.token_stats.total_tokens, 15);
        assert_eq!(snapshot.sequence_stats.total_sequences, 3);
        assert_eq!(snapshot.label_stats.total_samples, 3);
    }

    #[test]
    fn test_record_batch_with_padding() {
        let config = MonitorConfig::default();
        let monitor = DataMonitor::new(config);
        monitor.start();

        let input_ids = vec![
            vec![1, 2, 3, 0, 0],
            vec![1, 2, 0, 0, 0],
        ];
        let attention_mask = vec![
            vec![1, 1, 1, 0, 0],
            vec![1, 1, 0, 0, 0],
        ];

        monitor.record_batch(
            &input_ids,
            &attention_mask,
            None,
            Duration::from_millis(5),
            Duration::from_millis(30),
        );

        let snapshot = monitor.snapshot();
        assert_eq!(snapshot.token_stats.total_tokens, 10);
        assert!(snapshot.token_stats.padding_tokens > 0);
        assert!(snapshot.token_stats.padding_ratio > 0.0);
    }

    #[test]
    fn test_empty_sequence_detection() {
        let config = MonitorConfig::default();
        let monitor = DataMonitor::new(config);
        monitor.start();

        let input_ids = vec![
            vec![0, 0, 0],
            vec![1, 2, 3],
        ];
        let attention_mask = vec![
            vec![0, 0, 0],
            vec![1, 1, 1],
        ];

        monitor.record_batch(
            &input_ids,
            &attention_mask,
            None,
            Duration::from_millis(5),
            Duration::from_millis(30),
        );

        let snapshot = monitor.snapshot();
        assert_eq!(snapshot.sequence_stats.empty_sequences, 1);
        assert!(snapshot.sequence_stats.empty_ratio > 0.0);
    }

    #[test]
    fn test_label_distribution() {
        let config = MonitorConfig::default();
        let monitor = DataMonitor::new(config);
        monitor.start();

        let input_ids = vec![vec![1, 2, 3]; 10];
        let attention_mask = vec![vec![1, 1, 1]; 10];
        let labels: Vec<usize> = vec![0, 0, 0, 0, 0, 0, 0, 1, 1, 2];

        monitor.record_batch(
            &input_ids,
            &attention_mask,
            Some(&labels),
            Duration::from_millis(10),
            Duration::from_millis(50),
        );

        let snapshot = monitor.snapshot();
        assert_eq!(snapshot.label_stats.num_classes, 3);
        assert_eq!(snapshot.label_stats.majority_count, 7);
        assert_eq!(snapshot.label_stats.minority_count, 1);
        assert!(snapshot.label_stats.imbalance_ratio > 1.0);
        assert!(snapshot.label_stats.entropy > 0.0);
    }

    #[test]
    fn test_reset() {
        let config = MonitorConfig::default();
        let monitor = DataMonitor::new(config);
        monitor.start();

        let input_ids = vec![vec![1, 2, 3]; 5];
        let attention_mask = vec![vec![1, 1, 1]; 5];

        monitor.record_batch(
            &input_ids,
            &attention_mask,
            None,
            Duration::from_millis(10),
            Duration::from_millis(50),
        );

        monitor.reset();

        let snapshot = monitor.snapshot();
        assert_eq!(snapshot.performance_stats.total_samples_processed, 0);
        assert_eq!(snapshot.token_stats.total_tokens, 0);
        assert_eq!(snapshot.sequence_stats.total_sequences, 0);
    }
}
