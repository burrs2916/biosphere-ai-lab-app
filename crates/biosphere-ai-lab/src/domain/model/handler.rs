use std::sync::Arc;

use async_trait::async_trait;

use crate::core::{EventBus, LabError, Result};
use crate::domain::experiment::repository::ExperimentRepository;

use super::commands::ModelCommand;
use super::aggregate::{ModelRegistration, ModelLineage, ModelSignature, TensorSpec};
use super::repository::ModelRepository;

#[async_trait]
pub trait ModelCommandHandler: Send + Sync {
    async fn handle(&self, cmd: ModelCommand) -> Result<()>;
}

pub struct DefaultModelCommandHandler {
    repo: Arc<dyn ModelRepository>,
    #[allow(dead_code)]
    experiment_repo: Arc<dyn ExperimentRepository>,
    #[allow(dead_code)]
    event_bus: Arc<EventBus>,
}

impl DefaultModelCommandHandler {
    pub fn new(repo: Arc<dyn ModelRepository>, experiment_repo: Arc<dyn ExperimentRepository>, event_bus: Arc<EventBus>) -> Self {
        Self { repo, experiment_repo, event_bus }
    }
}

#[async_trait]
impl ModelCommandHandler for DefaultModelCommandHandler {
    async fn handle(&self, cmd: ModelCommand) -> Result<()> {
        match cmd {
            ModelCommand::RegisterModel { name, version, framework } => {
                let model = ModelRegistration::new(name, version, framework);
                self.repo.save(&model).await?;
                Ok(())
            }

            ModelCommand::RegisterModelFromExperiment { experiment_id, name, version } => {
                let mut experiment = self.experiment_repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::ModelError(format!("Experiment not found: {}", experiment_id)))?;

                if !experiment.status.is_terminal() {
                    return Err(LabError::ModelError(format!("Cannot register model from experiment in {} state, training must be completed first", experiment.status)));
                }

                let mut lineage = ModelLineage::from_experiment(
                    experiment_id.as_str(),
                    &experiment.name,
                );
                lineage.training_config = Some(serde_json::to_value(&experiment.config).unwrap_or_default());
                lineage.dataset = if experiment.config.data_path.is_empty() {
                    None
                } else {
                    Some(experiment.config.data_path.clone())
                };

                if let Some(ref dataset_id) = experiment.config.dataset_id {
                    lineage.datasets.push(crate::domain::model::aggregate::DatasetLineage {
                        dataset_id: dataset_id.clone(),
                        dataset_name: None,
                        dataset_version: experiment.config.dataset_version.clone(),
                        split_name: experiment.config.split_name.clone(),
                        data_path: if experiment.config.data_path.is_empty() { None } else { Some(experiment.config.data_path.clone()) },
                    });
                }
                lineage.split_name = experiment.config.split_name.clone();

                let resolved_version = if version.is_empty() || version == "auto" {
                    let existing = self.repo.list_by_name(&name).await.unwrap_or_default();
                    let max_version = existing.iter()
                        .filter_map(|m| {
                            let parts: Vec<&str> = m.version.split('.').collect();
                            let patch = parts.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                            let minor = parts.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                            let major = parts.first().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                            Some((major, minor, patch))
                        })
                        .max();
                    match max_version {
                        Some((maj, min, pat)) => format!("{}.{}.{}", maj, min, pat + 1),
                        None => "1.0.0".to_string(),
                    }
                } else {
                    version
                };

                let mut model = ModelRegistration::new(name, resolved_version, format!("{}", experiment.config.model_id))
                    .with_lineage(lineage);

                let metadata_path = format!("{}/model_metadata.json", crate::core::config::get_artifact_dir(experiment_id.as_str()));
                let mut inferred_num_features: Option<usize> = None;
                let mut inferred_num_classes: Option<usize> = None;
                let mut inferred_is_classification: Option<bool> = None;

                if let Ok(content) = std::fs::read_to_string(&metadata_path) {
                    if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(nf) = meta.get("num_features").and_then(|v| v.as_u64()) {
                            inferred_num_features = Some(nf as usize);
                        }
                        if let Some(nc) = meta.get("num_classes").and_then(|v| v.as_u64()) {
                            inferred_num_classes = Some(nc as usize);
                        }
                        if let Some(ic) = meta.get("is_classification").and_then(|v| v.as_bool()) {
                            inferred_is_classification = Some(ic);
                        }
                        for (key, value) in meta.as_object().unwrap_or(&serde_json::Map::new()) {
                            model.set_metadata(key.clone(), value.clone());
                        }
                    }
                }

                let num_features = inferred_num_features
                    .unwrap_or(experiment.config.feature_columns.len());
                let num_classes = inferred_num_classes.unwrap_or(0);
                let is_classification = inferred_is_classification
                    .unwrap_or(matches!(experiment.task_type, crate::types::TaskType::Classification));

                let input_spec = TensorSpec {
                    name: "features".to_string(),
                    dtype: "float32".to_string(),
                    shape: vec![1, num_features as i64],
                };
                let output_spec = if is_classification && num_classes > 0 {
                    TensorSpec {
                        name: "probabilities".to_string(),
                        dtype: "float32".to_string(),
                        shape: vec![1, num_classes as i64],
                    }
                } else if is_classification && num_classes == 0 {
                    TensorSpec {
                        name: "probabilities".to_string(),
                        dtype: "float32".to_string(),
                        shape: vec![1, 1],
                    }
                } else {
                    TensorSpec {
                        name: "prediction".to_string(),
                        dtype: "float32".to_string(),
                        shape: vec![1],
                    }
                };
                model = model.with_signature(ModelSignature::new(vec![input_spec], vec![output_spec]));

                for (metric_name, series) in experiment.metrics.all_series() {
                    if let Some(last_point) = series.values.last() {
                        model.set_metadata(format!("last_{}", metric_name), serde_json::json!(last_point.value));
                    }
                    let is_loss = metric_name.contains("loss") || metric_name.contains("error") || metric_name.contains("mse") || metric_name.contains("rmse") || metric_name.contains("mae");
                    let best = if is_loss {
                        series.values.iter().min_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal))
                    } else {
                        series.values.iter().max_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal))
                    };
                    if let Some(best) = best {
                        model.set_metadata(format!("best_{}", metric_name), serde_json::json!(best.value));
                    }
                }

                model.set_metadata("experiment_id".to_string(), serde_json::Value::String(experiment_id.as_str().to_string()));
                model.set_metadata("task_type".to_string(), serde_json::json!(experiment.task_type));
                model.set_metadata("epochs".to_string(), serde_json::json!(experiment.config.epochs));
                model.set_metadata("batch_size".to_string(), serde_json::json!(experiment.config.batch_size));

                if let Some(ref final_metrics) = experiment.final_metrics {
                    model.set_metadata("final_metrics".to_string(), final_metrics.clone());
                }

                let artifact_dir = crate::core::config::get_artifact_dir(experiment_id.as_str());
                if let Ok(entries) = std::fs::read_dir(&artifact_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();

                        let is_symlink = path.symlink_metadata()
                            .map(|m| m.file_type().is_symlink())
                            .unwrap_or(false);
                        if is_symlink {
                            continue;
                        }

                        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        if name.starts_with("checkpoint-") || name.starts_with("model-") {
                            model.set_path(path.to_string_lossy().to_string());
                            break;
                        }
                    }
                }

                self.repo.save(&model).await?;

                experiment.link_model(model.id.clone());
                self.experiment_repo.save(&experiment).await?;

                Ok(())
            }

            ModelCommand::AddModelVersion { name, version, framework, source_model_id } => {
                let source = self.repo.load(&source_model_id).await?
                    .ok_or_else(|| LabError::ModelError(format!("Source model not found: {}", source_model_id)))?;

                let mut new_model = ModelRegistration::new(name, version, framework);
                if let Some(ref sig) = source.structured_signature {
                    new_model = new_model.with_signature(sig.clone());
                }
                if let Some(ref lineage) = source.lineage {
                    let mut new_lineage = lineage.clone();
                    new_lineage.parent_model_id = Some(source_model_id.to_string());
                    new_model = new_model.with_lineage(new_lineage);
                }
                for (k, v) in &source.metadata {
                    new_model.set_metadata(k.clone(), v.clone());
                }
                if let Some(ref desc) = source.description {
                    new_model.set_description(desc.clone());
                }
                for tag in &source.tags {
                    new_model.add_tag(tag.clone());
                }

                self.repo.save(&new_model).await?;
                Ok(())
            }

            ModelCommand::PromoteToStaging { model_id } => {
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| LabError::ModelError(format!("Model not found: {}", model_id)))?;
                model.promote_to_staging().map_err(|e| LabError::ModelError(e))?;
                self.repo.save(&model).await?;
                Ok(())
            }

            ModelCommand::PromoteToProduction { model_id } => {
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| LabError::ModelError(format!("Model not found: {}", model_id)))?;
                model.promote_to_production().map_err(|e| LabError::ModelError(e))?;
                self.repo.save(&model).await?;
                Ok(())
            }

            ModelCommand::DemoteToStaging { model_id } => {
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| LabError::ModelError(format!("Model not found: {}", model_id)))?;
                model.demote_to_staging().map_err(|e| LabError::ModelError(e))?;
                self.repo.save(&model).await?;
                Ok(())
            }

            ModelCommand::ArchiveModel { model_id } => {
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| LabError::ModelError(format!("Model not found: {}", model_id)))?;
                model.archive().map_err(|e| LabError::ModelError(e))?;
                self.repo.save(&model).await?;
                Ok(())
            }

            ModelCommand::SetModelPath { model_id, path } => {
                if path.contains("..") || path.contains('~') {
                    return Err(LabError::ModelError("Model path contains invalid traversal sequence".to_string()));
                }
                let path_obj = std::path::Path::new(&path);
                if path_obj.is_absolute() && !path.starts_with('/') && !path.starts_with('\\') && !path.contains(':') {
                    return Err(LabError::ModelError("Model path must be relative or a valid absolute path".to_string()));
                }

                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| LabError::ModelError(format!("Model not found: {}", model_id)))?;
                model.set_path(path);
                self.repo.save(&model).await?;
                Ok(())
            }

            ModelCommand::SetModelMetadata { model_id, key, value } => {
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| LabError::ModelError(format!("Model not found: {}", model_id)))?;
                model.set_metadata(key, value);
                self.repo.save(&model).await?;
                Ok(())
            }

            ModelCommand::SetModelDescription { model_id, description } => {
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| LabError::ModelError(format!("Model not found: {}", model_id)))?;
                model.set_description(description);
                self.repo.save(&model).await?;
                Ok(())
            }

            ModelCommand::AddModelTag { model_id, tag } => {
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| LabError::ModelError(format!("Model not found: {}", model_id)))?;
                model.add_tag(tag);
                self.repo.save(&model).await?;
                Ok(())
            }

            ModelCommand::RemoveModelTag { model_id, tag } => {
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| LabError::ModelError(format!("Model not found: {}", model_id)))?;
                model.remove_tag(&tag);
                self.repo.save(&model).await?;
                Ok(())
            }

            ModelCommand::AddModelAlias { model_id, alias } => {
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| LabError::ModelError(format!("Model not found: {}", model_id)))?;
                model.add_alias(alias);
                self.repo.save(&model).await?;
                Ok(())
            }

            ModelCommand::RemoveModelAlias { model_id, alias } => {
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| LabError::ModelError(format!("Model not found: {}", model_id)))?;
                model.remove_alias(&alias);
                self.repo.save(&model).await?;
                Ok(())
            }

            ModelCommand::DeleteModel { model_id } => {
                let model = self.repo.load(&model_id).await?
                    .ok_or_else(|| LabError::ModelError(format!("Model not found: {}", model_id)))?;

                if model.status == crate::domain::model::aggregate::ModelStatus::Production {
                    return Err(LabError::ModelError("Cannot delete a model in Production status, please demote it first".to_string()));
                }

                self.repo.delete(&model_id).await?;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::infrastructure::persistence::sqlite::model_repo::SqliteModelRepository;
    use crate::infrastructure::persistence::sqlite::experiment_repo::SqliteExperimentRepository;
    use crate::domain::model::aggregate::ModelStatus;
    use crate::domain::experiment::repository::ExperimentRepository;

    async fn setup_repo() -> Arc<dyn ModelRepository> {
        let conn = rusqlite::Connection::open_in_memory()
            .expect("Failed to create in-memory DB");
        SqliteModelRepository::init_schema(&conn)
            .expect("Failed to init schema");
        let repo = SqliteModelRepository::new(std::sync::Arc::new(std::sync::Mutex::new(conn)));
        Arc::new(repo)
    }

    async fn setup_experiment_repo() -> Arc<dyn ExperimentRepository> {
        let repo = SqliteExperimentRepository::new(":memory:")
            .expect("Failed to create in-memory DB");
        Arc::new(repo)
    }

    fn setup_handler(repo: Arc<dyn ModelRepository>, experiment_repo: Arc<dyn ExperimentRepository>) -> DefaultModelCommandHandler {
        let event_bus = Arc::new(EventBus::new(256));
        DefaultModelCommandHandler::new(repo, experiment_repo, event_bus)
    }

    #[tokio::test]
    async fn test_model_stage_transitions() {
        let repo = setup_repo().await;
        let exp_repo = setup_experiment_repo().await;
        let handler = setup_handler(repo.clone(), exp_repo);

        let model = ModelRegistration::new("test-model".to_string(), "1.0.0".to_string(), "burn".to_string());
        let model_id = model.id.clone();
        repo.save(&model).await.unwrap();

        handler.handle(ModelCommand::PromoteToStaging {
            model_id: model_id.clone(),
        }).await.unwrap();
        let loaded = repo.load(&model_id).await.unwrap().unwrap();
        assert_eq!(loaded.status, ModelStatus::Staging);

        handler.handle(ModelCommand::PromoteToProduction {
            model_id: model_id.clone(),
        }).await.unwrap();
        let loaded = repo.load(&model_id).await.unwrap().unwrap();
        assert_eq!(loaded.status, ModelStatus::Production);

        handler.handle(ModelCommand::DemoteToStaging {
            model_id: model_id.clone(),
        }).await.unwrap();
        let loaded = repo.load(&model_id).await.unwrap().unwrap();
        assert_eq!(loaded.status, ModelStatus::Staging);

        handler.handle(ModelCommand::PromoteToProduction {
            model_id: model_id.clone(),
        }).await.unwrap();
        handler.handle(ModelCommand::ArchiveModel {
            model_id: model_id.clone(),
        }).await.unwrap();
        let loaded = repo.load(&model_id).await.unwrap().unwrap();
        assert_eq!(loaded.status, ModelStatus::Archived);
    }

    #[tokio::test]
    async fn test_model_alias_management() {
        let repo = setup_repo().await;
        let exp_repo = setup_experiment_repo().await;
        let handler = setup_handler(repo.clone(), exp_repo);

        let model = ModelRegistration::new("alias-model".to_string(), "2.0.0".to_string(), "burn".to_string());
        let model_id = model.id.clone();
        repo.save(&model).await.unwrap();

        handler.handle(ModelCommand::AddModelAlias {
            model_id: model_id.clone(),
            alias: "champion".to_string(),
        }).await.unwrap();

        handler.handle(ModelCommand::AddModelAlias {
            model_id: model_id.clone(),
            alias: "production-ready".to_string(),
        }).await.unwrap();

        let loaded = repo.load(&model_id).await.unwrap().unwrap();
        assert_eq!(loaded.aliases.len(), 2);
        assert!(loaded.aliases.contains(&"champion".to_string()));

        handler.handle(ModelCommand::RemoveModelAlias {
            model_id: model_id.clone(),
            alias: "champion".to_string(),
        }).await.unwrap();

        let loaded = repo.load(&model_id).await.unwrap().unwrap();
        assert_eq!(loaded.aliases.len(), 1);
        assert!(!loaded.aliases.contains(&"champion".to_string()));
    }

    #[tokio::test]
    async fn test_model_invalid_stage_transition() {
        let repo = setup_repo().await;
        let exp_repo = setup_experiment_repo().await;
        let handler = setup_handler(repo.clone(), exp_repo);

        let model = ModelRegistration::new("invalid-model".to_string(), "1.0.0".to_string(), "burn".to_string());
        let model_id = model.id.clone();
        repo.save(&model).await.unwrap();

        let result = handler.handle(ModelCommand::PromoteToProduction {
            model_id: model_id.clone(),
        }).await;
        assert!(result.is_err());

        handler.handle(ModelCommand::PromoteToStaging {
            model_id: model_id.clone(),
        }).await.unwrap();

        let result = handler.handle(ModelCommand::PromoteToStaging {
            model_id: model_id.clone(),
        }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_model_list_by_status() {
        let repo = setup_repo().await;

        let mut model1 = ModelRegistration::new("model-a".to_string(), "1.0.0".to_string(), "burn".to_string());
        model1.promote_to_staging().unwrap();
        repo.save(&model1).await.unwrap();

        let mut model2 = ModelRegistration::new("model-b".to_string(), "1.0.0".to_string(), "burn".to_string());
        model2.promote_to_staging().unwrap();
        model2.promote_to_production().unwrap();
        repo.save(&model2).await.unwrap();

        let model3 = ModelRegistration::new("model-c".to_string(), "1.0.0".to_string(), "burn".to_string());
        repo.save(&model3).await.unwrap();

        let staging = repo.list(Some(ModelStatus::Staging)).await.unwrap();
        assert_eq!(staging.len(), 1);

        let production = repo.list(Some(ModelStatus::Production)).await.unwrap();
        assert_eq!(production.len(), 1);

        let none = repo.list(Some(ModelStatus::None)).await.unwrap();
        assert_eq!(none.len(), 1);

        let all = repo.list(None).await.unwrap();
        assert_eq!(all.len(), 3);
    }
}
