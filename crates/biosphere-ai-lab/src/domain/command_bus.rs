use std::sync::Arc;

use crate::core::Result;

use super::experiment::commands::ExperimentCommand;
use super::experiment::aggregate::ExperimentId;
use super::training::commands::TrainingCommand;
use super::model::commands::ModelCommand;

pub enum LabCommand {
    Experiment(ExperimentCommand),
    Training(TrainingCommand),
    Model(ModelCommand),
}

pub struct CommandBus {
    event_bus: Arc<crate::core::EventBus>,
    experiment_handler: Arc<dyn super::experiment::handler::ExperimentCommandHandler + Send + Sync>,
    training_handler: Arc<dyn super::training::handler::TrainingCommandHandler + Send + Sync>,
    model_handler: Arc<dyn super::model::handler::ModelCommandHandler + Send + Sync>,
}

impl CommandBus {
    pub fn new(
        event_bus: Arc<crate::core::EventBus>,
        experiment_handler: Arc<dyn super::experiment::handler::ExperimentCommandHandler + Send + Sync>,
        training_handler: Arc<dyn super::training::handler::TrainingCommandHandler + Send + Sync>,
        model_handler: Arc<dyn super::model::handler::ModelCommandHandler + Send + Sync>,
    ) -> Self {
        Self {
            event_bus,
            experiment_handler,
            training_handler,
            model_handler,
        }
    }

    pub async fn dispatch_experiment(&self, cmd: ExperimentCommand) -> Result<Option<ExperimentId>> {
        self.experiment_handler.handle(cmd).await
    }

    pub async fn dispatch_training(&self, cmd: TrainingCommand) -> Result<()> {
        self.training_handler.handle(cmd).await
    }

    pub async fn dispatch_model(&self, cmd: ModelCommand) -> Result<()> {
        self.model_handler.handle(cmd).await
    }

    pub fn event_bus(&self) -> &Arc<crate::core::EventBus> {
        &self.event_bus
    }
}

impl std::fmt::Debug for CommandBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandBus").finish()
    }
}
