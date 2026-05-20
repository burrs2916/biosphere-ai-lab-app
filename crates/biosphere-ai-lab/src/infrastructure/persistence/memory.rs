use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::core::Result;
use crate::domain::experiment::aggregate::{Experiment, ExperimentId, ExperimentSummary, LogEntry};
use crate::domain::experiment::ExperimentFilter;
use crate::domain::experiment::metrics::MetricsTimeline;
use crate::domain::experiment::repository::ExperimentRepository;

fn is_loss_metric(name: &str) -> bool {
    name.contains("loss") || name.contains("error") || name.contains("mse") || name.contains("rmse") || name.contains("mae")
}

pub struct InMemoryExperimentRepository {
    experiments: Arc<RwLock<HashMap<String, Experiment>>>,
    logs: Arc<RwLock<HashMap<String, Vec<LogEntry>>>>,
}

impl InMemoryExperimentRepository {
    pub fn new() -> Self {
        Self {
            experiments: Arc::new(RwLock::new(HashMap::new())),
            logs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryExperimentRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExperimentRepository for InMemoryExperimentRepository {
    async fn save(&self, experiment: &Experiment) -> Result<()> {
        let mut experiments = self.experiments.write().await;
        experiments.insert(experiment.id.as_str().to_string(), experiment.clone());
        Ok(())
    }

    async fn load(&self, id: &ExperimentId) -> Result<Option<Experiment>> {
        let experiments = self.experiments.read().await;
        Ok(experiments.get(id.as_str()).cloned())
    }

    async fn list(&self, filter: &ExperimentFilter) -> Result<Vec<ExperimentSummary>> {
        let experiments = self.experiments.read().await;
        let mut results: Vec<ExperimentSummary> = experiments
            .values()
            .filter(|e| {
                if let Some(ref status) = filter.status {
                    if e.status != *status {
                        return false;
                    }
                }
                if let Some(ref name_contains) = filter.name_contains {
                    if !e.name.contains(name_contains) {
                        return false;
                    }
                }
                if !filter.tags.is_empty() {
                    if !filter.tags.iter().all(|t| e.tags.contains(t)) {
                        return false;
                    }
                }
                if let Some(ref task_type) = filter.task_type {
                    if e.task_type != *task_type {
                        return false;
                    }
                }
                if let Some(ref group) = filter.group {
                    if e.group.as_ref() != Some(group) {
                        return false;
                    }
                }
                true
            })
            .map(|e| ExperimentSummary {
                id: e.id.clone(),
                name: e.name.clone(),
                status: e.status,
                task_type: e.task_type,
                tags: e.tags.clone(),
                model_id: e.model_id.as_ref().map(|m| m.to_string()),
                dataset_id: e.dataset_id.clone(),
                dataset_version: e.dataset_version.clone(),
                group: e.group.clone(),
                created_at: e.created_at,
                updated_at: e.updated_at,
                metric_names: e.metrics.series_names(),
                best_metrics: e.metrics.all_series().iter().filter_map(|(name, series)| {
                    if is_loss_metric(name) {
                        series.min().map(|v| (name.clone(), v))
                    } else {
                        series.max().map(|v| (name.clone(), v))
                    }
                }).collect(),
            })
            .collect();

        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    async fn delete(&self, id: &ExperimentId) -> Result<()> {
        let mut experiments = self.experiments.write().await;
        experiments.remove(id.as_str());
        Ok(())
    }

    async fn query_metrics(
        &self,
        id: &ExperimentId,
        metric_names: &[String],
    ) -> Result<MetricsTimeline> {
        let experiments = self.experiments.read().await;
        match experiments.get(id.as_str()) {
            Some(experiment) => {
                let mut timeline = MetricsTimeline::new();
                for name in metric_names {
                    if let Some(series) = experiment.metrics.get_series(name) {
                        for point in &series.values {
                            timeline.record(name, point.value, point.step);
                        }
                    }
                }
                Ok(timeline)
            }
            None => Ok(MetricsTimeline::new()),
        }
    }

    async fn exists(&self, id: &ExperimentId) -> Result<bool> {
        let experiments = self.experiments.read().await;
        Ok(experiments.contains_key(id.as_str()))
    }

    async fn save_metric_point(
        &self,
        experiment_id: &ExperimentId,
        metric_name: &str,
        step: u64,
        value: f64,
        epoch: Option<usize>,
    ) -> Result<()> {
        let mut experiments = self.experiments.write().await;
        if let Some(experiment) = experiments.get_mut(experiment_id.as_str()) {
            if let Some(epoch_val) = epoch {
                experiment.metrics.record_with_epoch(metric_name, value, step, epoch_val);
            } else {
                experiment.metrics.record(metric_name, value, step);
            }
            experiment.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn update_status(
        &self,
        experiment_id: &ExperimentId,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        let mut experiments = self.experiments.write().await;
        if let Some(experiment) = experiments.get_mut(experiment_id.as_str()) {
            experiment.status = match status.to_lowercase().as_str() {
                "running" => crate::domain::experiment::aggregate::ExperimentStatus::Running,
                "paused" => crate::domain::experiment::aggregate::ExperimentStatus::Paused,
                "completed" => crate::domain::experiment::aggregate::ExperimentStatus::Completed,
                "failed" => crate::domain::experiment::aggregate::ExperimentStatus::Failed,
                "cancelled" => crate::domain::experiment::aggregate::ExperimentStatus::Cancelled,
                "archived" => crate::domain::experiment::aggregate::ExperimentStatus::Archived,
                "created" => crate::domain::experiment::aggregate::ExperimentStatus::Created,
                _ => crate::domain::experiment::aggregate::ExperimentStatus::Created,
            };
            experiment.updated_at = chrono::Utc::now();
            let status_lower = status.to_lowercase();
            if status_lower == "completed" || status_lower == "failed" || status_lower == "cancelled" {
                experiment.completed_at = Some(chrono::Utc::now());
            }
            if let Some(msg) = error_message {
                experiment.error_message = Some(msg.to_string());
            }
        }
        Ok(())
    }

    async fn save_environment(
        &self,
        experiment_id: &ExperimentId,
        environment: &crate::domain::experiment::aggregate::EnvironmentInfo,
    ) -> Result<()> {
        let mut experiments = self.experiments.write().await;
        if let Some(experiment) = experiments.get_mut(experiment_id.as_str()) {
            experiment.environment = Some(environment.clone());
            experiment.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    async fn save_log(
        &self,
        experiment_id: &ExperimentId,
        level: &str,
        message: &str,
    ) -> Result<()> {
        let mut logs = self.logs.write().await;
        let entry = LogEntry {
            level: level.to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now(),
        };
        logs.entry(experiment_id.as_str().to_string())
            .or_insert_with(Vec::new)
            .push(entry);
        Ok(())
    }

    async fn load_logs(
        &self,
        experiment_id: &ExperimentId,
        limit: usize,
    ) -> Result<Vec<LogEntry>> {
        let logs = self.logs.read().await;
        match logs.get(experiment_id.as_str()) {
            Some(entries) => {
                let start = entries.len().saturating_sub(limit);
                Ok(entries[start..].to_vec())
            }
            None => Ok(Vec::new()),
        }
    }
}
