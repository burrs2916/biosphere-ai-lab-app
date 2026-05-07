use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{ComputeBackend, TaskType, DataFormat, MetricType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub session_name: String,
    pub task_type: TaskType,
    pub engine_id: String,
    pub model_id: String,
    pub data_source_id: String,
    pub data_path: String,

    pub dataset_id: Option<String>,
    pub dataset_version: Option<String>,

    pub epochs: usize,
    pub batch_size: usize,
    pub learning_rate: f64,
    pub optimizer: OptimizerConfig,
    pub loss_function: String,

    pub compute_backend: ComputeBackend,
    pub data_format: DataFormat,

    pub validation_split: f64,
    pub test_split: f64,
    pub shuffle: bool,
    pub seed: Option<u64>,

    pub split_name: Option<String>,

    #[serde(default)]
    pub split_indices: Option<SplitIndices>,

    pub checkpoint_interval: Option<usize>,
    pub early_stopping: Option<EarlyStoppingConfig>,
    pub lr_scheduler: LrSchedulerConfig,

    pub target_columns: Vec<String>,
    pub feature_columns: Vec<String>,

    pub custom_params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizerConfig {
    Sgd {
        momentum: Option<f64>,
        weight_decay: Option<f64>,
    },
    Adam {
        beta1: f64,
        beta2: f64,
        weight_decay: Option<f64>,
    },
    AdamW {
        beta1: f64,
        beta2: f64,
        weight_decay: f64,
    },
    Rmsprop {
        alpha: f64,
        weight_decay: Option<f64>,
    },
    Custom {
        name: String,
        params: HashMap<String, serde_json::Value>,
    },
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        OptimizerConfig::Adam {
            beta1: 0.9,
            beta2: 0.999,
            weight_decay: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarlyStoppingConfig {
    pub metric: MetricType,
    pub patience: usize,
    pub min_delta: f64,
    pub mode: EarlyStoppingMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EarlyStoppingMode {
    #[serde(alias = "Min")]
    Min,
    #[serde(alias = "Max")]
    Max,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LrSchedulerConfig {
    Constant,
    Step {
        step_size: usize,
        gamma: f64,
    },
    Exponential {
        gamma: f64,
    },
    CosineAnnealing {
        min_lr: f64,
        num_iters: usize,
    },
    Linear {
        final_lr: f64,
        num_iters: usize,
    },
}

impl Default for LrSchedulerConfig {
    fn default() -> Self {
        LrSchedulerConfig::Constant
    }
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            session_name: String::from("Untitled Session"),
            task_type: TaskType::Classification,
            engine_id: String::from("burn"),
            model_id: String::from("mlp"),
            data_source_id: String::from("csv"),
            data_path: String::new(),
            dataset_id: None,
            dataset_version: None,
            epochs: 10,
            batch_size: 32,
            learning_rate: 0.001,
            optimizer: OptimizerConfig::default(),
            loss_function: String::from("cross_entropy"),
            compute_backend: ComputeBackend::Wgpu,
            data_format: DataFormat::Csv,
            validation_split: 0.2,
            test_split: 0.1,
            shuffle: true,
            seed: Some(42),
            split_name: None,
            split_indices: None,
            checkpoint_interval: None,
            early_stopping: None,
            lr_scheduler: LrSchedulerConfig::default(),
            target_columns: Vec::new(),
            feature_columns: Vec::new(),
            custom_params: HashMap::new(),
        }
    }
}

impl TrainingConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.epochs == 0 {
            return Err("epochs must be at least 1".to_string());
        }
        if self.batch_size == 0 {
            return Err("batch_size must be at least 1".to_string());
        }
        if self.learning_rate <= 0.0 {
            return Err("learning_rate must be positive".to_string());
        }
        if self.learning_rate > 10.0 {
            return Err("learning_rate is unreasonably large (>10.0)".to_string());
        }
        if self.validation_split < 0.0 || self.validation_split >= 1.0 {
            return Err("validation_split must be in [0.0, 1.0)".to_string());
        }
        if self.test_split < 0.0 || self.test_split >= 1.0 {
            return Err("test_split must be in [0.0, 1.0)".to_string());
        }
        if self.validation_split + self.test_split >= 1.0 {
            return Err("validation_split + test_split must be less than 1.0".to_string());
        }
        if let Some(interval) = self.checkpoint_interval {
            if interval == 0 {
                return Err("checkpoint_interval must be at least 1".to_string());
            }
        }
        if let Some(ref es) = self.early_stopping {
            if es.patience == 0 {
                return Err("early_stopping patience must be at least 1".to_string());
            }
            if es.min_delta < 0.0 {
                return Err("early_stopping min_delta must be non-negative".to_string());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub task_type: TaskType,
    pub num_classes: Option<usize>,
    pub num_outputs: Option<usize>,
    pub metrics: Vec<MetricType>,
    pub custom_params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitIndices {
    pub split_name: String,
    pub train_indices: Vec<usize>,
    pub val_indices: Vec<usize>,
    pub test_indices: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLoadConfig {
    pub path: String,
    pub format: DataFormat,
    pub has_header: bool,
    pub delimiter: Option<char>,
    pub encoding: Option<String>,
    pub max_rows: Option<usize>,
    pub custom_params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub architecture_id: String,
    pub layers: Vec<LayerConfig>,
    pub custom_params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerConfig {
    pub layer_type: String,
    pub params: HashMap<String, serde_json::Value>,
}

pub fn get_artifact_dir(id: &str) -> String {
    let base = std::env::var("BIOSPHERE_ARTIFACT_DIR").unwrap_or_else(|_| "./artifacts".to_string());
    let sanitized_id = id
        .replace("..", "")
        .replace('/', "_")
        .replace('\\', "_")
        .replace('\0', "")
        .replace('~', "");
    if sanitized_id.is_empty() {
        return format!("{}/unknown", base);
    }
    format!("{}/{}", base, sanitized_id)
}
