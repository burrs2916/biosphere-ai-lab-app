use std::sync::Arc;

use async_trait::async_trait;

use crate::core::{EventBus, LabError, Result};
use crate::domain::experiment::repository::ExperimentRepository;
use crate::infrastructure::log;

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
                log("MODEL_HANDLER", "---------- 处理RegisterModel命令 ----------", None);
                log("MODEL_HANDLER", &format!("name='{}', version='{}', framework='{}'", name, version, framework), None);

                log("MODEL_HANDLER", "创建模型注册对象...", None);
                let model = ModelRegistration::new(name.clone(), version.clone(), framework.clone());
                log("MODEL_HANDLER", &format!("模型对象创建成功: id={}, status={:?}", model.id, model.status), None);

                log("MODEL_HANDLER", "保存模型到仓库...", None);
                self.repo.save(&model).await?;
                log("MODEL_HANDLER", "---------- RegisterModel命令处理完成 ----------", None);
                Ok(())
            }

            ModelCommand::RegisterModelFromExperiment { experiment_id, name, version } => {
                log("MODEL_HANDLER", "---------- 处理RegisterModelFromExperiment命令 ----------", None);
                log("MODEL_HANDLER", &format!("experiment_id='{}', name='{}', version='{}'", experiment_id, name, version), None);

                log("MODEL_HANDLER", "加载实验...", None);
                let mut experiment = self.experiment_repo.load(&experiment_id).await?
                    .ok_or_else(|| {
                        log("MODEL_HANDLER", "ERROR", Some(&format!("实验不存在: {}", experiment_id)));
                        LabError::ModelError(format!("Experiment not found: {}", experiment_id))
                    })?;
                log("MODEL_HANDLER", &format!("实验加载成功: name='{}', status={:?}", experiment.name, experiment.status), None);

                log("MODEL_HANDLER", "验证实验状态...", None);
                if !experiment.status.is_terminal() {
                    log("MODEL_HANDLER", "ERROR", Some(&format!("实验状态不是终态: {:?}", experiment.status)));
                    return Err(LabError::ModelError(format!("Cannot register model from experiment in {} state, training must be completed first", experiment.status)));
                }
                log("MODEL_HANDLER", "实验状态验证通过", None);

                log("MODEL_HANDLER", "构建模型谱系信息...", None);
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

                let effective_dataset_id = experiment.dataset_id.as_ref()
                    .or(experiment.config.dataset_id.as_ref());
                let effective_dataset_version = experiment.dataset_version.as_ref()
                    .or(experiment.config.dataset_version.as_ref());

                if let Some(dataset_id) = effective_dataset_id {
                    log("MODEL_HANDLER", &format!("关联数据集到谱系: dataset_id='{}', version={:?}", dataset_id, effective_dataset_version), None);
                    lineage.datasets.push(crate::domain::model::aggregate::DatasetLineage {
                        dataset_id: (*dataset_id).clone(),
                        dataset_name: None,
                        dataset_version: effective_dataset_version.cloned(),
                        split_name: experiment.config.split_name.clone(),
                        data_path: if experiment.config.data_path.is_empty() { None } else { Some(experiment.config.data_path.clone()) },
                    });
                } else {
                    log("MODEL_HANDLER", "WARN", Some("实验未关联数据集，谱系中datasets将为空"));
                }
                lineage.split_name = experiment.config.split_name.clone();
                log("MODEL_HANDLER", &format!("谱系信息构建完成: datasets={}, split={:?}", lineage.datasets.len(), lineage.split_name), None);

                log("MODEL_HANDLER", "解析版本号...", None);
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
                    let v = match max_version {
                        Some((maj, min, pat)) => format!("{}.{}.{}", maj, min, pat + 1),
                        None => "1.0.0".to_string(),
                    };
                    log("MODEL_HANDLER", &format!("自动版本号: {}", v), None);
                    v
                } else {
                    log("MODEL_HANDLER", &format!("使用指定版本号: {}", version), None);
                    version
                };

                log("MODEL_HANDLER", "创建模型注册对象...", None);
                let mut model = ModelRegistration::new(name.clone(), resolved_version.clone(), format!("{}", experiment.config.model_id))
                    .with_lineage(lineage);
                log("MODEL_HANDLER", &format!("模型对象创建成功: id={}", model.id), None);

                log("MODEL_HANDLER", "读取模型元数据文件...", None);
                let metadata_path = format!("{}/model_metadata.json", crate::core::config::get_artifact_dir(experiment_id.as_str()));
                let mut inferred_num_features: Option<usize> = None;
                let mut inferred_num_classes: Option<usize> = None;
                let mut inferred_is_classification: Option<bool> = None;

                if let Ok(content) = std::fs::read_to_string(&metadata_path) {
                    log("MODEL_HANDLER", &format!("元数据文件读取成功: {} bytes", content.len()), None);
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
                        log("MODEL_HANDLER", &format!("元数据解析成功: features={:?}, classes={:?}, is_classification={:?}", 
                            inferred_num_features, inferred_num_classes, inferred_is_classification), None);
                    }
                } else {
                    log("MODEL_HANDLER", "元数据文件不存在，使用默认值", None);
                }

                let num_features = inferred_num_features
                    .unwrap_or(experiment.config.feature_columns.len());
                let num_classes = inferred_num_classes.unwrap_or(0);
                let is_classification = inferred_is_classification
                    .unwrap_or(matches!(experiment.task_type, crate::types::TaskType::Classification));

                log("MODEL_HANDLER", "构建模型签名...", None);
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
                log("MODEL_HANDLER", &format!("模型签名构建完成: features={}, classes={}, is_classification={}", 
                    num_features, num_classes, is_classification), None);

                log("MODEL_HANDLER", "提取训练指标...", None);
                let metrics_series = experiment.metrics.all_series();
                log("MODEL_HANDLER", &format!("实验内存中指标序列数: {}", metrics_series.len()), None);
                for (metric_name, series) in metrics_series {
                    log("MODEL_HANDLER", &format!("处理指标: name='{}', points={}", metric_name, series.values.len()), None);
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
                        log("MODEL_HANDLER", &format!("指标 '{}' best={:.4} (is_loss={})", metric_name, best.value, is_loss), None);
                    }
                }
                log("MODEL_HANDLER", &format!("指标提取完成: {} 个指标", experiment.metrics.all_series().len()), None);

                model.set_metadata("experiment_id".to_string(), serde_json::Value::String(experiment_id.as_str().to_string()));
                model.set_metadata("task_type".to_string(), serde_json::json!(experiment.task_type));
                model.set_metadata("epochs".to_string(), serde_json::json!(experiment.config.epochs));
                model.set_metadata("batch_size".to_string(), serde_json::json!(experiment.config.batch_size));

                if let Some(ref final_metrics) = experiment.final_metrics {
                    model.set_metadata("final_metrics".to_string(), final_metrics.clone());
                }

                log("MODEL_HANDLER", "查找模型文件...", None);
                let artifact_dir = crate::core::config::get_artifact_dir(experiment_id.as_str());
                let model_path = find_model_file_recursive(std::path::Path::new(&artifact_dir));
                if let Some(path) = model_path {
                    model.set_path(path.to_string_lossy().to_string());
                    log("MODEL_HANDLER", &format!("模型文件找到: '{}'", path.display()), None);
                } else {
                    log("MODEL_HANDLER", "WARN", Some(&format!("未找到模型文件，产物目录: '{}'", artifact_dir)));
                }

                log("MODEL_HANDLER", "保存模型到仓库...", None);
                self.repo.save(&model).await?;
                log("MODEL_HANDLER", &format!("模型保存成功: id={}", model.id), None);

                log("MODEL_HANDLER", "更新实验关联...", None);
                experiment.link_model(model.id.clone());
                self.experiment_repo.save(&experiment).await?;
                log("MODEL_HANDLER", "---------- RegisterModelFromExperiment命令处理完成 ----------", None);
                Ok(())
            }

            ModelCommand::AddModelVersion { name, version, framework, source_model_id } => {
                log("MODEL_HANDLER", "---------- 处理AddModelVersion命令 ----------", None);
                log("MODEL_HANDLER", &format!("name='{}', version='{}', framework='{}', source_model_id={}", name, version, framework, source_model_id), None);

                log("MODEL_HANDLER", "加载源模型...", None);
                let source = self.repo.load(&source_model_id).await?
                    .ok_or_else(|| {
                        log("MODEL_HANDLER", "ERROR", Some(&format!("源模型不存在: {}", source_model_id)));
                        LabError::ModelError(format!("Source model not found: {}", source_model_id))
                    })?;
                log("MODEL_HANDLER", &format!("源模型加载成功: name='{}', status={:?}", source.name, source.status), None);

                log("MODEL_HANDLER", "创建新版本模型...", None);
                let mut new_model = ModelRegistration::new(name.clone(), version.clone(), framework.clone());
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
                log("MODEL_HANDLER", &format!("新版本模型创建成功: id={}", new_model.id), None);

                self.repo.save(&new_model).await?;
                log("MODEL_HANDLER", "---------- AddModelVersion命令处理完成 ----------", None);
                Ok(())
            }

            ModelCommand::PromoteToStaging { model_id } => {
                log("MODEL_HANDLER", &format!("处理PromoteToStaging命令: model_id={}", model_id), None);
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| {
                        log("MODEL_HANDLER", "ERROR", Some(&format!("模型不存在: {}", model_id)));
                        LabError::ModelError(format!("Model not found: {}", model_id))
                    })?;
                log("MODEL_HANDLER", &format!("模型加载成功: name='{}', status={:?}", model.name, model.status), None);
                model.promote_to_staging().map_err(|e| {
                    log("MODEL_HANDLER", "ERROR", Some(&format!("提升失败: {}", e)));
                    LabError::ModelError(e)
                })?;
                self.repo.save(&model).await?;
                log("MODEL_HANDLER", &format!("模型提升成功: status={:?}", model.status), None);
                Ok(())
            }

            ModelCommand::PromoteToProduction { model_id } => {
                log("MODEL_HANDLER", &format!("处理PromoteToProduction命令: model_id={}", model_id), None);
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| {
                        log("MODEL_HANDLER", "ERROR", Some(&format!("模型不存在: {}", model_id)));
                        LabError::ModelError(format!("Model not found: {}", model_id))
                    })?;
                log("MODEL_HANDLER", &format!("模型加载成功: name='{}', status={:?}", model.name, model.status), None);
                model.promote_to_production().map_err(|e| {
                    log("MODEL_HANDLER", "ERROR", Some(&format!("提升失败: {}", e)));
                    LabError::ModelError(e)
                })?;
                self.repo.save(&model).await?;
                log("MODEL_HANDLER", &format!("模型提升成功: status={:?}", model.status), None);
                Ok(())
            }

            ModelCommand::DemoteToStaging { model_id } => {
                log("MODEL_HANDLER", &format!("处理DemoteToStaging命令: model_id={}", model_id), None);
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| {
                        log("MODEL_HANDLER", "ERROR", Some(&format!("模型不存在: {}", model_id)));
                        LabError::ModelError(format!("Model not found: {}", model_id))
                    })?;
                log("MODEL_HANDLER", &format!("模型加载成功: name='{}', status={:?}", model.name, model.status), None);
                model.demote_to_staging().map_err(|e| {
                    log("MODEL_HANDLER", "ERROR", Some(&format!("降级失败: {}", e)));
                    LabError::ModelError(e)
                })?;
                self.repo.save(&model).await?;
                log("MODEL_HANDLER", &format!("模型降级成功: status={:?}", model.status), None);
                Ok(())
            }

            ModelCommand::ArchiveModel { model_id } => {
                log("MODEL_HANDLER", &format!("处理ArchiveModel命令: model_id={}", model_id), None);
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| {
                        log("MODEL_HANDLER", "ERROR", Some(&format!("模型不存在: {}", model_id)));
                        LabError::ModelError(format!("Model not found: {}", model_id))
                    })?;
                log("MODEL_HANDLER", &format!("模型加载成功: name='{}', status={:?}", model.name, model.status), None);
                model.archive().map_err(|e| {
                    log("MODEL_HANDLER", "ERROR", Some(&format!("归档失败: {}", e)));
                    LabError::ModelError(e)
                })?;
                self.repo.save(&model).await?;
                log("MODEL_HANDLER", &format!("模型归档成功: status={:?}", model.status), None);
                Ok(())
            }

            ModelCommand::SetModelPath { model_id, path } => {
                log("MODEL_HANDLER", &format!("处理SetModelPath命令: model_id={}, path='{}'", model_id, path), None);
                if path.contains("..") || path.contains('~') {
                    log("MODEL_HANDLER", "ERROR", Some(&format!("路径包含非法字符: '{}'", path)));
                    return Err(LabError::ModelError("Model path contains invalid traversal sequence".to_string()));
                }
                let path_obj = std::path::Path::new(&path);
                if path_obj.is_absolute() && !path.starts_with('/') && !path.starts_with('\\') && !path.contains(':') {
                    log("MODEL_HANDLER", "ERROR", Some(&format!("路径格式无效: '{}'", path)));
                    return Err(LabError::ModelError("Model path must be relative or a valid absolute path".to_string()));
                }

                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| {
                        log("MODEL_HANDLER", "ERROR", Some(&format!("模型不存在: {}", model_id)));
                        LabError::ModelError(format!("Model not found: {}", model_id))
                    })?;
                model.set_path(path.clone());
                self.repo.save(&model).await?;
                log("MODEL_HANDLER", &format!("模型路径设置成功: '{}'", path), None);
                Ok(())
            }

            ModelCommand::SetModelMetadata { model_id, key, value } => {
                log("MODEL_HANDLER", &format!("处理SetModelMetadata命令: model_id={}, key='{}'", model_id, key), None);
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| {
                        log("MODEL_HANDLER", "ERROR", Some(&format!("模型不存在: {}", model_id)));
                        LabError::ModelError(format!("Model not found: {}", model_id))
                    })?;
                model.set_metadata(key.clone(), value);
                self.repo.save(&model).await?;
                log("MODEL_HANDLER", &format!("元数据设置成功: key='{}'", key), None);
                Ok(())
            }

            ModelCommand::SetModelDescription { model_id, description } => {
                log("MODEL_HANDLER", &format!("处理SetModelDescription命令: model_id={}, len={}", model_id, description.len()), None);
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| {
                        log("MODEL_HANDLER", "ERROR", Some(&format!("模型不存在: {}", model_id)));
                        LabError::ModelError(format!("Model not found: {}", model_id))
                    })?;
                model.set_description(description);
                self.repo.save(&model).await?;
                log("MODEL_HANDLER", "描述设置成功", None);
                Ok(())
            }

            ModelCommand::AddModelTag { model_id, tag } => {
                log("MODEL_HANDLER", &format!("处理AddModelTag命令: model_id={}, tag='{}'", model_id, tag), None);
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| {
                        log("MODEL_HANDLER", "ERROR", Some(&format!("模型不存在: {}", model_id)));
                        LabError::ModelError(format!("Model not found: {}", model_id))
                    })?;
                model.add_tag(tag.clone());
                self.repo.save(&model).await?;
                log("MODEL_HANDLER", &format!("标签添加成功: '{}'", tag), None);
                Ok(())
            }

            ModelCommand::RemoveModelTag { model_id, tag } => {
                log("MODEL_HANDLER", &format!("处理RemoveModelTag命令: model_id={}, tag='{}'", model_id, tag), None);
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| {
                        log("MODEL_HANDLER", "ERROR", Some(&format!("模型不存在: {}", model_id)));
                        LabError::ModelError(format!("Model not found: {}", model_id))
                    })?;
                model.remove_tag(&tag);
                self.repo.save(&model).await?;
                log("MODEL_HANDLER", &format!("标签移除成功: '{}'", tag), None);
                Ok(())
            }

            ModelCommand::AddModelAlias { model_id, alias } => {
                log("MODEL_HANDLER", &format!("处理AddModelAlias命令: model_id={}, alias='{}'", model_id, alias), None);
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| {
                        log("MODEL_HANDLER", "ERROR", Some(&format!("模型不存在: {}", model_id)));
                        LabError::ModelError(format!("Model not found: {}", model_id))
                    })?;
                model.add_alias(alias.clone());
                self.repo.save(&model).await?;
                log("MODEL_HANDLER", &format!("别名添加成功: '{}'", alias), None);
                Ok(())
            }

            ModelCommand::RemoveModelAlias { model_id, alias } => {
                log("MODEL_HANDLER", &format!("处理RemoveModelAlias命令: model_id={}, alias='{}'", model_id, alias), None);
                let mut model = self.repo.load(&model_id).await?
                    .ok_or_else(|| {
                        log("MODEL_HANDLER", "ERROR", Some(&format!("模型不存在: {}", model_id)));
                        LabError::ModelError(format!("Model not found: {}", model_id))
                    })?;
                model.remove_alias(&alias);
                self.repo.save(&model).await?;
                log("MODEL_HANDLER", &format!("别名移除成功: '{}'", alias), None);
                Ok(())
            }

            ModelCommand::DeleteModel { model_id } => {
                log("MODEL_HANDLER", &format!("处理DeleteModel命令: model_id={}", model_id), None);
                let model = self.repo.load(&model_id).await?
                    .ok_or_else(|| {
                        log("MODEL_HANDLER", "ERROR", Some(&format!("模型不存在: {}", model_id)));
                        LabError::ModelError(format!("Model not found: {}", model_id))
                    })?;
                log("MODEL_HANDLER", &format!("模型加载成功: name='{}', status={:?}", model.name, model.status), None);

                if model.status == crate::domain::model::aggregate::ModelStatus::Production {
                    log("MODEL_HANDLER", "ERROR", Some("不能删除Production状态的模型"));
                    return Err(LabError::ModelError("Cannot delete a model in Production status, please demote it first".to_string()));
                }

                self.repo.delete(&model_id).await?;
                log("MODEL_HANDLER", "模型删除成功", None);
                Ok(())
            }
        }
    }
}

fn find_model_file_recursive(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let model_final_path = dir.join("model.mpk");
    if model_final_path.exists() {
        log("MODEL_HANDLER", &format!("找到最终模型文件: '{}'", model_final_path.display()), None);
        return Some(model_final_path);
    }

    let model_mpkgz_path = dir.join("model.mpk.gz");
    if model_mpkgz_path.exists() {
        log("MODEL_HANDLER", &format!("找到最终模型文件: '{}'", model_mpkgz_path.display()), None);
        return Some(model_mpkgz_path);
    }

    let model_bin_path = dir.join("model.bin");
    if model_bin_path.exists() {
        log("MODEL_HANDLER", &format!("找到最终模型文件: '{}'", model_bin_path.display()), None);
        return Some(model_bin_path);
    }

    let model_ot_path = dir.join("model_final.ot");
    if model_ot_path.exists() {
        log("MODEL_HANDLER", &format!("找到Tch模型文件: '{}'", model_ot_path.display()), None);
        return Some(model_ot_path);
    }

    let checkpoint_dir = dir.join("checkpoint");
    if checkpoint_dir.exists() {
        let mut latest_epoch: usize = 0;
        let mut latest_path: Option<std::path::PathBuf> = None;

        if let Ok(entries) = std::fs::read_dir(&checkpoint_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() { continue; }
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                if name.starts_with("model-") {
                    let epoch_result = name
                        .strip_prefix("model-")
                        .and_then(|s| s.strip_suffix(".mpk").or_else(|| s.strip_suffix(".mpk.gz")).or_else(|| s.strip_suffix(".bin")))
                        .and_then(|s| s.parse::<usize>().ok());
                    if let Some(epoch) = epoch_result {
                        if epoch >= latest_epoch {
                            latest_epoch = epoch;
                            latest_path = Some(path);
                        }
                    }
                }
            }
        }

        if let Some(cp_path) = latest_path {
            log("MODEL_HANDLER", &format!("找到最新checkpoint: epoch={}, path='{}'", latest_epoch, cp_path.display()), None);
            return Some(cp_path);
        }
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            let is_symlink = path.symlink_metadata()
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false);
            if is_symlink {
                continue;
            }

            if path.is_dir() {
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                if dir_name == "checkpoint" || dir_name == "train" || dir_name == "valid" || dir_name == "exports" {
                    continue;
                }
                if let Some(found) = find_model_file_recursive(&path) {
                    return Some(found);
                }
                continue;
            }

            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("checkpoint-") || name.starts_with("model-") {
                return Some(path);
            }
            if name.ends_with(".mpk") || name.ends_with(".mpk.gz") || name.ends_with(".bin") || name.ends_with(".ot") {
                return Some(path);
            }
        }
    }

    if let Ok(train_dir) = std::fs::read_dir(dir.join("train")) {
        let mut best_epoch: usize = 0;
        let mut best_path: Option<std::path::PathBuf> = None;
        for entry in train_dir.flatten() {
            let epoch_dir = entry.path();
            if !epoch_dir.is_dir() { continue; }
            let dir_name = epoch_dir.file_name().unwrap_or_default().to_string_lossy().to_string();
            if let Some(epoch_str) = dir_name.strip_prefix("epoch-") {
                if let Ok(epoch_num) = epoch_str.parse::<usize>() {
                    if epoch_num > best_epoch {
                        best_epoch = epoch_num;
                        best_path = Some(epoch_dir);
                    }
                }
            }
        }
        if let Some(ref model_dir) = best_path {
            if let Ok(entries) = std::fs::read_dir(&model_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() { continue; }
                    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    if name.starts_with("checkpoint-") || name.starts_with("model-") {
                        return Some(path);
                    }
                }
            }
        }
    }

    log("MODEL_HANDLER", "WARN", Some(&format!("未找到模型文件，产物目录: '{}'", dir.display())));
    None
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
