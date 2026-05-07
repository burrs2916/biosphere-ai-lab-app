use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Created,
    Configuring,
    LoadingData,
    Ready,
    Training,
    Paused,
    Evaluating,
    Completed,
    Failed,
    Cancelled,
}

impl SessionStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, SessionStatus::Completed | SessionStatus::Failed | SessionStatus::Cancelled)
    }

    pub fn is_active(&self) -> bool {
        matches!(self, SessionStatus::Training | SessionStatus::Evaluating | SessionStatus::LoadingData)
    }
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Created => write!(f, "created"),
            SessionStatus::Configuring => write!(f, "configuring"),
            SessionStatus::LoadingData => write!(f, "loading_data"),
            SessionStatus::Ready => write!(f, "ready"),
            SessionStatus::Training => write!(f, "training"),
            SessionStatus::Paused => write!(f, "paused"),
            SessionStatus::Evaluating => write!(f, "evaluating"),
            SessionStatus::Completed => write!(f, "completed"),
            SessionStatus::Failed => write!(f, "failed"),
            SessionStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComputeBackend {
    #[serde(alias = "Cpu")]
    Cpu,
    #[serde(alias = "Cuda")]
    Cuda,
    #[serde(alias = "Wgpu")]
    Wgpu,
    #[serde(alias = "Metal")]
    Metal,
    #[serde(alias = "Rocm")]
    Rocm,
}

impl std::fmt::Display for ComputeBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComputeBackend::Cpu => write!(f, "cpu"),
            ComputeBackend::Cuda => write!(f, "cuda"),
            ComputeBackend::Wgpu => write!(f, "wgpu"),
            ComputeBackend::Metal => write!(f, "metal"),
            ComputeBackend::Rocm => write!(f, "rocm"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    #[serde(alias = "Classification")]
    Classification,
    #[serde(alias = "Regression")]
    Regression,
    #[serde(alias = "Clustering")]
    Clustering,
    #[serde(alias = "Detection")]
    Detection,
    #[serde(alias = "Segmentation")]
    Segmentation,
    #[serde(alias = "Generation")]
    Generation,
    #[serde(alias = "Nlp")]
    Nlp,
    #[serde(alias = "Custom")]
    Custom,
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskType::Classification => write!(f, "classification"),
            TaskType::Regression => write!(f, "regression"),
            TaskType::Clustering => write!(f, "clustering"),
            TaskType::Detection => write!(f, "detection"),
            TaskType::Segmentation => write!(f, "segmentation"),
            TaskType::Generation => write!(f, "generation"),
            TaskType::Nlp => write!(f, "nlp"),
            TaskType::Custom => write!(f, "custom"),
        }
    }
}

impl std::str::FromStr for TaskType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "classification" => Ok(TaskType::Classification),
            "regression" => Ok(TaskType::Regression),
            "clustering" => Ok(TaskType::Clustering),
            "detection" => Ok(TaskType::Detection),
            "segmentation" => Ok(TaskType::Segmentation),
            "generation" => Ok(TaskType::Generation),
            "nlp" => Ok(TaskType::Nlp),
            "custom" => Ok(TaskType::Custom),
            _ => Err(format!("Unknown task type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataFormat {
    #[serde(alias = "Csv")]
    Csv,
    #[serde(alias = "Json")]
    Json,
    #[serde(alias = "Image")]
    Image,
    #[serde(alias = "Text")]
    Text,
    #[serde(alias = "Binary")]
    Binary,
    #[serde(alias = "Parquet")]
    Parquet,
    #[serde(alias = "Excel")]
    Excel,
    #[serde(alias = "TfRecord")]
    TfRecord,
    #[serde(alias = "HuggingFace")]
    HuggingFace,
    #[serde(alias = "Database")]
    Database,
}

impl std::fmt::Display for DataFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataFormat::Csv => write!(f, "csv"),
            DataFormat::Json => write!(f, "json"),
            DataFormat::Image => write!(f, "image"),
            DataFormat::Text => write!(f, "text"),
            DataFormat::Binary => write!(f, "binary"),
            DataFormat::Parquet => write!(f, "parquet"),
            DataFormat::Excel => write!(f, "excel"),
            DataFormat::TfRecord => write!(f, "tfrecord"),
            DataFormat::HuggingFace => write!(f, "huggingface"),
            DataFormat::Database => write!(f, "database"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    #[serde(alias = "Loss")]
    Loss,
    #[serde(alias = "Accuracy")]
    Accuracy,
    #[serde(alias = "Precision")]
    Precision,
    #[serde(alias = "Recall")]
    Recall,
    #[serde(alias = "F1Score")]
    F1Score,
    #[serde(alias = "Mse")]
    Mse,
    #[serde(alias = "Rmse")]
    Rmse,
    #[serde(alias = "Mae")]
    Mae,
    #[serde(alias = "R2")]
    R2,
    #[serde(alias = "Auc")]
    Auc,
    #[serde(alias = "Custom")]
    Custom,
}

impl std::fmt::Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricType::Loss => write!(f, "loss"),
            MetricType::Accuracy => write!(f, "accuracy"),
            MetricType::Precision => write!(f, "precision"),
            MetricType::Recall => write!(f, "recall"),
            MetricType::F1Score => write!(f, "f1_score"),
            MetricType::Mse => write!(f, "mse"),
            MetricType::Rmse => write!(f, "rmse"),
            MetricType::Mae => write!(f, "mae"),
            MetricType::R2 => write!(f, "r2"),
            MetricType::Auc => write!(f, "auc"),
            MetricType::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArchType {
    #[serde(alias = "Mlp")]
    Mlp,
    #[serde(alias = "Cnn")]
    Cnn,
    #[serde(alias = "Rnn")]
    Rnn,
    #[serde(alias = "Lstm")]
    Lstm,
    #[serde(alias = "Gru")]
    Gru,
    #[serde(alias = "Transformer")]
    Transformer,
    #[serde(alias = "Autoencoder")]
    Autoencoder,
    #[serde(alias = "Gan")]
    Gan,
    #[serde(alias = "Custom")]
    Custom,
}

impl std::fmt::Display for ArchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArchType::Mlp => write!(f, "mlp"),
            ArchType::Cnn => write!(f, "cnn"),
            ArchType::Rnn => write!(f, "rnn"),
            ArchType::Lstm => write!(f, "lstm"),
            ArchType::Gru => write!(f, "gru"),
            ArchType::Transformer => write!(f, "transformer"),
            ArchType::Autoencoder => write!(f, "autoencoder"),
            ArchType::Gan => write!(f, "gan"),
            ArchType::Custom => write!(f, "custom"),
        }
    }
}
