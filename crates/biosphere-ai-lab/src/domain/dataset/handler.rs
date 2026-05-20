use std::sync::Arc;

use async_trait::async_trait;

use crate::core::{EventBus, LabError, Result};
use crate::infrastructure::log;

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
                log("DATASET_HANDLER", "---------- 处理RegisterDataset命令 ----------", None);
                log("DATASET_HANDLER", &format!("name='{}', format={:?}, path='{}'", name, format, path), None);
                log("DATASET_HANDLER", &format!("rows={}, columns={}, size_mb={:.2}, digest={}", 
                    rows, columns, memory_size_mb, &digest[..16.min(digest.len())]), None);

                log("DATASET_HANDLER", "验证: 检查名称非空...", None);
                if name.trim().is_empty() {
                    log("DATASET_HANDLER", "ERROR", Some("数据集名称不能为空"));
                    return Err(LabError::Custom("Dataset name cannot be empty".to_string()));
                }
                log("DATASET_HANDLER", "验证: 检查路径安全性...", None);
                if path.contains("..") || path.contains('~') {
                    log("DATASET_HANDLER", "ERROR", Some(&format!("路径包含非法字符: '{}'", path)));
                    return Err(LabError::Custom("Dataset path contains invalid traversal sequence".to_string()));
                }
                log("DATASET_HANDLER", "验证: 检查摘要重复...", None);
                if let Some(existing) = self.repo.find_by_digest(&digest).await? {
                    log("DATASET_HANDLER", "ERROR", Some(&format!("摘要重复: 已存在数据集 '{}' (id={})", existing.name, existing.id)));
                    return Err(LabError::Custom(format!(
                        "Dataset with same digest already exists: {} ({})",
                        existing.name, existing.id
                    )));
                }
                log("DATASET_HANDLER", "所有验证通过", None);

                log("DATASET_HANDLER", "创建数据集聚合对象...", None);
                let dataset = Dataset::register(name.clone(), format, path.clone(), digest.clone(), rows, columns, column_profiles, memory_size_mb);
                log("DATASET_HANDLER", &format!("数据集对象创建成功: id={}", dataset.id), None);

                log("DATASET_HANDLER", "保存数据集到仓库...", None);
                self.repo.save(&dataset).await?;
                log("DATASET_HANDLER", "数据集保存成功", None);
                log("DATASET_HANDLER", "---------- RegisterDataset命令处理完成 ----------", None);
                Ok(())
            }

            DatasetCommand::NewVersion { dataset_id, new_digest, new_rows, new_columns, new_profiles, new_size_mb } => {
                log("DATASET_HANDLER", "---------- 处理NewVersion命令 ----------", None);
                log("DATASET_HANDLER", &format!("dataset_id={}, new_rows={}, new_columns={}, new_size_mb={:.2}", 
                    dataset_id, new_rows, new_columns, new_size_mb), None);

                log("DATASET_HANDLER", "加载现有数据集...", None);
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| {
                        log("DATASET_HANDLER", "ERROR", Some(&format!("数据集不存在: {}", dataset_id)));
                        LabError::Custom(format!("Dataset not found: {}", dataset_id))
                    })?;
                log("DATASET_HANDLER", &format!("数据集加载成功: name='{}', status={:?}", dataset.name, dataset.status), None);

                log("DATASET_HANDLER", "验证数据集状态...", None);
                if dataset.status != super::aggregate::DatasetStatus::Active {
                    log("DATASET_HANDLER", "ERROR", Some(&format!("数据集状态不是Active: {:?}", dataset.status)));
                    return Err(LabError::Custom(format!(
                        "Cannot create new version: dataset is {} (must be active)",
                        dataset.status
                    )));
                }

                log("DATASET_HANDLER", "创建新版本...", None);
                dataset.new_version(new_digest, new_rows, new_columns, new_profiles, new_size_mb)
                    .map_err(|e| {
                        log("DATASET_HANDLER", "ERROR", Some(&format!("创建新版本失败: {}", e)));
                        LabError::Custom(e)
                    })?;
                log("DATASET_HANDLER", &format!("新版本创建成功: version={}", dataset.version), None);

                self.repo.save(&dataset).await?;
                log("DATASET_HANDLER", "---------- NewVersion命令处理完成 ----------", None);
                Ok(())
            }

            DatasetCommand::CreateSplit { dataset_id, name, strategy, train_ratio, val_ratio, test_ratio, seed, stratify_column, group_column } => {
                log("DATASET_HANDLER", "---------- 处理CreateSplit命令 ----------", None);
                log("DATASET_HANDLER", &format!("dataset_id={}, split_name='{}', strategy={:?}", dataset_id, name, strategy), None);
                log("DATASET_HANDLER", &format!("ratios: train={}, val={}, test={}, seed={}", train_ratio, val_ratio, test_ratio, seed), None);

                log("DATASET_HANDLER", "验证分割参数...", None);
                if name.trim().is_empty() {
                    log("DATASET_HANDLER", "ERROR", Some("分割名称不能为空"));
                    return Err(LabError::Custom("Split name cannot be empty".to_string()));
                }
                if train_ratio <= 0.0 || val_ratio <= 0.0 || test_ratio <= 0.0 {
                    log("DATASET_HANDLER", "ERROR", Some("分割比例必须为正数"));
                    return Err(LabError::Custom("Split ratios must be positive".to_string()));
                }
                let total_ratio = train_ratio + val_ratio + test_ratio;
                if (total_ratio - 1.0).abs() > 0.01 {
                    log("DATASET_HANDLER", "ERROR", Some(&format!("分割比例总和必须为1.0, 当前: {}", total_ratio)));
                    return Err(LabError::Custom(format!("Split ratios must sum to 1.0, got {}", total_ratio)));
                }

                log("DATASET_HANDLER", "加载数据集...", None);
                let dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| {
                        log("DATASET_HANDLER", "ERROR", Some(&format!("数据集不存在: {}", dataset_id)));
                        LabError::Custom(format!("Dataset not found: {}", dataset_id))
                    })?;
                log("DATASET_HANDLER", &format!("数据集加载成功: rows={}, status={:?}", dataset.rows, dataset.status), None);

                if dataset.status != super::aggregate::DatasetStatus::Active {
                    log("DATASET_HANDLER", "ERROR", Some(&format!("数据集状态不是Active: {:?}", dataset.status)));
                    return Err(LabError::Custom(format!(
                        "Cannot create split: dataset is {} (must be active)",
                        dataset.status
                    )));
                }

                if dataset.rows == 0 {
                    log("DATASET_HANDLER", "ERROR", Some("不能为空数据集创建分割"));
                    return Err(LabError::Custom("Cannot create split for empty dataset".to_string()));
                }

                log("DATASET_HANDLER", &format!("创建分割: strategy={:?}", strategy), None);
                let split = match strategy {
                    SplitStrategy::Random => {
                        log("DATASET_HANDLER", "使用随机分割策略", None);
                        DatasetSplit::new_random(name.clone(), train_ratio, val_ratio, test_ratio, seed, dataset.rows)
                    }
                    SplitStrategy::Stratified => {
                        log("DATASET_HANDLER", "使用分层分割策略", None);
                        let col_name = stratify_column.as_deref()
                            .ok_or_else(|| {
                                log("DATASET_HANDLER", "ERROR", Some("分层分割需要指定stratify_column"));
                                LabError::Custom("stratify_column is required for stratified split".to_string())
                            })?;
                        log("DATASET_HANDLER", &format!("分层列: '{}'", col_name), None);
                        let col_values = Self::get_column_values(&dataset, col_name)?;
                        DatasetSplit::new_stratified(name.clone(), train_ratio, val_ratio, test_ratio, seed, col_name.to_string(), &col_values)
                    }
                    SplitStrategy::Temporal => {
                        log("DATASET_HANDLER", "使用时间分割策略", None);
                        DatasetSplit::new_temporal(name.clone(), train_ratio, val_ratio, test_ratio, dataset.rows)
                    }
                    SplitStrategy::Group => {
                        log("DATASET_HANDLER", "使用分组分割策略", None);
                        let col_name = group_column.as_deref()
                            .ok_or_else(|| {
                                log("DATASET_HANDLER", "ERROR", Some("分组分割需要指定group_column"));
                                LabError::Custom("group_column is required for group split".to_string())
                            })?;
                        log("DATASET_HANDLER", &format!("分组列: '{}'", col_name), None);
                        let col_values = Self::get_column_values(&dataset, col_name)?;
                        DatasetSplit::new_group(name.clone(), train_ratio, val_ratio, test_ratio, seed, col_name.to_string(), &col_values)
                    }
                };

                log("DATASET_HANDLER", "验证分割结果...", None);
                split.validate().map_err(|e| {
                    log("DATASET_HANDLER", "ERROR", Some(&format!("分割验证失败: {}", e)));
                    LabError::Custom(e)
                })?;
                log("DATASET_HANDLER", &format!("分割创建成功: train={}, val={}, test={}", 
                    split.train_count(), split.val_count(), split.test_count()), None);

                self.repo.save_split(&dataset_id, &split).await?;
                log("DATASET_HANDLER", "---------- CreateSplit命令处理完成 ----------", None);
                Ok(())
            }

            DatasetCommand::DeleteSplit { dataset_id, name } => {
                log("DATASET_HANDLER", &format!("处理DeleteSplit命令: dataset_id={}, split_name='{}'", dataset_id, name), None);
                self.repo.delete_split(&dataset_id, &name).await?;
                log("DATASET_HANDLER", "分割删除成功", None);
                Ok(())
            }

            DatasetCommand::LinkExperiment { dataset_id, experiment_id } => {
                log("DATASET_HANDLER", &format!("处理LinkExperiment命令: dataset_id={}, experiment_id={}", dataset_id, experiment_id), None);
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| {
                        log("DATASET_HANDLER", "ERROR", Some(&format!("数据集不存在: {}", dataset_id)));
                        LabError::Custom(format!("Dataset not found: {}", dataset_id))
                    })?;
                if dataset.status != super::aggregate::DatasetStatus::Active {
                    log("DATASET_HANDLER", "ERROR", Some(&format!("数据集状态不是Active: {:?}", dataset.status)));
                    return Err(LabError::Custom(format!(
                        "Cannot link experiment: dataset is {} (must be active)",
                        dataset.status
                    )));
                }
                dataset.link_experiment(experiment_id.clone());
                self.repo.save(&dataset).await?;
                log("DATASET_HANDLER", &format!("实验关联成功: experiment_id={}", experiment_id), None);
                Ok(())
            }

            DatasetCommand::UnlinkExperiment { dataset_id, experiment_id } => {
                log("DATASET_HANDLER", &format!("处理UnlinkExperiment命令: dataset_id={}, experiment_id={}", dataset_id, experiment_id), None);
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| {
                        log("DATASET_HANDLER", "ERROR", Some(&format!("数据集不存在: {}", dataset_id)));
                        LabError::Custom(format!("Dataset not found: {}", dataset_id))
                    })?;
                dataset.unlink_experiment(&experiment_id);
                self.repo.save(&dataset).await?;
                log("DATASET_HANDLER", &format!("实验取消关联成功: experiment_id={}", experiment_id), None);
                Ok(())
            }

            DatasetCommand::ArchiveDataset { dataset_id } => {
                log("DATASET_HANDLER", &format!("处理ArchiveDataset命令: dataset_id={}", dataset_id), None);
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| {
                        log("DATASET_HANDLER", "ERROR", Some(&format!("数据集不存在: {}", dataset_id)));
                        LabError::Custom(format!("Dataset not found: {}", dataset_id))
                    })?;
                dataset.archive().map_err(|e| {
                    log("DATASET_HANDLER", "ERROR", Some(&format!("归档失败: {}", e)));
                    LabError::Custom(e)
                })?;
                self.repo.save(&dataset).await?;
                log("DATASET_HANDLER", "数据集归档成功", None);
                Ok(())
            }

            DatasetCommand::RestoreDataset { dataset_id } => {
                log("DATASET_HANDLER", &format!("处理RestoreDataset命令: dataset_id={}", dataset_id), None);
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| {
                        log("DATASET_HANDLER", "ERROR", Some(&format!("数据集不存在: {}", dataset_id)));
                        LabError::Custom(format!("Dataset not found: {}", dataset_id))
                    })?;
                dataset.restore().map_err(|e| {
                    log("DATASET_HANDLER", "ERROR", Some(&format!("恢复失败: {}", e)));
                    LabError::Custom(e)
                })?;
                self.repo.save(&dataset).await?;
                log("DATASET_HANDLER", "数据集恢复成功", None);
                Ok(())
            }

            DatasetCommand::AddTag { dataset_id, tag } => {
                log("DATASET_HANDLER", &format!("处理AddTag命令: dataset_id={}, tag='{}'", dataset_id, tag), None);
                if tag.trim().is_empty() {
                    log("DATASET_HANDLER", "ERROR", Some("标签不能为空"));
                    return Err(LabError::Custom("Tag cannot be empty".to_string()));
                }
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| {
                        log("DATASET_HANDLER", "ERROR", Some(&format!("数据集不存在: {}", dataset_id)));
                        LabError::Custom(format!("Dataset not found: {}", dataset_id))
                    })?;
                dataset.add_tag(tag.clone());
                self.repo.save(&dataset).await?;
                log("DATASET_HANDLER", &format!("标签添加成功: '{}'", tag), None);
                Ok(())
            }

            DatasetCommand::RemoveTag { dataset_id, tag } => {
                log("DATASET_HANDLER", &format!("处理RemoveTag命令: dataset_id={}, tag='{}'", dataset_id, tag), None);
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| {
                        log("DATASET_HANDLER", "ERROR", Some(&format!("数据集不存在: {}", dataset_id)));
                        LabError::Custom(format!("Dataset not found: {}", dataset_id))
                    })?;
                dataset.remove_tag(&tag);
                self.repo.save(&dataset).await?;
                log("DATASET_HANDLER", &format!("标签移除成功: '{}'", tag), None);
                Ok(())
            }

            DatasetCommand::SetDescription { dataset_id, description } => {
                log("DATASET_HANDLER", &format!("处理SetDescription命令: dataset_id={}, description='{}'", dataset_id, description), None);
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| {
                        log("DATASET_HANDLER", "ERROR", Some(&format!("数据集不存在: {}", dataset_id)));
                        LabError::Custom(format!("Dataset not found: {}", dataset_id))
                    })?;
                dataset.set_description(description);
                self.repo.save(&dataset).await?;
                log("DATASET_HANDLER", "描述设置成功", None);
                Ok(())
            }

            DatasetCommand::SetSource { dataset_id, source_type, source_uri } => {
                log("DATASET_HANDLER", &format!("处理SetSource命令: dataset_id={}, source_type='{}', source_uri='{}'", 
                    dataset_id, source_type, source_uri), None);
                if source_uri.contains("..") || source_uri.contains('~') {
                    log("DATASET_HANDLER", "ERROR", Some(&format!("源URI包含非法字符: '{}'", source_uri)));
                    return Err(LabError::Custom("Source URI contains invalid traversal sequence".to_string()));
                }
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| {
                        log("DATASET_HANDLER", "ERROR", Some(&format!("数据集不存在: {}", dataset_id)));
                        LabError::Custom(format!("Dataset not found: {}", dataset_id))
                    })?;
                dataset.set_source(source_type, source_uri);
                self.repo.save(&dataset).await?;
                log("DATASET_HANDLER", "源设置成功", None);
                Ok(())
            }

            DatasetCommand::SetMetadata { dataset_id, key, value } => {
                log("DATASET_HANDLER", &format!("处理SetMetadata命令: dataset_id={}, key='{}'", dataset_id, key), None);
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| {
                        log("DATASET_HANDLER", "ERROR", Some(&format!("数据集不存在: {}", dataset_id)));
                        LabError::Custom(format!("Dataset not found: {}", dataset_id))
                    })?;
                dataset.set_metadata(key.clone(), value);
                self.repo.save(&dataset).await?;
                log("DATASET_HANDLER", &format!("元数据设置成功: key='{}'", key), None);
                Ok(())
            }

            DatasetCommand::SetCard { dataset_id, card } => {
                log("DATASET_HANDLER", &format!("处理SetCard命令: dataset_id={}", dataset_id), None);
                let mut dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| {
                        log("DATASET_HANDLER", "ERROR", Some(&format!("数据集不存在: {}", dataset_id)));
                        LabError::Custom(format!("Dataset not found: {}", dataset_id))
                    })?;
                dataset.set_card(card);
                self.repo.save(&dataset).await?;
                log("DATASET_HANDLER", "数据卡片设置成功", None);
                Ok(())
            }

            DatasetCommand::DeleteDataset { dataset_id } => {
                log("DATASET_HANDLER", &format!("处理DeleteDataset命令: dataset_id={}", dataset_id), None);
                let dataset = self.repo.load(&dataset_id).await?
                    .ok_or_else(|| {
                        log("DATASET_HANDLER", "ERROR", Some(&format!("数据集不存在: {}", dataset_id)));
                        LabError::Custom(format!("Dataset not found: {}", dataset_id))
                    })?;
                log("DATASET_HANDLER", &format!("数据集加载成功: name='{}', status={:?}", dataset.name, dataset.status), None);
                
                if dataset.status == super::aggregate::DatasetStatus::Active {
                    log("DATASET_HANDLER", "ERROR", Some("不能删除活跃状态的数据集"));
                    return Err(LabError::Custom("Cannot delete an active dataset. Archive it first.".to_string()));
                }
                if !dataset.experiment_ids.is_empty() {
                    log("DATASET_HANDLER", "ERROR", Some(&format!("数据集关联了 {} 个实验，无法删除", dataset.experiment_ids.len())));
                    return Err(LabError::Custom(format!(
                        "Cannot delete dataset: it is linked to {} experiment(s). Unlink them first.",
                        dataset.experiment_ids.len()
                    )));
                }
                self.repo.delete(&dataset_id).await?;
                log("DATASET_HANDLER", "数据集删除成功", None);
                Ok(())
            }
        }
    }
}
