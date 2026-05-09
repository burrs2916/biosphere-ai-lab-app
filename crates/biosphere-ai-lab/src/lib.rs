#![recursion_limit = "256"]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::new_without_default)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::double_ended_iterator_last)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::map_identity)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::question_mark)]
#![allow(clippy::let_and_return)]
#![allow(clippy::useless_vec)]
#![allow(clippy::useless_format)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::unnecessary_filter_map)]
#![allow(clippy::manual_flatten)]
#![allow(clippy::manual_div_ceil)]
#![allow(clippy::manual_pattern_char_comparison)]
#![allow(clippy::implicit_saturating_sub)]
#![allow(clippy::wildcard_in_or_patterns)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::unused_enumerate_index)]
#![allow(clippy::manual_map)]
#![allow(clippy::single_match)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::comparison_chain)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::identity_op)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::inefficient_to_string)]
#![allow(unused_assignments)]
#![allow(clippy::collapsible_str_replace)]

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
