use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core::Result;
use crate::types::DataFormat;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPage {
    pub columns: Vec<String>,
    pub column_types: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub total_rows: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSampleConfig {
    pub n: usize,
    pub seed: Option<u64>,
    pub strategy: SampleStrategy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleStrategy {
    Random,
    First,
    Stratified,
}

impl Default for DataSampleConfig {
    fn default() -> Self {
        Self {
            n: 100,
            seed: None,
            strategy: SampleStrategy::First,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataStatistics {
    pub column_name: String,
    pub column_type: String,
    pub total_count: usize,
    pub null_count: usize,
    pub distinct_count: usize,
    pub min_value: Option<serde_json::Value>,
    pub max_value: Option<serde_json::Value>,
    pub mean_value: Option<f64>,
    pub std_value: Option<f64>,
    pub median_value: Option<f64>,
    pub q25_value: Option<f64>,
    pub q75_value: Option<f64>,
    pub top_values: Vec<(String, usize)>,
    pub value_distribution: Option<ValueDistribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueDistribution {
    pub bins: Vec<f64>,
    pub counts: Vec<usize>,
    pub is_categorical: bool,
    pub category_counts: Vec<(String, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataValidationResult {
    pub path: String,
    pub format: DataFormat,
    pub file_exists: bool,
    pub file_readable: bool,
    pub file_size_bytes: u64,
    pub digest_matches: Option<bool>,
    pub expected_digest: Option<String>,
    pub actual_digest: Option<String>,
    pub row_count_matches: Option<bool>,
    pub expected_rows: Option<usize>,
    pub actual_rows: Option<usize>,
    pub column_count_matches: Option<bool>,
    pub expected_columns: Option<usize>,
    pub actual_columns: Option<usize>,
    pub schema_matches: Option<bool>,
    pub missing_columns: Vec<String>,
    pub extra_columns: Vec<String>,
    pub type_mismatches: Vec<(String, String, String)>,
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[async_trait]
pub trait DataAccessor: Send + Sync {
    fn format(&self) -> DataFormat;

    async fn validate(&self, path: &str, expected_digest: Option<&str>, expected_rows: Option<usize>, expected_columns: Option<usize>, expected_column_names: Option<&[String]>) -> Result<DataValidationResult>;

    async fn page(&self, path: &str, offset: usize, limit: usize) -> Result<DataPage>;

    async fn sample(&self, path: &str, config: &DataSampleConfig) -> Result<DataPage>;

    async fn statistics(&self, path: &str, column_name: &str) -> Result<DataStatistics>;

    async fn row_count(&self, path: &str) -> Result<usize>;

    async fn compute_digest(&self, path: &str) -> Result<String>;

    async fn read_rows_by_indices(&self, path: &str, indices: &[usize]) -> Result<DataPage>;
}
