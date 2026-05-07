use std::sync::Arc;

use async_trait::async_trait;

use crate::core::{EventBus, LabError, Result};
use crate::core::event::LabEvent;

use super::commands::ExperimentCommand;
use super::events::ExperimentEvent;
use super::aggregate::{Experiment, ExperimentId, EnvironmentInfo};
use super::repository::ExperimentRepository;

fn event_to_value(event: ExperimentEvent, context: &str) -> serde_json::Value {
    match serde_json::to_value(&event) {
        Ok(v) => v,
        Err(e) => {
            crate::infrastructure::log("EVENT", &format!("Failed to serialize event {}: {}", context, e), None);
            serde_json::json!({ "error": format!("serialization failed: {}", e), "context": context })
        }
    }
}

#[async_trait]
pub trait ExperimentCommandHandler: Send + Sync {
    async fn handle(&self, cmd: ExperimentCommand) -> Result<Option<ExperimentId>>;
}

pub struct DefaultExperimentCommandHandler {
    repo: Arc<dyn ExperimentRepository>,
    event_bus: Arc<EventBus>,
}

impl DefaultExperimentCommandHandler {
    pub fn new(repo: Arc<dyn ExperimentRepository>, event_bus: Arc<EventBus>) -> Self {
        Self { repo, event_bus }
    }
}

#[async_trait]
impl ExperimentCommandHandler for DefaultExperimentCommandHandler {
    async fn handle(&self, cmd: ExperimentCommand) -> Result<Option<ExperimentId>> {
        match cmd {
            ExperimentCommand::CreateExperiment { name, task_type: _, config } => {
                let mut experiment = Experiment::create(name.clone(), config);
                experiment.environment = Some(EnvironmentInfo::capture_all());
                let id = experiment.id.clone();
                self.repo.save(&experiment).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentCreated".to_string(),
                    event_to_value(ExperimentEvent::ExperimentCreated {
                        experiment_id: id.clone(),
                        name,
                    }, "ExperimentCreated"),
                ));

                Ok(Some(id))
            }

            ExperimentCommand::TrackMetric { experiment_id, metric_name, value, step } => {
                self.repo.save_metric_point(&experiment_id, &metric_name, step, value, None).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "MetricTracked".to_string(),
                    event_to_value(ExperimentEvent::MetricTracked {
                        experiment_id,
                        metric_name,
                        value,
                        step,
                    }, "MetricTracked"),
                ));

                Ok(None)
            }

            ExperimentCommand::TrackMetricWithEpoch { experiment_id, metric_name, value, step, epoch } => {
                self.repo.save_metric_point(&experiment_id, &metric_name, step, value, Some(epoch)).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "MetricTracked".to_string(),
                    event_to_value(ExperimentEvent::MetricTracked {
                        experiment_id,
                        metric_name,
                        value,
                        step,
                    }, "MetricTrackedWithEpoch"),
                ));

                Ok(None)
            }

            ExperimentCommand::StartExperiment { experiment_id } => {
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;
                experiment.start().map_err(|e| LabError::Custom(e))?;
                self.repo.save(&experiment).await?;

                let env_info = EnvironmentInfo::capture_all();
                self.repo.save_environment(&experiment_id, &env_info).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentStarted".to_string(),
                    event_to_value(ExperimentEvent::ExperimentStarted {
                        experiment_id,
                    }, "ExperimentStarted"),
                ));

                Ok(None)
            }

            ExperimentCommand::RestartExperiment { experiment_id } => {
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;
                experiment.restart().map_err(|e| LabError::Custom(e))?;
                self.repo.save(&experiment).await?;

                let env_info = EnvironmentInfo::capture_all();
                self.repo.save_environment(&experiment_id, &env_info).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentRestarted".to_string(),
                    event_to_value(ExperimentEvent::ExperimentStarted {
                        experiment_id,
                    }, "ExperimentRestarted"),
                ));

                Ok(None)
            }

            ExperimentCommand::PauseExperiment { experiment_id } => {
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;
                experiment.pause().map_err(|e| LabError::Custom(e))?;
                self.repo.save(&experiment).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentPaused".to_string(),
                    event_to_value(ExperimentEvent::ExperimentPaused {
                        experiment_id,
                    }, "ExperimentPaused"),
                ));

                Ok(None)
            }

            ExperimentCommand::ResumeExperiment { experiment_id } => {
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;
                experiment.resume().map_err(|e| LabError::Custom(e))?;
                self.repo.save(&experiment).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentResumed".to_string(),
                    event_to_value(ExperimentEvent::ExperimentResumed {
                        experiment_id,
                    }, "ExperimentResumed"),
                ));

                Ok(None)
            }

            ExperimentCommand::CompleteExperiment { experiment_id, final_metrics } => {
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;
                experiment.complete(final_metrics.clone()).map_err(|e| LabError::Custom(e))?;
                self.repo.save(&experiment).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentCompleted".to_string(),
                    event_to_value(ExperimentEvent::ExperimentCompleted {
                        experiment_id,
                        final_metrics,
                    }, "ExperimentCompleted"),
                ));

                Ok(None)
            }

            ExperimentCommand::FailExperiment { experiment_id, error } => {
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                experiment.fail(error.clone()).map_err(|e| LabError::Custom(e))?;
                self.repo.save(&experiment).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentFailed".to_string(),
                    event_to_value(ExperimentEvent::ExperimentFailed {
                        experiment_id,
                        error,
                    }, "ExperimentFailed"),
                ));

                Ok(None)
            }

            ExperimentCommand::CancelExperiment { experiment_id } => {
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                experiment.cancel().map_err(|e| LabError::Custom(e))?;
                self.repo.save(&experiment).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentCancelled".to_string(),
                    event_to_value(ExperimentEvent::ExperimentCancelled {
                        experiment_id,
                    }, "ExperimentCancelled"),
                ));

                Ok(None)
            }

            ExperimentCommand::SetParam { experiment_id, key, value } => {
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                experiment.set_param(key.clone(), value);
                self.repo.save(&experiment).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ParamSet".to_string(),
                    event_to_value(ExperimentEvent::ParamSet {
                        experiment_id,
                        key,
                    }, "ParamSet"),
                ));

                Ok(None)
            }

            ExperimentCommand::AddTag { experiment_id, tag } => {
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                experiment.add_tag(tag.clone());
                self.repo.save(&experiment).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "TagAdded".to_string(),
                    event_to_value(ExperimentEvent::TagAdded {
                        experiment_id,
                        tag,
                    }, "TagAdded"),
                ));

                Ok(None)
            }

            ExperimentCommand::DeleteExperiment { experiment_id } => {
                let experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                if experiment.status == super::aggregate::ExperimentStatus::Running
                    || experiment.status == super::aggregate::ExperimentStatus::Paused
                {
                    return Err(LabError::Custom("Cannot delete a running or paused experiment. Stop or cancel it first.".to_string()));
                }

                self.repo.delete(&experiment_id).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentDeleted".to_string(),
                    event_to_value(ExperimentEvent::ExperimentDeleted {
                        experiment_id,
                    }, "ExperimentDeleted"),
                ));

                Ok(None)
            }

            ExperimentCommand::SetDescription { experiment_id, description } => {
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                experiment.set_description(description);
                self.repo.save(&experiment).await?;

                Ok(None)
            }

            ExperimentCommand::RemoveTag { experiment_id, tag } => {
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                experiment.remove_tag(&tag);
                self.repo.save(&experiment).await?;

                Ok(None)
            }

            ExperimentCommand::CloneExperiment { experiment_id, new_name } => {
                let source = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                let mut cloned = Experiment::create(new_name.clone(), source.config.clone());
                for tag in &source.tags {
                    cloned.add_tag(tag.clone());
                }
                for (key, value) in &source.params {
                    cloned.set_param(key.clone(), value.clone());
                }
                if let Some(desc) = &source.description {
                    cloned.set_description(desc.clone());
                }
                if let Some(ds_id) = &source.dataset_id {
                    let ds_ver = source.dataset_version.clone().unwrap_or_default();
                    cloned.link_dataset(ds_id.clone(), ds_ver);
                }
                let new_id = cloned.id.clone();
                self.repo.save(&cloned).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentCloned".to_string(),
                    event_to_value(ExperimentEvent::ExperimentCloned {
                        source_experiment_id: experiment_id,
                        new_experiment_id: new_id.clone(),
                        new_name,
                    }, "ExperimentCloned"),
                ));

                Ok(Some(new_id))
            }

            ExperimentCommand::AddArtifact { experiment_id, artifact } => {
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                experiment.add_artifact(artifact);
                self.repo.save(&experiment).await?;

                Ok(None)
            }

            ExperimentCommand::ArchiveExperiment { experiment_id } => {
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                experiment.archive().map_err(|e| LabError::Custom(e))?;
                self.repo.save(&experiment).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentArchived".to_string(),
                    event_to_value(ExperimentEvent::ExperimentArchived {
                        experiment_id,
                    }, "ExperimentArchived"),
                ));

                Ok(None)
            }

            ExperimentCommand::RestoreExperiment { experiment_id } => {
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                experiment.restore().map_err(|e| LabError::Custom(e))?;
                self.repo.save(&experiment).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentRestored".to_string(),
                    event_to_value(ExperimentEvent::ExperimentRestored {
                        experiment_id,
                    }, "ExperimentRestored"),
                ));

                Ok(None)
            }

            ExperimentCommand::LinkDataset { experiment_id, dataset_id, dataset_version } => {
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                experiment.link_dataset(dataset_id.clone(), dataset_version.clone().unwrap_or_else(|| "1.0.0".to_string()));
                self.repo.save(&experiment).await?;

                Ok(None)
            }

            ExperimentCommand::SetGroup { experiment_id, group } => {
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                experiment.set_group(group);
                self.repo.save(&experiment).await?;

                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::infrastructure::persistence::sqlite::experiment_repo::SqliteExperimentRepository;
    use crate::domain::experiment::aggregate::{Experiment, ExperimentStatus};
    use crate::core::config::TrainingConfig;
    use crate::domain::experiment::ExperimentFilter;

    async fn setup_repo() -> Arc<dyn ExperimentRepository> {
        let repo = SqliteExperimentRepository::new(":memory:")
            .expect("Failed to create in-memory DB");
        Arc::new(repo)
    }

    #[tokio::test]
    async fn test_experiment_lifecycle_commands() {
        let repo = setup_repo().await;
        let event_bus = Arc::new(EventBus::new(256));
        let handler = DefaultExperimentCommandHandler::new(repo.clone(), event_bus);

        let experiment = Experiment::create("test-lifecycle".to_string(), TrainingConfig::default());
        let exp_id = experiment.id.clone();
        repo.save(&experiment).await.unwrap();

        handler.handle(ExperimentCommand::StartExperiment {
            experiment_id: exp_id.clone(),
        }).await.unwrap();

        let loaded = repo.load(&exp_id).await.unwrap().unwrap();
        assert_eq!(loaded.status, ExperimentStatus::Running);

        handler.handle(ExperimentCommand::PauseExperiment {
            experiment_id: exp_id.clone(),
        }).await.unwrap();

        let loaded = repo.load(&exp_id).await.unwrap().unwrap();
        assert_eq!(loaded.status, ExperimentStatus::Paused);

        handler.handle(ExperimentCommand::ResumeExperiment {
            experiment_id: exp_id.clone(),
        }).await.unwrap();

        let loaded = repo.load(&exp_id).await.unwrap().unwrap();
        assert_eq!(loaded.status, ExperimentStatus::Running);

        handler.handle(ExperimentCommand::CompleteExperiment {
            experiment_id: exp_id.clone(),
            final_metrics: serde_json::json!({"accuracy": 0.95}),
        }).await.unwrap();

        let loaded = repo.load(&exp_id).await.unwrap().unwrap();
        assert_eq!(loaded.status, ExperimentStatus::Completed);
        assert!(loaded.final_metrics.is_some());
    }

    #[tokio::test]
    async fn test_experiment_group_command() {
        let repo = setup_repo().await;
        let event_bus = Arc::new(EventBus::new(256));
        let handler = DefaultExperimentCommandHandler::new(repo.clone(), event_bus);

        let experiment = Experiment::create("grouped-exp".to_string(), TrainingConfig::default());
        let exp_id = experiment.id.clone();
        repo.save(&experiment).await.unwrap();

        handler.handle(ExperimentCommand::SetGroup {
            experiment_id: exp_id.clone(),
            group: "nlp-experiments".to_string(),
        }).await.unwrap();

        let loaded = repo.load(&exp_id).await.unwrap().unwrap();
        assert_eq!(loaded.group, Some("nlp-experiments".to_string()));
    }

    #[tokio::test]
    async fn test_experiment_archive_restore() {
        let repo = setup_repo().await;
        let event_bus = Arc::new(EventBus::new(256));
        let handler = DefaultExperimentCommandHandler::new(repo.clone(), event_bus);

        let mut experiment = Experiment::create("archive-test".to_string(), TrainingConfig::default());
        let exp_id = experiment.id.clone();
        experiment.start().unwrap();
        experiment.complete(serde_json::json!({"accuracy": 0.9})).unwrap();
        repo.save(&experiment).await.unwrap();

        handler.handle(ExperimentCommand::ArchiveExperiment {
            experiment_id: exp_id.clone(),
        }).await.unwrap();

        let loaded = repo.load(&exp_id).await.unwrap().unwrap();
        assert_eq!(loaded.status, ExperimentStatus::Archived);

        handler.handle(ExperimentCommand::RestoreExperiment {
            experiment_id: exp_id.clone(),
        }).await.unwrap();

        let loaded = repo.load(&exp_id).await.unwrap().unwrap();
        assert_eq!(loaded.status, ExperimentStatus::Completed);
    }

    #[tokio::test]
    async fn test_experiment_list_with_group_filter() {
        let repo = setup_repo().await;

        let mut exp1 = Experiment::create("exp-a".to_string(), TrainingConfig::default());
        exp1.set_group("group-a".to_string());
        repo.save(&exp1).await.unwrap();

        let mut exp2 = Experiment::create("exp-b".to_string(), TrainingConfig::default());
        exp2.set_group("group-b".to_string());
        repo.save(&exp2).await.unwrap();

        let exp3 = Experiment::create("exp-c".to_string(), TrainingConfig::default());
        repo.save(&exp3).await.unwrap();

        let mut filter = ExperimentFilter::default();
        filter.group = Some("group-a".to_string());
        let results = repo.list(&filter).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].group, Some("group-a".to_string()));

        filter.group = None;
        let all = repo.list(&filter).await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn test_experiment_metrics_persistence() {
        let repo = setup_repo().await;

        let experiment = Experiment::create("metrics-test".to_string(), TrainingConfig::default());
        let exp_id = experiment.id.clone();
        repo.save(&experiment).await.unwrap();

        repo.save_metric_point(&exp_id, "train_loss", 1, 0.5, None).await.unwrap();
        repo.save_metric_point(&exp_id, "train_loss", 2, 0.3, None).await.unwrap();
        repo.save_metric_point(&exp_id, "train_loss", 3, 0.1, None).await.unwrap();
        repo.save_metric_point(&exp_id, "val_loss", 1, 0.6, None).await.unwrap();
        repo.save_metric_point(&exp_id, "val_loss", 2, 0.4, None).await.unwrap();

        let timeline = repo.query_metrics(&exp_id, &["train_loss".to_string(), "val_loss".to_string()]).await.unwrap();
        let series = timeline.all_series();
        assert!(series.contains_key("train_loss"));
        assert!(series.contains_key("val_loss"));
        assert_eq!(series.get("train_loss").unwrap().len(), 3);
        assert_eq!(series.get("val_loss").unwrap().len(), 2);
    }
}
