pub mod engine_trait;
pub mod registry;
pub mod burn_engine;
pub mod burn_training;

#[cfg(feature = "tch-engine")]
pub mod tch_engine;

pub use engine_trait::{Engine, SessionHandle, EngineRef, InferenceOutput};
pub use registry::EngineRegistry;
pub use burn_engine::BurnEngine;
pub use burn_training::TrainControl;

#[cfg(feature = "tch-engine")]
pub use tch_engine::TchEngine;
