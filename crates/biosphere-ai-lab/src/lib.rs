#![recursion_limit = "256"]

pub mod infrastructure;
pub mod core;
pub mod types;
pub mod engine;
pub mod task;
pub mod model;
pub mod data;
pub mod training;
pub mod hardware;
pub mod remote;
pub mod domain;
pub mod gateway;

pub use gateway::AppState;
pub use infrastructure::log;

pub mod prelude {
    pub use crate::core::{
        Plugin, PluginInfo, PluginKind,
        EventBus, LabEvent,
        Session, SessionInfo,
        TrainingConfig, OptimizerConfig, EarlyStoppingConfig, TaskConfig, DataLoadConfig, ModelConfig, LayerConfig,
        LabError, Result,
    };
    pub use crate::types::{
        SessionId, PluginId, CheckpointId,
        SessionStatus, ComputeBackend, TaskType, DataFormat, MetricType, ArchType,
        TensorShape, TensorSpec, TensorDtype,
    };
    pub use crate::engine::{Engine, EngineRegistry, BurnEngine, SessionHandle};
    pub use crate::task::{Task, TaskRegistry, ClassificationTask};
    pub use crate::model::{ModelArch, ModelRegistry, ModelArchDef, LayerDescription, MlpModel, CnnModel};
    pub use crate::data::{DataSource, DataSourceRegistry, CsvLoader, JsonLoader, DatasetInfo, DataPreview, DataPipeline, PreprocessType};
    pub use crate::training::{TrainingManager, MetricsCollector, EpochMetrics, TrainingResult, CheckpointInfo};
    pub use crate::hardware::{HardwareDetector, HardwareInfo, GpuInfo, ConfigRecommender, TrainingRecommendation};
    pub use crate::remote::{RemoteBackend, RemoteConfig, RemoteAuth, ConnectionHandle, RemoteStatus};
    pub use crate::gateway::AppState;
    pub use crate::infrastructure::{LogConfig, log};
    pub use crate::domain::CommandBus;
    pub use crate::domain::experiment::{Experiment, ExperimentId, ExperimentStatus, ExperimentSummary, ExperimentFilter, MetricsTimeline, ExperimentRepository};
    pub use crate::domain::training::{TrainingCommand, TrainingService};
    pub use crate::domain::model::{ModelRegistration, ModelId, ModelStatus, ModelRepository};
}
