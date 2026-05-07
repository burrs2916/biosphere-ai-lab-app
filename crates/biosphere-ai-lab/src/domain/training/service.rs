use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::core::EventBus;
use crate::core::config::TrainingConfig;
use crate::core::event::LabEvent;
use crate::domain::experiment::aggregate::ExperimentId;
use crate::domain::experiment::repository::ExperimentRepository;
use crate::domain::experiment::commands::ExperimentCommand;
use crate::domain::experiment::handler::ExperimentCommandHandler;
use crate::domain::experiment::ArtifactRef;
use crate::domain::dataset::repository::DatasetRepository;
use crate::domain::dataset::aggregate::DatasetId;
use crate::domain::hardware::ResourceMonitor;
use crate::domain::model::commands::ModelCommand;
use crate::domain::model::handler::ModelCommandHandler;
use crate::engine::{EngineRegistry, SessionHandle};
use crate::types::SessionId;
use crate::infrastructure::log;

#[allow(dead_code)]
struct ActiveSession {
    experiment_id: ExperimentId,
    session_id: SessionId,
    handle: SessionHandle,
    config: TrainingConfig,
    engine_id: String,
    resource_monitor: Arc<ResourceMonitor>,
}

pub struct TrainingService {
    event_bus: Arc<EventBus>,
    #[allow(dead_code)]
    experiment_repo: Arc<dyn ExperimentRepository>,
    dataset_repo: Option<Arc<dyn DatasetRepository>>,
    experiment_handler: Arc<dyn ExperimentCommandHandler>,
    model_handler: Arc<dyn ModelCommandHandler>,
    engine_registry: Arc<EngineRegistry>,
    active_sessions: Arc<RwLock<HashMap<String, ActiveSession>>>,
    max_concurrent_experiments: usize,
}

struct EventLoopParams {
    event_bus: Arc<EventBus>,
    experiment_handler: Arc<dyn ExperimentCommandHandler>,
    model_handler: Arc<dyn ModelCommandHandler>,
    active_sessions: Arc<RwLock<HashMap<String, ActiveSession>>>,
    experiment_id: ExperimentId,
    session_id: SessionId,
    resource_monitor: Arc<ResourceMonitor>,
    log_prefix: String,
    experiment_repo: Arc<dyn ExperimentRepository>,
}

fn spawn_event_loop(params: EventLoopParams) {
    let EventLoopParams {
        event_bus,
        experiment_handler,
        model_handler,
        active_sessions,
        experiment_id,
        session_id,
        resource_monitor,
        log_prefix,
        experiment_repo,
    } = params;

    let mut rx = event_bus.subscribe();
    let mut last_event_time = std::time::Instant::now();
    let session_timeout = std::time::Duration::from_secs(300);
    let heartbeat_interval = std::time::Duration::from_secs(15);
    let mut last_heartbeat = std::time::Instant::now();
    let training_start = std::time::Instant::now();
    let mut current_epoch: usize = 0;
    let mut current_total_epochs: usize = 0;

    tokio::spawn(async move {
        loop {
            let event = match tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv()).await {
                Ok(Ok(event)) => {
                    last_event_time = std::time::Instant::now();
                    event
                }
                Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(n))) => {
                    log("TRAINING", &format!("EventBus lagged by {} events", n), None);
                    continue;
                }
                Ok(Err(tokio::sync::broadcast::error::RecvError::Closed)) => {
                    log("TRAINING", &format!("EventBus closed for {}", experiment_id), None);
                    break;
                }
                Err(_) => {
                    if last_heartbeat.elapsed() >= heartbeat_interval {
                        last_heartbeat = std::time::Instant::now();
                        event_bus.emit(LabEvent::Heartbeat {
                            session_id: session_id.clone(),
                            epoch: current_epoch,
                            total_epochs: current_total_epochs,
                            elapsed_secs: training_start.elapsed().as_secs_f64(),
                        });
                    }
                    if last_event_time.elapsed() > session_timeout {
                        log("TRAINING", &format!("Session timeout, no events for {:?}: {}", session_timeout, experiment_id), None);
                        if let Err(e) = experiment_handler.handle(ExperimentCommand::FailExperiment {
                            experiment_id: experiment_id.clone(),
                            error: format!("Session timed out - no events received for {:?}", session_timeout),
                        }).await {
                            log("TRAINING", &format!("Failed to mark experiment as failed: {}", e), None);
                        }
                        break;
                    }
                    continue;
                }
            };

            match &event {
                LabEvent::EpochCompleted { session_id: sid, epoch, total_epochs, train_loss, val_loss, metrics } => {
                    if sid == &session_id {
                        current_epoch = *epoch;
                        current_total_epochs = *total_epochs;

                        if let Err(e) = experiment_handler.handle(ExperimentCommand::TrackMetricWithEpoch {
                            experiment_id: experiment_id.clone(),
                            metric_name: "train_loss".to_string(),
                            value: *train_loss,
                            step: *epoch as u64,
                            epoch: *epoch,
                        }).await {
                            log("TRAINING", &format!("Failed to track train_loss: {}", e), None);
                        }

                        if let Some(vl) = val_loss {
                            if let Err(e) = experiment_handler.handle(ExperimentCommand::TrackMetricWithEpoch {
                                experiment_id: experiment_id.clone(),
                                metric_name: "val_loss".to_string(),
                                value: *vl,
                                step: *epoch as u64,
                                epoch: *epoch,
                            }).await {
                                log("TRAINING", &format!("Failed to track val_loss: {}", e), None);
                            }
                        }

                        if let Some(acc) = metrics.get("train_accuracy").and_then(|v| v.as_f64()) {
                            if let Err(e) = experiment_handler.handle(ExperimentCommand::TrackMetricWithEpoch {
                                experiment_id: experiment_id.clone(),
                                metric_name: "accuracy".to_string(),
                                value: acc,
                                step: *epoch as u64,
                                epoch: *epoch,
                            }).await {
                                log("TRAINING", &format!("Failed to track accuracy: {}", e), None);
                            }
                        }

                        if let Some(val_acc) = metrics.get("val_accuracy").and_then(|v| v.as_f64()) {
                            if let Err(e) = experiment_handler.handle(ExperimentCommand::TrackMetricWithEpoch {
                                experiment_id: experiment_id.clone(),
                                metric_name: "val_accuracy".to_string(),
                                value: val_acc,
                                step: *epoch as u64,
                                epoch: *epoch,
                            }).await {
                                log("TRAINING", &format!("Failed to track val_accuracy: {}", e), None);
                            }
                        }

                        for (key, value) in metrics.as_object().unwrap_or(&serde_json::Map::new()) {
                            if !matches!(key.as_str(), "train_accuracy" | "val_accuracy") {
                                if let Some(v) = value.as_f64() {
                                    if let Err(e) = experiment_handler.handle(ExperimentCommand::TrackMetricWithEpoch {
                                        experiment_id: experiment_id.clone(),
                                        metric_name: key.clone(),
                                        value: v,
                                        step: *epoch as u64,
                                        epoch: *epoch,
                                    }).await {
                                        log("TRAINING", &format!("Failed to track custom metric '{}': {}", key, e), None);
                                    }
                                }
                            }
                        }

                        log("TRAINING", &format!(
                            "{} Epoch {}/{} | loss={:.4} val_loss={:.4}",
                            log_prefix, epoch, total_epochs, train_loss,
                            val_loss.unwrap_or(0.0)
                        ), None);
                    }
                }
                LabEvent::SessionCompleted { session_id: sid, final_metrics } => {
                    if sid == &session_id {
                        log("TRAINING", &format!("{}训练完成: {}", log_prefix, experiment_id), None);

                        let collected_final_metrics = if final_metrics.as_object().map_or(true, |o| o.is_empty()) {
                            match experiment_repo.load(&experiment_id).await {
                                Ok(Some(exp)) => {
                                    let mut m = serde_json::Map::new();
                                    for (name, series) in exp.metrics.all_series() {
                                        if let Some(last) = series.values.last() {
                                            m.insert(name.clone(), serde_json::Value::from(last.value));
                                        }
                                    }
                                    serde_json::Value::Object(m)
                                }
                                _ => final_metrics.clone(),
                            }
                        } else {
                            final_metrics.clone()
                        };

                        if let Err(e) = experiment_handler.handle(ExperimentCommand::CompleteExperiment {
                            experiment_id: experiment_id.clone(),
                            final_metrics: collected_final_metrics,
                        }).await {
                            log("TRAINING", &format!("Failed to complete experiment: {}", e), None);
                        }

                        let model_name = format!("{}_model", experiment_id);
                        match model_handler.handle(ModelCommand::RegisterModelFromExperiment {
                            experiment_id: experiment_id.clone(),
                            name: model_name,
                            version: "auto".to_string(),
                        }).await {
                            Ok(_) => {
                                log("TRAINING", &format!("自动注册模型成功: {}", experiment_id), None);
                            }
                            Err(e) => {
                                log("TRAINING", &format!("自动注册模型失败: {} - {}", experiment_id, e), None);
                            }
                        }

                        scan_and_register_artifacts(&experiment_id, &experiment_handler).await;

                        break;
                    }
                }
                LabEvent::SessionFailed { session_id: sid, error } => {
                    if sid == &session_id {
                        log("TRAINING", &format!("{}训练失败: {} - {}", log_prefix, experiment_id, error), None);

                        if let Err(e) = experiment_handler.handle(ExperimentCommand::FailExperiment {
                            experiment_id: experiment_id.clone(),
                            error: error.clone(),
                        }).await {
                            log("TRAINING", &format!("Failed to mark experiment as failed: {}", e), None);
                        }

                        break;
                    }
                }
                LabEvent::SessionCancelled { session_id: sid } => {
                    if sid == &session_id {
                        log("TRAINING", &format!("{}训练取消: {}", log_prefix, experiment_id), None);

                        if let Err(e) = experiment_handler.handle(ExperimentCommand::CancelExperiment {
                            experiment_id: experiment_id.clone(),
                        }).await {
                            log("TRAINING", &format!("Failed to cancel experiment: {}", e), None);
                        }

                        break;
                    }
                }
                LabEvent::LogOutput { session_id: sid, level, message } => {
                    if sid == &session_id {
                        if let Err(e) = experiment_repo.save_log(&experiment_id, level, message).await {
                            log("TRAINING", &format!("Failed to save log: {}", e), None);
                        }
                    }
                }
                LabEvent::BatchCompleted { session_id: sid, batch, total_batches, loss } => {
                    if sid == &session_id {
                        if let Err(e) = experiment_handler.handle(ExperimentCommand::TrackMetric {
                            experiment_id: experiment_id.clone(),
                            metric_name: "batch_loss".to_string(),
                            value: *loss,
                            step: *batch as u64,
                        }).await {
                            log("TRAINING", &format!("Failed to track batch_loss: {}", e), None);
                        }
                        if batch % 10 == 0 || batch == total_batches {
                            event_bus.emit(LabEvent::ProgressUpdate {
                                session_id: session_id.clone(),
                                progress: *batch as f64 / *total_batches as f64,
                                message: format!("Batch {}/{} loss={:.4}", batch, total_batches, loss),
                            });
                        }
                    }
                }
                LabEvent::CheckpointSaved { session_id: sid, path, epoch } => {
                    if sid == &session_id {
                        let artifact = ArtifactRef::new(
                            "checkpoint".to_string(),
                            path.clone(),
                            std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
                        );
                        if let Err(e) = experiment_handler.handle(ExperimentCommand::AddArtifact {
                            experiment_id: experiment_id.clone(),
                            artifact,
                        }).await {
                            log("TRAINING", &format!("Failed to add checkpoint artifact: {}", e), None);
                        }
                        log("TRAINING", &format!("{}Checkpoint saved: epoch={}, path={}", log_prefix, epoch, path), None);
                    }
                }
                _ => {}
            }
        }

        cleanup_session(&active_sessions, &experiment_id, &resource_monitor).await;
    });
}

async fn scan_and_register_artifacts(
    experiment_id: &ExperimentId,
    experiment_handler: &Arc<dyn ExperimentCommandHandler>,
) {
    let artifact_dir = crate::core::config::get_artifact_dir(&experiment_id.to_string());
    if let Ok(entries) = std::fs::read_dir(&artifact_dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            let is_symlink = path.symlink_metadata()
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false);
            if is_symlink {
                continue;
            }

            if !path.is_file() {
                continue;
            }
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let size = path.metadata().map(|m| m.len()).unwrap_or(0);
            let artifact_type = if name.starts_with("checkpoint-") {
                "checkpoint"
            } else if name.starts_with("model-") {
                "model"
            } else if name.ends_with(".json") {
                "metadata"
            } else if name.ends_with(".log") {
                "log"
            } else {
                "other"
            };
            let artifact = ArtifactRef::new(
                artifact_type.to_string(),
                path.to_string_lossy().to_string(),
                size,
            );
            if let Err(e) = experiment_handler.handle(ExperimentCommand::AddArtifact {
                experiment_id: experiment_id.clone(),
                artifact,
            }).await {
                log("TRAINING", &format!("Failed to add artifact: {}", e), None);
            }
        }
        log("TRAINING", &format!("Artifact扫描完成: {}", experiment_id), None);
    }
}

async fn cleanup_session(
    active_sessions: &Arc<RwLock<HashMap<String, ActiveSession>>>,
    experiment_id: &ExperimentId,
    resource_monitor: &Arc<ResourceMonitor>,
) {
    resource_monitor.stop().await;
    resource_monitor.set_session_id(None).await;
    let mut sessions = active_sessions.write().await;
    sessions.remove(&experiment_id.to_string());
}

impl TrainingService {
    pub fn new(
        event_bus: Arc<EventBus>,
        experiment_repo: Arc<dyn ExperimentRepository>,
        model_handler: Arc<dyn ModelCommandHandler>,
    ) -> Self {
        Self {
            event_bus: event_bus.clone(),
            experiment_repo: experiment_repo.clone(),
            dataset_repo: None,
            experiment_handler: Arc::new(crate::domain::experiment::handler::DefaultExperimentCommandHandler::new(
                experiment_repo,
                event_bus,
            )),
            model_handler,
            engine_registry: Arc::new(EngineRegistry::new()),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent_experiments: 1,
        }
    }

    pub fn with_engine_registry(
        event_bus: Arc<EventBus>,
        experiment_repo: Arc<dyn ExperimentRepository>,
        experiment_handler: Arc<dyn ExperimentCommandHandler>,
        model_handler: Arc<dyn ModelCommandHandler>,
        engine_registry: Arc<EngineRegistry>,
    ) -> Self {
        Self {
            event_bus,
            experiment_repo,
            dataset_repo: None,
            experiment_handler,
            model_handler,
            engine_registry,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent_experiments: 1,
        }
    }

    pub fn with_dataset_repo(mut self, repo: Arc<dyn DatasetRepository>) -> Self {
        self.dataset_repo = Some(repo);
        self
    }

    pub fn set_dataset_repo(&mut self, repo: Arc<dyn DatasetRepository>) {
        self.dataset_repo = Some(repo);
    }

    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    pub fn set_max_concurrent_experiments(&mut self, max: usize) {
        self.max_concurrent_experiments = max.max(1);
    }

    pub async fn start_training(
        &self,
        experiment_id: ExperimentId,
        mut config: TrainingConfig,
    ) -> Result<(), String> {
        let exp_id_str = experiment_id.to_string();

        if let Some(ref dataset_id_str) = config.dataset_id {
            if let Some(ref dataset_repo) = self.dataset_repo {
                let ds_id = DatasetId::from_str(dataset_id_str);
                let dataset = dataset_repo.load(&ds_id).await
                    .map_err(|e| format!("Failed to load dataset: {}", e))?
                    .ok_or_else(|| format!("Dataset not found: {}", dataset_id_str))?;

                if dataset.status != crate::domain::dataset::aggregate::DatasetStatus::Active {
                    return Err(format!(
                        "Dataset '{}' is in {} status, must be Active to train",
                        dataset_id_str, dataset.status
                    ));
                }

                if config.data_path.is_empty() {
                    config.data_path = dataset.path.clone();
                }
                if config.data_format != dataset.format {
                    config.data_format = dataset.format;
                }

                if config.feature_columns.is_empty() {
                    config.feature_columns = dataset.numeric_columns()
                        .iter()
                        .map(|p| p.name.clone())
                        .collect();
                }

                if let Some(ref version) = config.dataset_version {
                    if version != dataset.version.as_str() {
                        log("TRAINING", &format!(
                            "Warning: requested dataset version {} but current is {}",
                            version, dataset.version
                        ), None);
                    }
                }
                config.dataset_version = Some(dataset.version.to_string());

                if let Some(ref split_name) = config.split_name {
                    match dataset_repo.load_split(&ds_id, split_name).await {
                        Ok(Some(split)) => {
                            config.split_indices = Some(crate::core::config::SplitIndices {
                                split_name: split.name.clone(),
                                train_indices: split.train_indices.clone(),
                                val_indices: split.val_indices.clone(),
                                test_indices: split.test_indices.clone(),
                            });
                            config.validation_split = 0.0;
                            config.test_split = 0.0;
                            log("TRAINING", &format!(
                                "Resolved split '{}': train={}, val={}, test={}",
                                split_name, split.train_count(), split.val_count(), split.test_count()
                            ), None);
                        }
                        Ok(None) => {
                            return Err(format!("Split '{}' not found in dataset '{}'", split_name, dataset_id_str));
                        }
                        Err(e) => {
                            return Err(format!("Failed to load split '{}': {}", split_name, e));
                        }
                    }
                }

                log("TRAINING", &format!(
                    "Resolved dataset: id={}, version={}, path={}, rows={}, cols={}",
                    dataset_id_str, dataset.version, dataset.path, dataset.rows, dataset.columns
                ), None);
            } else {
                log("TRAINING", &format!(
                    "Warning: dataset_id '{}' specified but no dataset_repo configured",
                    dataset_id_str
                ), None);
            }
        }

        if config.epochs == 0 {
            return Err("Training epochs must be greater than 0".to_string());
        }
        if config.batch_size == 0 {
            return Err("Batch size must be greater than 0".to_string());
        }
        if config.learning_rate <= 0.0 {
            return Err("Learning rate must be greater than 0".to_string());
        }
        if config.batch_size > 4096 {
            return Err("Batch size cannot exceed 4096".to_string());
        }
        if config.epochs > 10000 {
            return Err("Epochs cannot exceed 10000".to_string());
        }
        if config.validation_split + config.test_split >= 1.0 {
            return Err(format!(
                "Validation split ({}) + test split ({}) must be less than 1.0",
                config.validation_split, config.test_split
            ));
        }
        if config.validation_split < 0.0 || config.test_split < 0.0 {
            return Err("Validation split and test split must be non-negative".to_string());
        }
        if !config.data_path.is_empty() {
            let data_path = std::path::Path::new(&config.data_path);
            if !data_path.exists() {
                return Err(format!("Data path does not exist: {}", config.data_path));
            }
        }

        log("TRAINING", &format!("启动训练: experiment={}", exp_id_str), None);

        let engine = self.engine_registry.find_by_id_str(&config.engine_id).await
            .ok_or_else(|| format!("Engine '{}' not found in registry", config.engine_id))?;

        let handle = engine.create_session(&config).await
            .map_err(|e| format!("Failed to create session: {}", e))?;

        let session_id = handle.session_id.clone();

        self.event_bus.emit(LabEvent::SessionCreated {
            session_id: session_id.clone(),
        });

        self.event_bus.emit(LabEvent::Custom(
            "SessionExperimentMapping".to_string(),
            serde_json::json!({
                "session_id": session_id.to_string(),
                "experiment_id": experiment_id.to_string(),
            }),
        ));

        self.experiment_handler.handle(ExperimentCommand::StartExperiment {
            experiment_id: experiment_id.clone(),
        }).await.map_err(|e| e.to_string())?;

        if !config.data_source_id.is_empty() {
            let _ = self.experiment_handler.handle(ExperimentCommand::LinkDataset {
                experiment_id: experiment_id.clone(),
                dataset_id: config.data_source_id.clone(),
                dataset_version: None,
            }).await;
        }

        let resource_monitor = Arc::new(ResourceMonitor::new(self.event_bus.clone()).with_interval(3));
        let engine_id = config.engine_id.clone();

        {
            let mut sessions = self.active_sessions.write().await;
            if sessions.contains_key(&exp_id_str) {
                let _ = self.experiment_handler.handle(ExperimentCommand::FailExperiment {
                    experiment_id: experiment_id.clone(),
                    error: "Duplicate training session detected".to_string(),
                }).await;
                return Err(format!("Training session already active for experiment: {}", experiment_id));
            }
            if sessions.len() >= self.max_concurrent_experiments {
                let _ = self.experiment_handler.handle(ExperimentCommand::FailExperiment {
                    experiment_id: experiment_id.clone(),
                    error: "Maximum concurrent experiments limit reached".to_string(),
                }).await;
                return Err(format!(
                    "Maximum concurrent experiments limit reached ({}/{}). Stop an existing experiment before starting a new one.",
                    sessions.len(), self.max_concurrent_experiments
                ));
            }
            sessions.insert(exp_id_str.clone(), ActiveSession {
                experiment_id: experiment_id.clone(),
                session_id: session_id.clone(),
                handle: handle.clone(),
                config: config.clone(),
                engine_id: engine_id.clone(),
                resource_monitor: resource_monitor.clone(),
            });
        }

        engine.start(handle.clone()).await
            .map_err(|e| {
                let error_msg = e.to_string();
                let exp_id = experiment_id.clone();
                let exp_id_str = exp_id_str.clone();
                let handler = self.experiment_handler.clone();
                let sessions = self.active_sessions.clone();
                let fail_reason = error_msg.clone();
                tokio::spawn(async move {
                    let _ = handler.handle(ExperimentCommand::FailExperiment {
                        experiment_id: exp_id,
                        error: format!("Failed to start training engine: {}", fail_reason),
                    }).await;
                    let mut s = sessions.write().await;
                    s.remove(&exp_id_str);
                });
                format!("Failed to start training: {}", error_msg)
            })?;

        resource_monitor.start().await;
        resource_monitor.set_session_id(Some(session_id.clone())).await;

        spawn_event_loop(EventLoopParams {
            event_bus: self.event_bus.clone(),
            experiment_handler: self.experiment_handler.clone(),
            model_handler: self.model_handler.clone(),
            active_sessions: self.active_sessions.clone(),
            experiment_id: experiment_id.clone(),
            session_id,
            resource_monitor,
            log_prefix: String::new(),
            experiment_repo: self.experiment_repo.clone(),
        });

        Ok(())
    }

    pub async fn stop_training(&self, experiment_id: &ExperimentId) -> Result<(), String> {
        let (engine_id, handle) = {
            let sessions = self.active_sessions.read().await;
            let session = sessions.get(&experiment_id.to_string())
                .ok_or_else(|| format!("No active training session for experiment: {}", experiment_id))?;
            (session.engine_id.clone(), session.handle.clone())
        };

        let engine = self.engine_registry.find_by_id_str(&engine_id).await
            .ok_or_else(|| format!("Engine '{}' not found", engine_id))?;

        engine.stop(handle).await.map_err(|e| e.to_string())?;

        log("TRAINING", &format!("停止训练: {} (等待event loop处理状态变更)", experiment_id), None);
        Ok(())
    }

    pub async fn pause_training(&self, experiment_id: &ExperimentId) -> Result<(), String> {
        let (engine_id, handle) = {
            let sessions = self.active_sessions.read().await;
            let session = sessions.get(&experiment_id.to_string())
                .ok_or_else(|| format!("No active training session for experiment: {}", experiment_id))?;
            (session.engine_id.clone(), session.handle.clone())
        };

        let engine = self.engine_registry.find_by_id_str(&engine_id).await
            .ok_or_else(|| format!("Engine '{}' not found", engine_id))?;

        engine.pause(handle).await.map_err(|e| e.to_string())?;

        if let Err(e) = self.experiment_handler.handle(ExperimentCommand::PauseExperiment {
            experiment_id: experiment_id.clone(),
        }).await {
            log("TRAINING", &format!("Failed to update experiment status to paused: {}", e), None);
        }

        log("TRAINING", &format!("暂停训练: {}", experiment_id), None);
        Ok(())
    }

    pub async fn resume_training(&self, experiment_id: &ExperimentId) -> Result<(), String> {
        let (engine_id, handle) = {
            let sessions = self.active_sessions.read().await;
            let session = sessions.get(&experiment_id.to_string())
                .ok_or_else(|| format!("No active training session for experiment: {}", experiment_id))?;
            (session.engine_id.clone(), session.handle.clone())
        };

        let engine = self.engine_registry.find_by_id_str(&engine_id).await
            .ok_or_else(|| format!("Engine '{}' not found", engine_id))?;

        engine.resume(handle).await.map_err(|e| e.to_string())?;

        if let Err(e) = self.experiment_handler.handle(ExperimentCommand::ResumeExperiment {
            experiment_id: experiment_id.clone(),
        }).await {
            log("TRAINING", &format!("Failed to update experiment status to running: {}", e), None);
        }

        log("TRAINING", &format!("恢复训练: {}", experiment_id), None);
        Ok(())
    }

    pub async fn resume_from_checkpoint(&self, experiment_id: &ExperimentId, checkpoint_epoch: usize) -> Result<(), String> {
        let exp_id_str = experiment_id.to_string();

        let experiment = self.experiment_repo.load(experiment_id).await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Experiment not found: {}", experiment_id))?;

        let config = experiment.config.clone();
        let engine_id = config.engine_id.clone();
        let artifact_dir = crate::core::config::get_artifact_dir(&exp_id_str);

        log("TRAINING", &format!("从checkpoint恢复训练: experiment={}, epoch={}", exp_id_str, checkpoint_epoch), None);

        let engine = self.engine_registry.find_by_id_str(&engine_id).await
            .ok_or_else(|| format!("Engine '{}' not found in registry", engine_id))?;

        let handle = engine.create_session(&config).await
            .map_err(|e| format!("Failed to create session: {}", e))?;

        let session_id = handle.session_id.clone();

        self.event_bus.emit(LabEvent::SessionCreated {
            session_id: session_id.clone(),
        });

        self.event_bus.emit(LabEvent::Custom(
            "SessionExperimentMapping".to_string(),
            serde_json::json!({
                "session_id": session_id.to_string(),
                "experiment_id": experiment_id.to_string(),
            }),
        ));

        self.experiment_handler.handle(ExperimentCommand::RestartExperiment {
            experiment_id: experiment_id.clone(),
        }).await.map_err(|e| e.to_string())?;

        let resource_monitor = Arc::new(ResourceMonitor::new(self.event_bus.clone()).with_interval(3));

        {
            let mut sessions = self.active_sessions.write().await;
            if sessions.contains_key(&exp_id_str) {
                return Err(format!("Training session already active for experiment: {}", experiment_id));
            }
            if sessions.len() >= self.max_concurrent_experiments {
                return Err(format!(
                    "Maximum concurrent experiments limit reached ({}/{}). Stop an existing experiment before starting a new one.",
                    sessions.len(), self.max_concurrent_experiments
                ));
            }
            sessions.insert(exp_id_str.clone(), ActiveSession {
                experiment_id: experiment_id.clone(),
                session_id: session_id.clone(),
                handle: handle.clone(),
                config: config.clone(),
                engine_id: engine_id.clone(),
                resource_monitor: resource_monitor.clone(),
            });
        }

        resource_monitor.set_session_id(Some(session_id.clone())).await;

        engine.start_from_checkpoint(handle, checkpoint_epoch, &artifact_dir).await
            .map_err(|e| {
                let error_msg = e.to_string();
                let exp_id = experiment_id.clone();
                let exp_id_str = exp_id_str.clone();
                let handler = self.experiment_handler.clone();
                let sessions = self.active_sessions.clone();
                let rm = resource_monitor.clone();
                let fail_reason = error_msg.clone();
                tokio::spawn(async move {
                    rm.stop().await;
                    let _ = handler.handle(ExperimentCommand::FailExperiment {
                        experiment_id: exp_id,
                        error: format!("Failed to start training from checkpoint: {}", fail_reason),
                    }).await;
                    let mut s = sessions.write().await;
                    s.remove(&exp_id_str);
                });
                format!("Failed to start training from checkpoint: {}", error_msg)
            })?;

        resource_monitor.start().await;

        spawn_event_loop(EventLoopParams {
            event_bus: self.event_bus.clone(),
            experiment_handler: self.experiment_handler.clone(),
            model_handler: self.model_handler.clone(),
            active_sessions: self.active_sessions.clone(),
            experiment_id: experiment_id.clone(),
            session_id,
            resource_monitor,
            log_prefix: "[Checkpoint恢复] ".to_string(),
            experiment_repo: self.experiment_repo.clone(),
        });

        Ok(())
    }

    pub async fn get_experiment_config(&self, experiment_id: &ExperimentId) -> Option<TrainingConfig> {
        let sessions = self.active_sessions.read().await;
        sessions.get(&experiment_id.to_string()).map(|s| s.config.clone())
    }

    pub async fn active_session_count(&self) -> usize {
        self.active_sessions.read().await.len()
    }
}

impl std::fmt::Debug for TrainingService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrainingService").finish()
    }
}
