use crate::core::{PluginKind, Result};
use crate::core::config::TaskConfig;
use crate::types::{DataFormat, MetricType, PluginId, TaskType};

pub trait Task: Send + Sync {
    fn id(&self) -> &PluginId;
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    fn task_type(&self) -> TaskType;
    fn default_config(&self) -> TaskConfig;
    fn validate_config(&self, config: &TaskConfig) -> Result<()>;
    fn required_data_format(&self) -> DataFormat;
    fn supported_metrics(&self) -> Vec<MetricType>;

    fn plugin_kind(&self) -> PluginKind {
        PluginKind::Task
    }
}
