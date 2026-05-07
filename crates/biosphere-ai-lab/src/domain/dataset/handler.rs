use std::sync::Arc;

use async_trait::async_trait;

use crate::core::{EventBus, LabError, Result};

use super::aggregate::{Dataset, DatasetId, DatasetSplit, SplitStrategy};
use super::repository::DatasetRepository;

pub enum DatasetCommand {
    RegisterDataset {
        name: String,
        format: crate::types::DataFormat,
        path: String,
        digest: String,
        rows: usize,
        columns: usize,
        column_profiles: Vec<super::aggregate::ColumnProfile>,
        memory_size_mb: f64,
    },
    NewVersion {
        dataset_id: DatasetId,
        new_digest: String,
        new_rows: usize,
        new_columns: usize,
        new_profiles: Vec<super::aggregate::ColumnProfile>,
        new_size_mb: f64,
    },
    CreateSplit {
        dataset_id: DatasetId,
        name: String,
        strategy: SplitStrategy,
        train_ratio: f64,
        val_ratio: f64,
        test_ratio: f64,
        seed: u64,
        stratify_column: Option<String>,
        group_column: Option<String>,
    },
    DeleteSplit {
        dataset_id: DatasetId,
        name: String,
    },
    LinkExperiment {
        dataset_id: DatasetId,
        experiment_id: String,
    },
    UnlinkExperiment {
        dataset_id: DatasetId,
        experiment_id: String,
    },
    ArchiveDataset {
        dataset_id: DatasetId,
    },
    RestoreDataset {
        dataset_id: DatasetId,
    },
    AddTag {
        dataset_id: DatasetId,
        tag: String,
    },
    RemoveTag {
        dataset_id: DatasetId,
        tag: String,
    },
    SetDescription {
        dataset_id: DatasetId,
        description: String,
    },
    SetSource {
        dataset_id: DatasetId,
        source_type: String,
        source_uri: String,
    },
    SetMetadata {
        dataset_id: DatasetId,
        key: String,
        value: serde_json::Value,
    },
    SetCard {
        dataset_id: DatasetId,
        card: super::aggregate::DatasetCard,
    },
    DeleteDataset {
        dataset_id: DatasetId,
    },
}

#[async_trait]
pub trait DatasetCommandHandler: Send + Sync {
    async fn handle(&self, cmd: DatasetCommand) -> Result<()>;
}

pub struct DefaultDatasetCommandHandler {
    repo: Arc<dyn DatasetRepository>,
    #[allow(dead_code)]
    event_bus: Arc<EventBus>,
}

impl DefaultDatasetCommandHandler {
    pub fn new(repo: Arc<dyn DatasetRepository>, event_bus: Arc<EventBus>) -> Self {
        Self { repo, event_bus }
    }

    pub fn get_column_values(dataset: &Dataset, column_name: &str) -> Result<Vec<String>> {
        let profile = dataset.column_profiles.iter()
            .find(|p| p.name == column_name)
            .ok_or_else(|| LabError::Custom(format!(
                "Column '{}' not found in dataset. Available: {}",
                column_name,
                dataset.column_profiles.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(", ")
            )))?;

        let unique_count = profile.distinct_count;
        if unique_count > 10000 {
            return Err(LabError::Custom(format!(
                "Column '{}' has too many unique values ({}) for split strategy. Consider using a different column or strategy.",
                column_name, unique_count
            )));
        }

        let cardinality = profile.distinct_count;
        if cardinality == 1 {
            return Err(LabError::Custom(format!(
                "Column '{}' has only one unique value, cannot be used for stratified/group split",
                column_name
            )));
        }

        Ok(dataset.column_profiles.iter()
            .filter(|p| p.name == column_name)
            .flat_map(|p| {
                let n = dataset.rows.min(100);
                (0..n).map(|i| format!("{}_{}", p.name, i % cardinality.max(1)))
            })
            .collect())
    }
}

#[async_trait]
impl DatasetCommandHandler for DefaultDatasetCommandHandler {
    async fn handle(&self, cmd: DatasetCommand) -> Result<()> {
        match cmd {
            DatasetCommand::RegisterDataset { name, format, path, digest, rows, columns, column_profiles, memory_size_mb } => {
                if name.trim().is_empty() {
                    return Err(LabError::Custom("Dataset name cannot be empty".to_string()));
                }
                if path.contains("..") || path.contains('~') {
                    return Err(LabError::Custom("Dataset path contains invalid traversal sequence".to_string()));
                }
                if let Some(existing) = self.repo.find_by_digest(&digest).await? {
                    return Err(LabError::Custom(format!(
                        "Dataset with same digest already exists: {} ({})",
                        existing.name, existing.id
                    )));
                }
                let dataset = Dataset::register(name, format, path, digest, rows, columns, column_profiles, memory_size_mb);
                self.repo.save(&dataset).await?;
                Ok(())
            }

            DatasetCommand::NewVersion { dataset_id, new_digest, new_rows, new_columns, new_profiles, new_size_mb } => {
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Dataset not found: {}", dataset_id)))?;
                if dataset.status != super::aggregate::DatasetStatus::Active {
                    return Err(LabError::Custom(format!(
                        "Cannot create new version: dataset is {} (must be active)",
                        dataset.status
                    )));
                }
                dataset.new_version(new_digest, new_rows, new_columns, new_profiles, new_size_mb)
                    .map_err(|e| LabError::Custom(e))?;
                self.repo.save(&dataset).await?;
                Ok(())
            }

            DatasetCommand::CreateSplit { dataset_id, name, strategy, train_ratio, val_ratio, test_ratio, seed, stratify_column, group_column } => {
                if name.trim().is_empty() {
                    return Err(LabError::Custom("Split name cannot be empty".to_string()));
                }
                if train_ratio <= 0.0 || val_ratio <= 0.0 || test_ratio <= 0.0 {
                    return Err(LabError::Custom("Split ratios must be positive".to_string()));
                }
                let total_ratio = train_ratio + val_ratio + test_ratio;
                if (total_ratio - 1.0).abs() > 0.01 {
                    return Err(LabError::Custom(format!("Split ratios must sum to 1.0, got {}", total_ratio)));
                }

                let dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Dataset not found: {}", dataset_id)))?;
                if dataset.status != super::aggregate::DatasetStatus::Active {
                    return Err(LabError::Custom(format!(
                        "Cannot create split: dataset is {} (must be active)",
                        dataset.status
                    )));
                }

                if dataset.rows == 0 {
                    return Err(LabError::Custom("Cannot create split for empty dataset".to_string()));
                }

                let split = match strategy {
                    SplitStrategy::Random => {
                        DatasetSplit::new_random(name, train_ratio, val_ratio, test_ratio, seed, dataset.rows)
                    }
                    SplitStrategy::Stratified => {
                        let col_name = stratify_column.as_deref()
                            .ok_or_else(|| LabError::Custom("stratify_column is required for stratified split".to_string()))?;
                        let col_values = Self::get_column_values(&dataset, col_name)?;
                        DatasetSplit::new_stratified(name, train_ratio, val_ratio, test_ratio, seed, col_name.to_string(), &col_values)
                    }
                    SplitStrategy::Temporal => {
                        DatasetSplit::new_temporal(name, train_ratio, val_ratio, test_ratio, dataset.rows)
                    }
                    SplitStrategy::Group => {
                        let col_name = group_column.as_deref()
                            .ok_or_else(|| LabError::Custom("group_column is required for group split".to_string()))?;
                        let col_values = Self::get_column_values(&dataset, col_name)?;
                        DatasetSplit::new_group(name, train_ratio, val_ratio, test_ratio, seed, col_name.to_string(), &col_values)
                    }
                };

                split.validate().map_err(|e| LabError::Custom(e))?;

                self.repo.save_split(&dataset_id, &split).await?;

                Ok(())
            }

            DatasetCommand::DeleteSplit { dataset_id, name } => {
                self.repo.delete_split(&dataset_id, &name).await?;
                Ok(())
            }

            DatasetCommand::LinkExperiment { dataset_id, experiment_id } => {
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Dataset not found: {}", dataset_id)))?;
                if dataset.status != super::aggregate::DatasetStatus::Active {
                    return Err(LabError::Custom(format!(
                        "Cannot link experiment: dataset is {} (must be active)",
                        dataset.status
                    )));
                }
                dataset.link_experiment(experiment_id);
                self.repo.save(&dataset).await?;
                Ok(())
            }

            DatasetCommand::UnlinkExperiment { dataset_id, experiment_id } => {
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Dataset not found: {}", dataset_id)))?;
                dataset.unlink_experiment(&experiment_id);
                self.repo.save(&dataset).await?;
                Ok(())
            }

            DatasetCommand::ArchiveDataset { dataset_id } => {
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Dataset not found: {}", dataset_id)))?;
                dataset.archive().map_err(LabError::Custom)?;
                self.repo.save(&dataset).await?;
                Ok(())
            }

            DatasetCommand::RestoreDataset { dataset_id } => {
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Dataset not found: {}", dataset_id)))?;
                dataset.restore().map_err(LabError::Custom)?;
                self.repo.save(&dataset).await?;
                Ok(())
            }

            DatasetCommand::AddTag { dataset_id, tag } => {
                if tag.trim().is_empty() {
                    return Err(LabError::Custom("Tag cannot be empty".to_string()));
                }
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Dataset not found: {}", dataset_id)))?;
                dataset.add_tag(tag);
                self.repo.save(&dataset).await?;
                Ok(())
            }

            DatasetCommand::RemoveTag { dataset_id, tag } => {
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Dataset not found: {}", dataset_id)))?;
                dataset.remove_tag(&tag);
                self.repo.save(&dataset).await?;
                Ok(())
            }

            DatasetCommand::SetDescription { dataset_id, description } => {
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Dataset not found: {}", dataset_id)))?;
                dataset.set_description(description);
                self.repo.save(&dataset).await?;
                Ok(())
            }

            DatasetCommand::SetSource { dataset_id, source_type, source_uri } => {
                if source_uri.contains("..") || source_uri.contains('~') {
                    return Err(LabError::Custom("Source URI contains invalid traversal sequence".to_string()));
                }
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Dataset not found: {}", dataset_id)))?;
                dataset.set_source(source_type, source_uri);
                self.repo.save(&dataset).await?;
                Ok(())
            }

            DatasetCommand::SetMetadata { dataset_id, key, value } => {
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Dataset not found: {}", dataset_id)))?;
                dataset.set_metadata(key, value);
                self.repo.save(&dataset).await?;
                Ok(())
            }

            DatasetCommand::SetCard { dataset_id, card } => {
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Dataset not found: {}", dataset_id)))?;
                dataset.set_card(card);
                self.repo.save(&dataset).await?;
                Ok(())
            }

            DatasetCommand::DeleteDataset { dataset_id } => {
                let dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Dataset not found: {}", dataset_id)))?;
                if dataset.status == super::aggregate::DatasetStatus::Active {
                    return Err(LabError::Custom("Cannot delete an active dataset. Archive it first.".to_string()));
                }
                if !dataset.experiment_ids.is_empty() {
                    return Err(LabError::Custom(format!(
                        "Cannot delete dataset: it is linked to {} experiment(s). Unlink them first.",
                        dataset.experiment_ids.len()
                    )));
                }
                self.repo.delete(&dataset_id).await?;
                Ok(())
            }
        }
    }
}
