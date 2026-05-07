use crate::core::{LabError, Result};
use crate::core::config::TaskConfig;
use crate::types::{DataFormat, MetricType, PluginId, TaskType};
use super::Task;

pub struct ClassificationTask {
    id: PluginId,
}

impl ClassificationTask {
    pub fn new() -> Self {
        Self {
            id: PluginId::new("classification"),
        }
    }
}

impl Default for ClassificationTask {
    fn default() -> Self {
        Self::new()
    }
}

impl Task for ClassificationTask {
    fn id(&self) -> &PluginId {
        &self.id
    }

    fn name(&self) -> &str {
        "Classification"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Supervised classification task for categorical targets"
    }

    fn task_type(&self) -> TaskType {
        TaskType::Classification
    }

    fn default_config(&self) -> TaskConfig {
        TaskConfig {
            task_type: TaskType::Classification,
            num_classes: Some(2),
            num_outputs: None,
            metrics: vec![MetricType::Loss, MetricType::Accuracy, MetricType::Precision, MetricType::Recall, MetricType::F1Score],
            custom_params: std::collections::HashMap::new(),
        }
    }

    fn validate_config(&self, config: &TaskConfig) -> Result<()> {
        if config.num_classes.is_none() {
            return Err(LabError::InvalidConfig("Classification requires num_classes".to_string()));
        }
        Ok(())
    }

    fn required_data_format(&self) -> DataFormat {
        DataFormat::Csv
    }

    fn supported_metrics(&self) -> Vec<MetricType> {
        vec![
            MetricType::Loss,
            MetricType::Accuracy,
            MetricType::Precision,
            MetricType::Recall,
            MetricType::F1Score,
            MetricType::Auc,
        ]
    }
}
