#[cfg(feature = "tch-engine")]
mod tch_engine_impl;
#[cfg(feature = "tch-engine")]
mod tch_training;

#[cfg(feature = "tch-engine")]
pub use tch_engine_impl::TchEngine;
#[cfg(feature = "tch-engine")]
pub use tch_training::{run_tch_inference, run_tch_training_from_checkpoint, create_model_for_export, TchInferenceResult};
