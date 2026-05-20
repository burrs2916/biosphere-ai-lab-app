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
    model_repo: Option<Arc<dyn crate::domain::model::repository::ModelRepository>>,
    dataset_repo: Option<Arc<dyn crate::domain::dataset::repository::DatasetRepository>>,
    event_bus: Arc<EventBus>,
}

impl DefaultExperimentCommandHandler {
    pub fn new(repo: Arc<dyn ExperimentRepository>, event_bus: Arc<EventBus>) -> Self {
        Self { repo, model_repo: None, dataset_repo: None, event_bus }
    }

    pub fn with_model_repo(mut self, repo: Arc<dyn crate::domain::model::repository::ModelRepository>) -> Self {
        self.model_repo = Some(repo);
        self
    }

    pub fn with_dataset_repo(mut self, repo: Arc<dyn crate::domain::dataset::repository::DatasetRepository>) -> Self {
        self.dataset_repo = Some(repo);
        self
    }
}

#[async_trait]
impl ExperimentCommandHandler for DefaultExperimentCommandHandler {
    async fn handle(&self, cmd: ExperimentCommand) -> Result<Option<ExperimentId>> {
        match cmd {
            ExperimentCommand::CreateExperiment { name, task_type: _, config } => {
                crate::infrastructure::log("EXP_HANDLER", "---------- 处理CreateExperiment命令 ----------", None);
                crate::infrastructure::log("EXP_HANDLER", &format!("name='{}', engine='{}', model='{}', data_path='{}'",
                    name, config.engine_id, config.model_id, config.data_path), None);
                crate::infrastructure::log("EXP_HANDLER", &format!("config: epochs={}, batch_size={}, lr={}, validation_split={}", 
                    config.epochs, config.batch_size, config.learning_rate, config.validation_split), None);

                crate::infrastructure::log("EXP_HANDLER", "创建实验聚合对象...", None);
                let mut experiment = Experiment::create(name.clone(), config);
                experiment.environment = Some(EnvironmentInfo::capture_all());
                let id = experiment.id.clone();
                crate::infrastructure::log("EXP_HANDLER", &format!("实验对象创建成功: id={}, status={:?}", id, experiment.status), None);

                crate::infrastructure::log("EXP_HANDLER", "保存实验到仓库...", None);
                self.repo.save(&experiment).await.map_err(|e| {
                    crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("保存实验失败: {}", e)));
                    e
                })?;
                crate::infrastructure::log("EXP_HANDLER", "实验保存成功", None);

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentCreated".to_string(),
                    event_to_value(ExperimentEvent::ExperimentCreated {
                        experiment_id: id.clone(),
                        name,
                    }, "ExperimentCreated"),
                ));
                crate::infrastructure::log("EXP_HANDLER", "---------- CreateExperiment命令处理完成 ----------", None);
                Ok(Some(id))
            }

            ExperimentCommand::TrackMetric { experiment_id, metric_name, value, step } => {
                crate::infrastructure::log("EXP_HANDLER", &format!("TrackMetric: exp={}, metric='{}', value={:.4}, step={}", 
                    experiment_id, metric_name, value, step), None);
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
                crate::infrastructure::log("EXP_HANDLER", &format!("TrackMetricWithEpoch: exp={}, metric='{}', value={:.4}, step={}, epoch={}", 
                    experiment_id, metric_name, value, step, epoch), None);
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
                crate::infrastructure::log("EXP_HANDLER", "---------- 处理StartExperiment命令 ----------", None);
                crate::infrastructure::log("EXP_HANDLER", &format!("experiment_id={}", experiment_id), None);

                crate::infrastructure::log("EXP_HANDLER", "加载实验...", None);
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| {
                        crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("实验不存在: {}", experiment_id)));
                        LabError::Custom(format!("Experiment not found: {}", experiment_id))
                    })?;
                crate::infrastructure::log("EXP_HANDLER", &format!("实验加载成功: name='{}', status={:?}", experiment.name, experiment.status), None);

                crate::infrastructure::log("EXP_HANDLER", "更新实验状态为Running...", None);
                experiment.start().map_err(|e| {
                    crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("启动实验失败: {}", e)));
                    LabError::Custom(e)
                })?;
                self.repo.save(&experiment).await?;
                crate::infrastructure::log("EXP_HANDLER", "实验状态已更新为Running", None);

                crate::infrastructure::log("EXP_HANDLER", "捕获环境信息...", None);
                let env_info = EnvironmentInfo::capture_all();
                self.repo.save_environment(&experiment_id, &env_info).await?;
                crate::infrastructure::log("EXP_HANDLER", "环境信息已保存", None);

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentStarted".to_string(),
                    event_to_value(ExperimentEvent::ExperimentStarted {
                        experiment_id,
                    }, "ExperimentStarted"),
                ));
                crate::infrastructure::log("EXP_HANDLER", "---------- StartExperiment命令处理完成 ----------", None);
                Ok(None)
            }

            ExperimentCommand::RestartExperiment { experiment_id } => {
                crate::infrastructure::log("EXP_HANDLER", &format!("处理RestartExperiment命令: experiment_id={}", experiment_id), None);
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| {
                        crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("实验不存在: {}", experiment_id)));
                        LabError::Custom(format!("Experiment not found: {}", experiment_id))
                    })?;
                experiment.restart().map_err(|e| {
                    crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("重启实验失败: {}", e)));
                    LabError::Custom(e)
                })?;
                self.repo.save(&experiment).await?;

                let env_info = EnvironmentInfo::capture_all();
                self.repo.save_environment(&experiment_id, &env_info).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentRestarted".to_string(),
                    event_to_value(ExperimentEvent::ExperimentStarted {
                        experiment_id,
                    }, "ExperimentRestarted"),
                ));
                crate::infrastructure::log("EXP_HANDLER", "实验重启成功", None);
                Ok(None)
            }

            ExperimentCommand::PauseExperiment { experiment_id } => {
                crate::infrastructure::log("EXP_HANDLER", &format!("处理PauseExperiment命令: experiment_id={}", experiment_id), None);
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| {
                        crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("实验不存在: {}", experiment_id)));
                        LabError::Custom(format!("Experiment not found: {}", experiment_id))
                    })?;
                experiment.pause().map_err(|e| {
                    crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("暂停实验失败: {}", e)));
                    LabError::Custom(e)
                })?;
                self.repo.save(&experiment).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentPaused".to_string(),
                    event_to_value(ExperimentEvent::ExperimentPaused {
                        experiment_id,
                    }, "ExperimentPaused"),
                ));
                crate::infrastructure::log("EXP_HANDLER", "实验暂停成功", None);
                Ok(None)
            }

            ExperimentCommand::ResumeExperiment { experiment_id } => {
                crate::infrastructure::log("EXP_HANDLER", &format!("处理ResumeExperiment命令: experiment_id={}", experiment_id), None);
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| {
                        crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("实验不存在: {}", experiment_id)));
                        LabError::Custom(format!("Experiment not found: {}", experiment_id))
                    })?;
                experiment.resume().map_err(|e| {
                    crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("恢复实验失败: {}", e)));
                    LabError::Custom(e)
                })?;
                self.repo.save(&experiment).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentResumed".to_string(),
                    event_to_value(ExperimentEvent::ExperimentResumed {
                        experiment_id,
                    }, "ExperimentResumed"),
                ));
                crate::infrastructure::log("EXP_HANDLER", "实验恢复成功", None);
                Ok(None)
            }

            ExperimentCommand::CompleteExperiment { experiment_id, final_metrics } => {
                crate::infrastructure::log("EXP_HANDLER", "---------- 处理CompleteExperiment命令 ----------", None);
                crate::infrastructure::log("EXP_HANDLER", &format!("experiment_id={}, metrics_count={}, metrics_keys={:?}",
                    experiment_id,
                    final_metrics.as_object().map_or(0, |o| o.len()),
                    final_metrics.as_object().map_or(vec![], |o| o.keys().take(10).cloned().collect::<Vec<_>>())
                ), None);

                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| {
                        crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("实验不存在: {}", experiment_id)));
                        LabError::Custom(format!("Experiment not found: {}", experiment_id))
                    })?;
                crate::infrastructure::log("EXP_HANDLER", &format!("实验加载成功: name='{}', status={:?}, metrics_in_memory={}", experiment.name, experiment.status, experiment.metrics.all_series().len()), None);

                experiment.complete(final_metrics.clone()).map_err(|e| {
                    crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("完成实验失败: {}", e)));
                    LabError::Custom(e)
                })?;
                self.repo.save(&experiment).await?;
                crate::infrastructure::log("EXP_HANDLER", &format!("实验状态已更新为Completed, dataset_id={:?}, model_id={:?}", experiment.dataset_id, experiment.model_id), None);

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentCompleted".to_string(),
                    event_to_value(ExperimentEvent::ExperimentCompleted {
                        experiment_id,
                        final_metrics,
                    }, "ExperimentCompleted"),
                ));
                crate::infrastructure::log("EXP_HANDLER", "---------- CompleteExperiment命令处理完成 ----------", None);
                Ok(None)
            }

            ExperimentCommand::FailExperiment { experiment_id, error } => {
                crate::infrastructure::log("EXP_HANDLER", "---------- 处理FailExperiment命令 ----------", None);
                crate::infrastructure::log("EXP_HANDLER", &format!("experiment_id={}, error='{}'", experiment_id, error), None);

                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| {
                        crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("实验不存在: {}", experiment_id)));
                        LabError::Custom(format!("Experiment not found: {}", experiment_id))
                    })?;
                crate::infrastructure::log("EXP_HANDLER", &format!("实验加载成功: name='{}', status={:?}", experiment.name, experiment.status), None);

                experiment.fail(error.clone()).map_err(|e| {
                    crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("标记实验失败失败: {}", e)));
                    LabError::Custom(e)
                })?;
                self.repo.save(&experiment).await?;
                crate::infrastructure::log("EXP_HANDLER", "实验状态已更新为Failed", None);

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentFailed".to_string(),
                    event_to_value(ExperimentEvent::ExperimentFailed {
                        experiment_id,
                        error,
                    }, "ExperimentFailed"),
                ));
                crate::infrastructure::log("EXP_HANDLER", "---------- FailExperiment命令处理完成 ----------", None);
                Ok(None)
            }

            ExperimentCommand::CancelExperiment { experiment_id } => {
                crate::infrastructure::log("EXP_HANDLER", &format!("处理CancelExperiment命令: experiment_id={}", experiment_id), None);
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| {
                        crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("实验不存在: {}", experiment_id)));
                        LabError::Custom(format!("Experiment not found: {}", experiment_id))
                    })?;

                experiment.cancel().map_err(|e| {
                    crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("取消实验失败: {}", e)));
                    LabError::Custom(e)
                })?;
                self.repo.save(&experiment).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentCancelled".to_string(),
                    event_to_value(ExperimentEvent::ExperimentCancelled {
                        experiment_id,
                    }, "ExperimentCancelled"),
                ));
                crate::infrastructure::log("EXP_HANDLER", "实验取消成功", None);
                Ok(None)
            }

            ExperimentCommand::SetParam { experiment_id, key, value } => {
                crate::infrastructure::log("EXP_HANDLER", &format!("SetParam: exp={}, key='{}', value={:?}", experiment_id, key, value), None);
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
                crate::infrastructure::log("EXP_HANDLER", &format!("AddTag: exp={}, tag='{}'", experiment_id, tag), None);
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
                crate::infrastructure::log("EXP_HANDLER", &format!("处理DeleteExperiment命令: experiment_id={}", experiment_id), None);
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| {
                        crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("实验不存在: {}", experiment_id)));
                        LabError::Custom(format!("Experiment not found: {}", experiment_id))
                    })?;

                if experiment.status == super::aggregate::ExperimentStatus::Running
                    || experiment.status == super::aggregate::ExperimentStatus::Paused
                {
                    crate::infrastructure::log("EXP_HANDLER", "WARN", Some(&format!("实验状态为{:?}，标记为中断后继续删除", experiment.status)));
                    experiment.status = super::aggregate::ExperimentStatus::Failed;
                    experiment.error_message = Some("Force deleted while in running/paused state (orphaned session).".to_string());
                    experiment.completed_at = Some(chrono::Utc::now());
                    let _ = self.repo.save(&experiment).await;
                }

                if let Some(ref model_repo) = self.model_repo {
                    if let Some(ref model_id) = experiment.model_id {
                        crate::infrastructure::log("EXP_HANDLER", &format!("清理关联模型: model_id={}", model_id), None);
                        match model_repo.delete(model_id).await {
                            Ok(_) => crate::infrastructure::log("EXP_HANDLER", "关联模型删除成功", None),
                            Err(e) => crate::infrastructure::log("EXP_HANDLER", "WARN", Some(&format!("关联模型删除失败(非致命): {}", e))),
                        }
                    }
                }

                if let Some(ref dataset_repo) = self.dataset_repo {
                    if let Some(ref dataset_id) = experiment.dataset_id {
                        crate::infrastructure::log("EXP_HANDLER", &format!("清理数据集关联: dataset_id={}", dataset_id), None);
                        if let Ok(Some(mut dataset)) = dataset_repo.load(
                            &crate::domain::dataset::aggregate::DatasetId::from_str(dataset_id)
                        ).await {
                            dataset.unlink_experiment(experiment_id.as_str());
                            if let Err(e) = dataset_repo.save(&dataset).await {
                                crate::infrastructure::log("EXP_HANDLER", "WARN", Some(&format!("数据集关联清理失败(非致命): {}", e)));
                            } else {
                                crate::infrastructure::log("EXP_HANDLER", "数据集关联清理成功", None);
                            }
                        }
                    }
                }

                self.repo.delete(&experiment_id).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentDeleted".to_string(),
                    event_to_value(ExperimentEvent::ExperimentDeleted {
                        experiment_id,
                    }, "ExperimentDeleted"),
                ));
                crate::infrastructure::log("EXP_HANDLER", "实验删除成功", None);
                Ok(None)
            }

            ExperimentCommand::SetDescription { experiment_id, description } => {
                crate::infrastructure::log("EXP_HANDLER", &format!("SetDescription: exp={}, len={}", experiment_id, description.len()), None);
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                experiment.set_description(description);
                self.repo.save(&experiment).await?;

                Ok(None)
            }

            ExperimentCommand::RemoveTag { experiment_id, tag } => {
                crate::infrastructure::log("EXP_HANDLER", &format!("RemoveTag: exp={}, tag='{}'", experiment_id, tag), None);
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                experiment.remove_tag(&tag);
                self.repo.save(&experiment).await?;

                Ok(None)
            }

            ExperimentCommand::CloneExperiment { experiment_id, new_name } => {
                crate::infrastructure::log("EXP_HANDLER", "---------- 处理CloneExperiment命令 ----------", None);
                crate::infrastructure::log("EXP_HANDLER", &format!("source_id={}, new_name='{}'", experiment_id, new_name), None);

                let source = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| {
                        crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("源实验不存在: {}", experiment_id)));
                        LabError::Custom(format!("Experiment not found: {}", experiment_id))
                    })?;
                crate::infrastructure::log("EXP_HANDLER", &format!("源实验加载成功: name='{}', tags={}, params={}", 
                    source.name, source.tags.len(), source.params.len()), None);

                crate::infrastructure::log("EXP_HANDLER", "创建克隆实验...", None);
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
                crate::infrastructure::log("EXP_HANDLER", &format!("克隆实验创建成功: new_id={}", new_id), None);

                self.repo.save(&cloned).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentCloned".to_string(),
                    event_to_value(ExperimentEvent::ExperimentCloned {
                        source_experiment_id: experiment_id,
                        new_experiment_id: new_id.clone(),
                        new_name,
                    }, "ExperimentCloned"),
                ));
                crate::infrastructure::log("EXP_HANDLER", "---------- CloneExperiment命令处理完成 ----------", None);
                Ok(Some(new_id))
            }

            ExperimentCommand::AddArtifact { experiment_id, artifact } => {
                crate::infrastructure::log("EXP_HANDLER", &format!("AddArtifact: exp={}, artifact_type='{}', path='{}', size={}",
                    experiment_id, artifact.artifact_type, artifact.path, artifact.size_bytes), None);
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                let artifact_count_before = experiment.artifacts.len();
                experiment.add_artifact(artifact);
                self.repo.save(&experiment).await?;
                crate::infrastructure::log("EXP_HANDLER", &format!("产物添加成功: artifacts总数 {} -> {}", artifact_count_before, experiment.artifacts.len()), None);

                Ok(None)
            }

            ExperimentCommand::ArchiveExperiment { experiment_id } => {
                crate::infrastructure::log("EXP_HANDLER", &format!("处理ArchiveExperiment命令: experiment_id={}", experiment_id), None);
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| {
                        crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("实验不存在: {}", experiment_id)));
                        LabError::Custom(format!("Experiment not found: {}", experiment_id))
                    })?;

                experiment.archive().map_err(|e| {
                    crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("归档实验失败: {}", e)));
                    LabError::Custom(e)
                })?;
                self.repo.save(&experiment).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentArchived".to_string(),
                    event_to_value(ExperimentEvent::ExperimentArchived {
                        experiment_id,
                    }, "ExperimentArchived"),
                ));
                crate::infrastructure::log("EXP_HANDLER", "实验归档成功", None);
                Ok(None)
            }

            ExperimentCommand::RestoreExperiment { experiment_id } => {
                crate::infrastructure::log("EXP_HANDLER", &format!("处理RestoreExperiment命令: experiment_id={}", experiment_id), None);
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| {
                        crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("实验不存在: {}", experiment_id)));
                        LabError::Custom(format!("Experiment not found: {}", experiment_id))
                    })?;

                experiment.restore().map_err(|e| {
                    crate::infrastructure::log("EXP_HANDLER", "ERROR", Some(&format!("恢复实验失败: {}", e)));
                    LabError::Custom(e)
                })?;
                self.repo.save(&experiment).await?;

                self.event_bus.emit(LabEvent::Custom(
                    "ExperimentRestored".to_string(),
                    event_to_value(ExperimentEvent::ExperimentRestored {
                        experiment_id,
                    }, "ExperimentRestored"),
                ));
                crate::infrastructure::log("EXP_HANDLER", "实验恢复成功", None);
                Ok(None)
            }

            ExperimentCommand::LinkDataset { experiment_id, dataset_id, dataset_version } => {
                crate::infrastructure::log("EXP_HANDLER", &format!("LinkDataset: exp={}, dataset={}, version={:?}", 
                    experiment_id, dataset_id, dataset_version), None);
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                crate::infrastructure::log("EXP_HANDLER", &format!("关联前: experiment.dataset_id={:?}", experiment.dataset_id), None);
                experiment.link_dataset(dataset_id.clone(), dataset_version.clone().unwrap_or_else(|| "1.0.0".to_string()));
                self.repo.save(&experiment).await?;
                crate::infrastructure::log("EXP_HANDLER", &format!("数据集关联成功: experiment.dataset_id={:?}", experiment.dataset_id), None);
                Ok(None)
            }

            ExperimentCommand::SetGroup { experiment_id, group } => {
                crate::infrastructure::log("EXP_HANDLER", &format!("SetGroup: exp={}, group='{}'", experiment_id, group), None);
                let mut experiment = self.repo.load(&experiment_id).await?
                    .ok_or_else(|| LabError::Custom(format!("Experiment not found: {}", experiment_id)))?;

                experiment.set_group(group);
                self.repo.save(&experiment).await?;
                crate::infrastructure::log("EXP_HANDLER", "分组设置成功", None);
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
