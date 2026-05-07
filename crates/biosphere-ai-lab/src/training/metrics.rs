use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::MetricType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochMetrics {
    pub epoch: usize,
    pub train_loss: f64,
    pub val_loss: Option<f64>,
    pub metrics: HashMap<String, f64>,
    pub learning_rate: f64,
    pub epoch_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingResult {
    pub final_train_loss: f64,
    pub final_val_loss: Option<f64>,
    pub best_epoch: usize,
    pub best_metric: Option<f64>,
    pub total_epochs: usize,
    pub total_time_ms: u64,
    pub metrics_history: Vec<EpochMetrics>,
    pub final_metrics: HashMap<String, f64>,
}

pub struct MetricsCollector {
    history: Vec<EpochMetrics>,
    best_epoch: Option<usize>,
    best_metric: Option<f64>,
    metric_type: MetricType,
}

impl MetricsCollector {
    pub fn new(metric_type: MetricType) -> Self {
        Self {
            history: Vec::new(),
            best_epoch: None,
            best_metric: None,
            metric_type,
        }
    }

    pub fn record(&mut self, metrics: EpochMetrics) {
        let epoch = metrics.epoch;
        let metric_key = self.metric_type.to_string();
        let metric_val = metrics.metrics.get(&metric_key).copied();

        if let Some(val) = metric_val {
            match self.best_metric {
                None => {
                    self.best_metric = Some(val);
                    self.best_epoch = Some(epoch);
                }
                Some(best) => {
                    let is_better = matches!(
                        self.metric_type,
                        MetricType::Accuracy | MetricType::Precision | MetricType::Recall
                            | MetricType::F1Score | MetricType::R2 | MetricType::Auc
                    );
                    if (is_better && val > best) || (!is_better && val < best) {
                        self.best_metric = Some(val);
                        self.best_epoch = Some(epoch);
                    }
                }
            }
        }

        self.history.push(metrics);
    }

    pub fn history(&self) -> &[EpochMetrics] {
        &self.history
    }

    pub fn best_epoch(&self) -> Option<usize> {
        self.best_epoch
    }

    pub fn best_metric(&self) -> Option<f64> {
        self.best_metric
    }

    pub fn latest(&self) -> Option<&EpochMetrics> {
        self.history.last()
    }

    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }
}
