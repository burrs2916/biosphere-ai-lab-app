use serde::{Deserialize, Serialize};

use super::super::experiment::aggregate::ExperimentId;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum TrainingEvent {
    TrainingStarted {
        experiment_id: ExperimentId,
    },
    TrainingPaused {
        experiment_id: ExperimentId,
    },
    TrainingResumed {
        experiment_id: ExperimentId,
    },
    TrainingStopped {
        experiment_id: ExperimentId,
    },
    EpochCompleted {
        experiment_id: ExperimentId,
        epoch: usize,
        total_epochs: usize,
        train_loss: f64,
        val_loss: Option<f64>,
        metrics: serde_json::Value,
    },
    BatchCompleted {
        experiment_id: ExperimentId,
        batch: usize,
        total_batches: usize,
        loss: f64,
    },
    CheckpointSaved {
        experiment_id: ExperimentId,
        path: String,
        epoch: usize,
    },
}
