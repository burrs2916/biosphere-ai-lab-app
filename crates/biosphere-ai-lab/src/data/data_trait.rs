use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core::Result;
use crate::core::config::DataLoadConfig;
use crate::types::{DataFormat, PluginId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetInfo {
    pub name: String,
    pub format: DataFormat,
    pub rows: usize,
    pub columns: usize,
    pub column_names: Vec<String>,
    pub column_types: Vec<String>,
    pub has_missing_values: bool,
    pub memory_size_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPreview {
    pub columns: Vec<String>,
    pub column_types: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub total_rows: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreprocessType {
    Normalize,
    Standardize,
    OneHotEncode,
    LabelEncode,
    FillMissing,
    DropMissing,
    Tokenize,
    PadSequence,
    AugmentImage,
    ResizeImage,
    Custom(String),
}

#[async_trait]
pub trait DataSource: Send + Sync {
    fn id(&self) -> &PluginId;
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    fn data_format(&self) -> DataFormat;
    fn supported_preprocessing(&self) -> Vec<PreprocessType>;

    async fn load(&self, config: &DataLoadConfig) -> Result<DatasetInfo>;
    async fn preview(&self, config: &DataLoadConfig, offset: usize, limit: usize) -> Result<DataPreview>;
}
