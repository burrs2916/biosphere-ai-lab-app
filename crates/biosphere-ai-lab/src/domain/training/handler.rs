use std::sync::Arc;

use async_trait::async_trait;

use crate::core::{EventBus, Result};
use crate::core::event::LabEvent;

use super::commands::TrainingCommand;
use super::events::TrainingEvent;
use crate::domain::experiment::repository::ExperimentRepository;
use crate::domain::experiment::commands::ExperimentCommand;

fn training_event_to_value(event: &TrainingEvent, context: &str) -> serde_json::Value {
    match serde_json::to_value(event) {
        Ok(v) => v,
        Err(e) => {
            crate::infrastructure::log("EVENT", &format!("Failed to serialize training event {}: {}", context, e), None);
            serde_json::json!({ "error": format!("serialization failed: {}", e), "context": context })
        }
    }
}

#[async_trait]
pub trait TrainingCommandHandler: Send + Sync {
    async fn handle(&self, cmd: TrainingCommand) -> Result<()>;
}

pub struct DefaultTrainingCommandHandler {
    #[allow(dead_code)]
    experiment_repo: Arc<dyn ExperimentRepository>,
    experiment_handler: Arc<dyn crate::domain::experiment::handler::ExperimentCommandHandler>,
    event_bus: Arc<EventBus>,
}

impl DefaultTrainingCommandHandler {
    pub fn new(
        experiment_repo: Arc<dyn ExperimentRepository>,
        experiment_handler: Arc<dyn crate::domain::experiment::handler::ExperimentCommandHandler>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            experiment_repo,
            experiment_handler,
            event_bus,
        }
    }
}

#[async_trait]
impl TrainingCommandHandler for DefaultTrainingCommandHandler {
    async fn handle(&self, cmd: TrainingCommand) -> Result<()> {
        match cmd {
            TrainingCommand::StartTraining { experiment_id, config: _ } => {
                self.experiment_handler.handle(ExperimentCommand::StartExperiment {
                    experiment_id: experiment_id.clone(),
                }).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "TrainingStarted".to_string(),
                    training_event_to_value(&TrainingEvent::TrainingStarted {
                        experiment_id,
                    }, "TrainingStarted"),
                ));

                Ok(())
            }

            TrainingCommand::PauseTraining { experiment_id } => {
                self.experiment_handler.handle(ExperimentCommand::PauseExperiment {
                    experiment_id: experiment_id.clone(),
                }).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "TrainingPaused".to_string(),
                    training_event_to_value(&TrainingEvent::TrainingPaused {
                        experiment_id,
                    }, "TrainingPaused"),
                ));

                Ok(())
            }

            TrainingCommand::ResumeTraining { experiment_id } => {
                self.experiment_handler.handle(ExperimentCommand::ResumeExperiment {
                    experiment_id: experiment_id.clone(),
                }).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "TrainingResumed".to_string(),
                    training_event_to_value(&TrainingEvent::TrainingResumed {
                        experiment_id,
                    }, "TrainingResumed"),
                ));

                Ok(())
            }

            TrainingCommand::StopTraining { experiment_id } => {
                self.experiment_handler.handle(ExperimentCommand::CancelExperiment {
                    experiment_id: experiment_id.clone(),
                }).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "TrainingStopped".to_string(),
                    training_event_to_value(&TrainingEvent::TrainingStopped {
                        experiment_id,
                    }, "TrainingStopped"),
                ));

                Ok(())
            }

            TrainingCommand::RecordEpoch { experiment_id, epoch, total_epochs, train_loss, val_loss, metrics } => {
                self.experiment_handler.handle(ExperimentCommand::TrackMetricWithEpoch {
                    experiment_id: experiment_id.clone(),
                    metric_name: "train_loss".to_string(),
                    value: train_loss,
                    step: epoch as u64,
                    epoch,
                }).await?;

                if let Some(vl) = val_loss {
                    self.experiment_handler.handle(ExperimentCommand::TrackMetricWithEpoch {
                        experiment_id: experiment_id.clone(),
                        metric_name: "val_loss".to_string(),
                        value: vl,
                        step: epoch as u64,
                        epoch,
                    }).await?;
                }

                self.event_bus.emit(LabEvent::Custom(
                    "EpochCompleted".to_string(),
                    training_event_to_value(&TrainingEvent::EpochCompleted {
                        experiment_id,
                        epoch,
                        total_epochs,
                        train_loss,
                        val_loss,
                        metrics,
                    }, "EpochCompleted"),
                ));

                Ok(())
            }

            TrainingCommand::RecordBatch { experiment_id, batch, total_batches, loss } => {
                self.event_bus.emit(LabEvent::Custom(
                    "BatchCompleted".to_string(),
                    training_event_to_value(&TrainingEvent::BatchCompleted {
                        experiment_id,
                        batch,
                        total_batches,
                        loss,
                    }, "BatchCompleted"),
                ));

                Ok(())
            }

            TrainingCommand::SaveCheckpoint { experiment_id, path, epoch } => {
                self.event_bus.emit(LabEvent::Custom(
                    "CheckpointSaved".to_string(),
                    training_event_to_value(&TrainingEvent::CheckpointSaved {
                        experiment_id,
                        path,
                        epoch,
                    }, "CheckpointSaved"),
                ));

                Ok(())
            }
        }
    }
}
