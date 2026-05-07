use async_trait::async_trait;

use crate::core::Result;

use super::aggregate::{Experiment, ExperimentId, ExperimentSummary};
use super::ExperimentFilter;
use super::metrics::MetricsTimeline;

#[async_trait]
pub trait ExperimentRepository: Send + Sync {
    async fn save(&self, experiment: &Experiment) -> Result<()>;

    async fn load(&self, id: &ExperimentId) -> Result<Option<Experiment>>;

    async fn list(&self, filter: &ExperimentFilter) -> Result<Vec<ExperimentSummary>>;

    async fn delete(&self, id: &ExperimentId) -> Result<()>;

    async fn query_metrics(
        &self,
        id: &ExperimentId,
        metric_names: &[String],
    ) -> Result<MetricsTimeline>;

    async fn exists(&self, id: &ExperimentId) -> Result<bool>;

    async fn save_metric_point(
        &self,
        experiment_id: &ExperimentId,
        metric_name: &str,
        step: u64,
        value: f64,
        epoch: Option<usize>,
    ) -> Result<()>;

    async fn update_status(
        &self,
        experiment_id: &ExperimentId,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<()>;

    async fn save_environment(
        &self,
        experiment_id: &ExperimentId,
        environment: &super::aggregate::EnvironmentInfo,
    ) -> Result<()>;

    async fn save_log(
        &self,
        experiment_id: &ExperimentId,
        level: &str,
        message: &str,
    ) -> Result<()>;

    async fn load_logs(
        &self,
        experiment_id: &ExperimentId,
        limit: usize,
    ) -> Result<Vec<super::aggregate::LogEntry>>;
}
