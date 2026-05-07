use crate::core::config::TrainingConfig;

use super::super::experiment::aggregate::ExperimentId;

#[derive(Debug, Clone)]
pub enum TrainingCommand {
    StartTraining {
        experiment_id: ExperimentId,
        config: TrainingConfig,
    },
    PauseTraining {
        experiment_id: ExperimentId,
    },
    ResumeTraining {
        experiment_id: ExperimentId,
    },
    StopTraining {
        experiment_id: ExperimentId,
    },
    RecordEpoch {
        experiment_id: ExperimentId,
        epoch: usize,
        total_epochs: usize,
        train_loss: f64,
        val_loss: Option<f64>,
        metrics: serde_json::Value,
    },
    RecordBatch {
        experiment_id: ExperimentId,
        batch: usize,
        total_batches: usize,
        loss: f64,
    },
    SaveCheckpoint {
        experiment_id: ExperimentId,
        path: String,
        epoch: usize,
    },
}
