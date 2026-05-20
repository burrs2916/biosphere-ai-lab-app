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
    let mut global_step: u64 = 0;

    tokio::spawn(async move {
        log("EVENT_LOOP", "========== 事件循环启动 ==========", None);
        log("EVENT_LOOP", &format!(
            "experiment_id={}, session_id={}, timeout={:?}, heartbeat={:?}",
            experiment_id, session_id, session_timeout, heartbeat_interval
        ), None);

        loop {
            let event = match tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv()).await {
                Ok(Ok(event)) => {
                    last_event_time = std::time::Instant::now();
                    event
                }
                Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(n))) => {
                    log("EVENT_LOOP", "WARN", Some(&format!("EventBus落后 {} 个事件", n)));
                    continue;
                }
                Ok(Err(tokio::sync::broadcast::error::RecvError::Closed)) => {
                    log("EVENT_LOOP", "EventBus已关闭", None);
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
                        log("EVENT_LOOP", "ERROR", Some(&format!(
                            "会话超时，{:?}内未收到事件: {}",
                            session_timeout, experiment_id
                        )));
                        if let Err(e) = experiment_handler.handle(ExperimentCommand::FailExperiment {
                            experiment_id: experiment_id.clone(),
                            error: format!("Session timed out - no events received for {:?}", session_timeout),
                        }).await {
                            log("EVENT_LOOP", "ERROR", Some(&format!("标记实验失败失败: {}", e)));
                        }
                        break;
                    }
                    continue;
                }
            };

            match &event {
                LabEvent::EpochCompleted { session_id: sid, epoch, total_epochs, train_loss, val_loss, metrics } => {
                    if sid == &session_id {
                        log("EVENT_LOOP", &format!(
                            "收到EpochCompleted事件: epoch={}/{}, train_loss={:.4}, val_loss={:.4}, metrics={}",
                            epoch, total_epochs, train_loss, val_loss.unwrap_or(0.0), metrics.as_object().map_or(0, |o| o.len())
                        ), None);
                        current_epoch = *epoch;
                        current_total_epochs = *total_epochs;

                        if let Err(e) = experiment_handler.handle(ExperimentCommand::TrackMetricWithEpoch {
                            experiment_id: experiment_id.clone(),
                            metric_name: "train_loss".to_string(),
                            value: *train_loss,
                            step: *epoch as u64,
                            epoch: *epoch,
                        }).await {
                            log("EVENT_LOOP", "ERROR", Some(&format!("记录train_loss失败: {}", e)));
                        }

                        if let Some(vl) = val_loss {
                            if let Err(e) = experiment_handler.handle(ExperimentCommand::TrackMetricWithEpoch {
                                experiment_id: experiment_id.clone(),
                                metric_name: "val_loss".to_string(),
                                value: *vl,
                                step: *epoch as u64,
                                epoch: *epoch,
                            }).await {
                                log("EVENT_LOOP", "ERROR", Some(&format!("记录val_loss失败: {}", e)));
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
                                log("EVENT_LOOP", "ERROR", Some(&format!("记录accuracy失败: {}", e)));
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
                                log("EVENT_LOOP", "ERROR", Some(&format!("记录val_accuracy失败: {}", e)));
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
                                        log("EVENT_LOOP", "ERROR", Some(&format!("记录自定义指标 '{}' 失败: {}", key, e)));
                                    }
                                }
                            }
                        }

                        log("TRAINING", &format!(
                            "{} Epoch {}/{} | loss={:.4} val_loss={:.4}",
                            log_prefix, epoch, total_epochs, train_loss,
                            val_loss.unwrap_or(0.0)
                        ), None);

                        if let Some(vl) = val_loss {
                            let gap = vl - train_loss;
                            if gap > 0.1 {
                                log("TRAINING", "WARN", Some(&format!(
                                    "过拟合预警: val_loss({:.4}) - train_loss({:.4}) = {:.4} > 0.1, 建议增加正则化或减少模型复杂度",
                                    vl, train_loss, gap
                                )));
                            }
                        }

                        if let Some(acc) = metrics.get("train_accuracy").and_then(|v| v.as_f64()) {
                            if let Some(val_acc) = metrics.get("val_accuracy").and_then(|v| v.as_f64()) {
                                let acc_gap = acc - val_acc;
                                if acc >= 99.0 && acc_gap > 5.0 {
                                    log("TRAINING", "WARN", Some(&format!(
                                        "过拟合预警: train_accuracy={:.1}% val_accuracy={:.1}% 差距={:.1}%, 训练集已完全拟合但验证集表现不佳",
                                        acc, val_acc, acc_gap
                                    )));
                                }
                            }
                        }
                    }
                }
                LabEvent::SessionCompleted { session_id: sid, final_metrics } => {
                    if sid == &session_id {
                        log("EVENT_LOOP", "========== 收到SessionCompleted事件 ==========", None);
                        log("EVENT_LOOP", &format!("experiment_id={}, final_metrics_keys={:?}, metrics_count={}",
                            experiment_id,
                            final_metrics.as_object().map_or(vec![], |o| o.keys().cloned().collect::<Vec<_>>()),
                            final_metrics.as_object().map_or(0, |o| o.len())
                        ), None);

                        let collected_final_metrics = if final_metrics.as_object().map_or(true, |o| o.is_empty()) {
                            log("EVENT_LOOP", "final_metrics为空，从实验记录中收集指标...", None);
                            match experiment_repo.load(&experiment_id).await {
                                Ok(Some(exp)) => {
                                    let mut m = serde_json::Map::new();
                                    for (name, series) in exp.metrics.all_series() {
                                        if let Some(last) = series.values.last() {
                                            m.insert(name.clone(), serde_json::Value::from(last.value));
                                            log("EVENT_LOOP", &format!("  指标 '{}' 最后值={:.4} (共{}个数据点)", name, last.value, series.values.len()), None);
                                        }
                                    }
                                    log("EVENT_LOOP", &format!("收集到 {} 个指标", m.len()), None);
                                    serde_json::Value::Object(m)
                                }
                                _ => {
                                    log("EVENT_LOOP", "WARN", Some("无法从实验记录中加载指标"));
                                    final_metrics.clone()
                                }
                            }
                        } else {
                            log("EVENT_LOOP", &format!("使用引擎提供的final_metrics: {} 个指标", final_metrics.as_object().map_or(0, |o| o.len())), None);
                            final_metrics.clone()
                        };

                        log("EVENT_LOOP", "标记实验完成...", None);
                        if let Err(e) = experiment_handler.handle(ExperimentCommand::CompleteExperiment {
                            experiment_id: experiment_id.clone(),
                            final_metrics: collected_final_metrics,
                        }).await {
                            log("EVENT_LOOP", "ERROR", Some(&format!("标记实验完成失败: {}", e)));
                        }

                        let model_name = format!("{}_model", experiment_id);
                        log("EVENT_LOOP", &format!("自动注册模型: '{}'", model_name), None);
                        match model_handler.handle(ModelCommand::RegisterModelFromExperiment {
                            experiment_id: experiment_id.clone(),
                            name: model_name,
                            version: "auto".to_string(),
                        }).await {
                            Ok(_) => {
                                log("EVENT_LOOP", "模型注册成功", None);
                            }
                            Err(e) => {
                                log("EVENT_LOOP", "ERROR", Some(&format!("模型注册失败: {}", e)));
                            }
                        }

                        log("EVENT_LOOP", "扫描并注册产物...", None);
                        scan_and_register_artifacts(&experiment_id, &experiment_handler).await;

                        log("EVENT_LOOP", "========== SessionCompleted处理完成 ==========", None);
                        break;
                    }
                }
                LabEvent::SessionFailed { session_id: sid, error } => {
                    if sid == &session_id {
                        log("EVENT_LOOP", "========== 收到SessionFailed事件 ==========", None);
                        log("EVENT_LOOP", &format!("experiment_id={}, error='{}'", experiment_id, error), None);

                        if let Err(e) = experiment_handler.handle(ExperimentCommand::FailExperiment {
                            experiment_id: experiment_id.clone(),
                            error: error.clone(),
                        }).await {
                            log("EVENT_LOOP", "ERROR", Some(&format!("标记实验失败失败: {}", e)));
                        }

                        log("EVENT_LOOP", "========== SessionFailed处理完成 ==========", None);
                        break;
                    }
                }
                LabEvent::SessionCancelled { session_id: sid } => {
                    if sid == &session_id {
                        log("EVENT_LOOP", "========== 收到SessionCancelled事件 ==========", None);
                        log("EVENT_LOOP", &format!("experiment_id={}", experiment_id), None);

                        if let Err(e) = experiment_handler.handle(ExperimentCommand::CancelExperiment {
                            experiment_id: experiment_id.clone(),
                        }).await {
                            log("EVENT_LOOP", "ERROR", Some(&format!("标记实验取消失败: {}", e)));
                        }

                        log("EVENT_LOOP", "========== SessionCancelled处理完成 ==========", None);
                        break;
                    }
                }
                LabEvent::LogOutput { session_id: sid, level, message } => {
                    if sid == &session_id {
                        log("EVENT_LOOP", &format!("LogOutput: level='{}', message='{}'", level, message), None);
                        if let Err(e) = experiment_repo.save_log(&experiment_id, level, message).await {
                            log("EVENT_LOOP", "ERROR", Some(&format!("保存日志失败: {}", e)));
                        }
                    }
                }
                LabEvent::BatchCompleted { session_id: sid, batch, total_batches, loss } => {
                    if sid == &session_id {
                        global_step += 1;
                        if *batch % 10 == 0 || batch == total_batches {
                            log("EVENT_LOOP", &format!(
                                "BatchCompleted: batch={}/{}, global_step={}, loss={:.4}",
                                batch, total_batches, global_step, loss
                            ), None);
                        }
                        if let Err(e) = experiment_handler.handle(ExperimentCommand::TrackMetric {
                            experiment_id: experiment_id.clone(),
                            metric_name: "batch_loss".to_string(),
                            value: *loss,
                            step: global_step,
                        }).await {
                            log("EVENT_LOOP", "ERROR", Some(&format!("记录batch_loss失败: {}", e)));
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
                        log("EVENT_LOOP", &format!("CheckpointSaved: epoch={}, path='{}'", epoch, path), None);
                        let artifact = ArtifactRef::new(
                            "checkpoint".to_string(),
                            path.clone(),
                            std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
                        );
                        if let Err(e) = experiment_handler.handle(ExperimentCommand::AddArtifact {
                            experiment_id: experiment_id.clone(),
                            artifact,
                        }).await {
                            log("EVENT_LOOP", "ERROR", Some(&format!("添加checkpoint产物失败: {}", e)));
                        }
                    }
                }
                LabEvent::DataLoaded { session_id: sid, rows, columns } => {
                    if sid == &session_id {
                        log("EVENT_LOOP", &format!("DataLoaded: rows={}, columns={}", rows, columns), None);
                    }
                }
                LabEvent::SessionCreated { session_id: sid } => {
                    if sid == &session_id {
                        log("EVENT_LOOP", &format!("SessionCreated: session_id={}", sid), None);
                    }
                }
                LabEvent::ProgressUpdate { session_id: sid, progress, message } => {
                    if sid == &session_id {
                        log("EVENT_LOOP", &format!("ProgressUpdate: {:.1}% - {}", progress * 100.0, message), None);
                    }
                }
                _ => {}
            }
        }

        log("EVENT_LOOP", "========== 事件循环退出 ==========", None);
        log("EVENT_LOOP", &format!(
            "experiment_id={}, elapsed={:.1}s, final_epoch={}/{}",
            experiment_id, training_start.elapsed().as_secs_f64(), current_epoch, current_total_epochs
        ), None);

        log("EVENT_LOOP", "清理会话资源...", None);
        cleanup_session(&active_sessions, &experiment_id, &resource_monitor).await;
        log("EVENT_LOOP", "会话资源清理完成", None);
    });
}

async fn scan_and_register_artifacts(
    experiment_id: &ExperimentId,
    experiment_handler: &Arc<dyn ExperimentCommandHandler>,
) {
    log("ARTIFACT_SCAN", "========== 扫描产物文件 ==========", None);
    log("ARTIFACT_SCAN", &format!("experiment_id={}", experiment_id), None);

    let artifact_dir = crate::core::config::get_artifact_dir(&experiment_id.to_string());
    log("ARTIFACT_SCAN", &format!("产物目录: '{}'", artifact_dir), None);

    let mut all_files: Vec<(std::path::PathBuf, String, u64)> = Vec::new();
    let mut dirs_count = 0;
    let mut model_found = false;

    collect_files_recursive(std::path::Path::new(&artifact_dir), &mut all_files, &mut dirs_count, &mut model_found);

    let count = all_files.len();
    log("ARTIFACT_SCAN", &format!("扫描到 {} 个文件, {} 个子目录, model_found={}", count, dirs_count, model_found), None);

    for (path, artifact_type, size) in &all_files {
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        log("ARTIFACT_SCAN", &format!("注册产物: type='{}', name='{}', size={}, path='{}'", artifact_type, name, size, path.display()), None);
        let artifact = ArtifactRef::new(
            artifact_type.clone(),
            path.to_string_lossy().to_string(),
            *size,
        );
        if let Err(e) = experiment_handler.handle(ExperimentCommand::AddArtifact {
            experiment_id: experiment_id.clone(),
            artifact,
        }).await {
            log("ARTIFACT_SCAN", "ERROR", Some(&format!("添加产物失败: {}", e)));
        }
    }

    if !model_found {
        log("ARTIFACT_SCAN", "WARN", Some("未找到模型文件(以checkpoint-或model-开头)，尝试在子目录中查找..."));
        find_model_in_subdirs(&artifact_dir, experiment_id, experiment_handler).await;
    }

    log("ARTIFACT_SCAN", &format!("产物扫描完成: 共 {} 个文件, {} 个子目录", count, dirs_count), None);
}

fn collect_files_recursive(
    dir: &std::path::Path,
    files: &mut Vec<(std::path::PathBuf, String, u64)>,
    dirs_count: &mut usize,
    model_found: &mut bool,
) {
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
                *dirs_count += 1;
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                log("ARTIFACT_SCAN", &format!("发现子目录: '{}' (path='{}')", dir_name, path.display()), None);
                collect_files_recursive(&path, files, dirs_count, model_found);
                continue;
            }

            if !path.is_file() {
                continue;
            }

            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name == ".DS_Store" {
                continue;
            }

            let size = path.metadata().map(|m| m.len()).unwrap_or(0);
            let artifact_type = if name.starts_with("checkpoint-") || name.starts_with("model-") || name == "model.mpk" || name == "model.mpk.gz" || name == "model.bin" {
                *model_found = true;
                if name.starts_with("checkpoint-") { "checkpoint".to_string() } else { "model".to_string() }
            } else if name.ends_with(".json") {
                "metadata".to_string()
            } else if name.ends_with(".log") {
                "log".to_string()
            } else if name.ends_with(".mpk") || name.ends_with(".mpk.gz") || name.ends_with(".bin") {
                *model_found = true;
                "model".to_string()
            } else {
                "other".to_string()
            };
            files.push((path, artifact_type, size));
        }
    }
}

async fn find_model_in_subdirs(
    artifact_dir: &str,
    experiment_id: &ExperimentId,
    experiment_handler: &Arc<dyn ExperimentCommandHandler>,
) {
    let artifact_path = std::path::Path::new(artifact_dir);

    let model_final_path = artifact_path.join("model.mpk");
    if model_final_path.exists() {
        log("ARTIFACT_SCAN", &format!("找到最终模型文件: '{}'", model_final_path.display()), None);
        let size = model_final_path.metadata().map(|m| m.len()).unwrap_or(0);
        let artifact = ArtifactRef::new(
            "model".to_string(),
            model_final_path.to_string_lossy().to_string(),
            size,
        );
        if let Err(e) = experiment_handler.handle(ExperimentCommand::AddArtifact {
            experiment_id: experiment_id.clone(),
            artifact,
        }).await {
            log("ARTIFACT_SCAN", "ERROR", Some(&format!("添加模型产物失败: {}", e)));
        }
        return;
    }

    let checkpoint_dir = artifact_path.join("checkpoint");
    if checkpoint_dir.exists() {
        log("ARTIFACT_SCAN", &format!("搜索checkpoint目录: '{}'", checkpoint_dir.display()), None);
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
            log("ARTIFACT_SCAN", &format!("找到最新checkpoint: epoch={}, path='{}'", latest_epoch, cp_path.display()), None);
            let size = cp_path.metadata().map(|m| m.len()).unwrap_or(0);
            let artifact = ArtifactRef::new(
                "model".to_string(),
                cp_path.to_string_lossy().to_string(),
                size,
            );
            if let Err(e) = experiment_handler.handle(ExperimentCommand::AddArtifact {
                experiment_id: experiment_id.clone(),
                artifact,
            }).await {
                log("ARTIFACT_SCAN", "ERROR", Some(&format!("添加模型产物失败: {}", e)));
            }
            return;
        }
    }

    let mut best_model_path: Option<std::path::PathBuf> = None;
    let mut best_epoch: usize = 0;

    if let Ok(train_dir) = std::fs::read_dir(std::path::Path::new(artifact_dir).join("train")) {
        for entry in train_dir.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let dir_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if let Some(epoch_num) = dir_name.strip_prefix("epoch-") {
                if let Ok(num) = epoch_num.parse::<usize>() {
                    if num > best_epoch {
                        best_epoch = num;
                        best_model_path = Some(path);
                    }
                }
            }
        }
    }

    if let Some(model_dir) = best_model_path {
        log("ARTIFACT_SCAN", &format!("找到最新epoch目录: epoch={}, path='{}'", best_epoch, model_dir.display()), None);
        if let Ok(entries) = std::fs::read_dir(&model_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() { continue; }
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                let artifact_type = if name.starts_with("checkpoint-") || name.starts_with("model-") {
                    "model"
                } else if name.ends_with(".log") {
                    "metric_log"
                } else if name.ends_with(".json") {
                    "metadata"
                } else {
                    "other"
                };
                log("ARTIFACT_SCAN", &format!("注册epoch产物: type='{}', name='{}', size={}, epoch={}", artifact_type, name, size, best_epoch), None);
                let artifact = ArtifactRef::new(
                    artifact_type.to_string(),
                    path.to_string_lossy().to_string(),
                    size,
                );
                if let Err(e) = experiment_handler.handle(ExperimentCommand::AddArtifact {
                    experiment_id: experiment_id.clone(),
                    artifact,
                }).await {
                    log("ARTIFACT_SCAN", "ERROR", Some(&format!("添加模型产物失败: {}", e)));
                }
            }
        }
    } else {
        log("ARTIFACT_SCAN", "WARN", Some("未找到任何模型文件或epoch目录"));
    }
}

async fn cleanup_session(
    active_sessions: &Arc<RwLock<HashMap<String, ActiveSession>>>,
    experiment_id: &ExperimentId,
    resource_monitor: &Arc<ResourceMonitor>,
) {
    log("SESSION_CLEANUP", "---------- 清理会话 ----------", None);
    log("SESSION_CLEANUP", &format!("experiment_id={}", experiment_id), None);

    log("SESSION_CLEANUP", "停止资源监控...", None);
    resource_monitor.stop().await;
    resource_monitor.set_session_id(None).await;

    log("SESSION_CLEANUP", "从活跃会话列表中移除...", None);
    let mut sessions = active_sessions.write().await;
    sessions.remove(&experiment_id.to_string());
    log("SESSION_CLEANUP", &format!("剩余活跃会话数: {}", sessions.len()), None);
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
        log("TRAINING_SERVICE", "========== TrainingService.start_training ==========", None);
        log("TRAINING_SERVICE", &format!("experiment_id='{}'", exp_id_str), None);
        log("TRAINING_SERVICE", &format!("初始配置: engine='{}', model='{}', data_path='{}'", 
            config.engine_id, config.model_id, config.data_path), None);
        log("TRAINING_SERVICE", &format!("初始配置: epochs={}, batch_size={}, lr={}, validation_split={}, test_split={}", 
            config.epochs, config.batch_size, config.learning_rate, config.validation_split, config.test_split), None);

        log("TRAINING_SERVICE", "步骤1: 解析数据集配置...", None);
        if let Some(ref dataset_id_str) = config.dataset_id {
            log("TRAINING_SERVICE", &format!("配置中指定了数据集: '{}'", dataset_id_str), None);
            if let Some(ref dataset_repo) = self.dataset_repo {
                let ds_id = DatasetId::from_str(dataset_id_str);
                let dataset = dataset_repo.load(&ds_id).await
                    .map_err(|e| {
                        log("TRAINING_SERVICE", "ERROR", Some(&format!("加载数据集失败: {}", e)));
                        format!("Failed to load dataset: {}", e)
                    })?
                    .ok_or_else(|| {
                        log("TRAINING_SERVICE", "ERROR", Some(&format!("数据集不存在: '{}'", dataset_id_str)));
                        format!("Dataset not found: {}", dataset_id_str)
                    })?;
                log("TRAINING_SERVICE", &format!("数据集加载成功: name='{}', status={:?}, rows={}, cols={}", 
                    dataset.name, dataset.status, dataset.rows, dataset.columns), None);

                if dataset.status != crate::domain::dataset::aggregate::DatasetStatus::Active {
                    log("TRAINING_SERVICE", "ERROR", Some(&format!("数据集状态不是Active: {:?}", dataset.status)));
                    return Err(format!(
                        "Dataset '{}' is in {} status, must be Active to train",
                        dataset_id_str, dataset.status
                    ));
                }

                if config.data_path.is_empty() {
                    log("TRAINING_SERVICE", &format!("使用数据集路径: '{}'", dataset.path), None);
                    config.data_path = dataset.path.clone();
                }
                if config.data_format != dataset.format {
                    log("TRAINING_SERVICE", &format!("使用数据集格式: {:?}", dataset.format), None);
                    config.data_format = dataset.format;
                }

                if config.feature_columns.is_empty() {
                    let numeric_cols: Vec<String> = dataset.numeric_columns()
                        .iter()
                        .map(|p| p.name.clone())
                        .collect();
                    log("TRAINING_SERVICE", &format!("自动选择数值列作为特征: {:?}", numeric_cols), None);
                    config.feature_columns = numeric_cols;
                }

                if let Some(ref version) = config.dataset_version {
                    if version != dataset.version.as_str() {
                        log("TRAINING_SERVICE", "WARN", Some(&format!(
                            "请求的数据集版本 {} 与当前版本 {} 不匹配",
                            version, dataset.version
                        )));
                    }
                }
                config.dataset_version = Some(dataset.version.to_string());

                if let Some(ref split_name) = config.split_name {
                    log("TRAINING_SERVICE", &format!("加载数据集分割: '{}'", split_name), None);
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
                            log("TRAINING_SERVICE", &format!(
                                "分割加载成功: train={}, val={}, test={}",
                                split.train_count(), split.val_count(), split.test_count()
                            ), None);
                        }
                        Ok(None) => {
                            log("TRAINING_SERVICE", "ERROR", Some(&format!("分割 '{}' 不存在于数据集 '{}'", split_name, dataset_id_str)));
                            return Err(format!("Split '{}' not found in dataset '{}'", split_name, dataset_id_str));
                        }
                        Err(e) => {
                            log("TRAINING_SERVICE", "ERROR", Some(&format!("加载分割 '{}' 失败: {}", split_name, e)));
                            return Err(format!("Failed to load split '{}': {}", split_name, e));
                        }
                    }
                }

                log("TRAINING_SERVICE", &format!(
                    "数据集解析完成: id={}, version={}, path={}, rows={}, cols={}",
                    dataset_id_str, dataset.version, dataset.path, dataset.rows, dataset.columns
                ), None);
            } else {
                log("TRAINING_SERVICE", "WARN", Some(&format!(
                    "指定了数据集 '{}' 但 dataset_repo 未配置",
                    dataset_id_str
                )));
            }
        } else {
            log("TRAINING_SERVICE", "未指定数据集ID，使用配置中的数据路径", None);
        }

        log("TRAINING_SERVICE", "步骤2: 验证训练参数...", None);
        if config.epochs == 0 {
            log("TRAINING_SERVICE", "ERROR", Some("epochs 必须大于 0"));
            return Err("Training epochs must be greater than 0".to_string());
        }
        if config.batch_size == 0 {
            log("TRAINING_SERVICE", "ERROR", Some("batch_size 必须大于 0"));
            return Err("Batch size must be greater than 0".to_string());
        }
        if config.learning_rate <= 0.0 {
            log("TRAINING_SERVICE", "ERROR", Some(&format!("learning_rate 必须大于 0, 当前: {}", config.learning_rate)));
            return Err("Learning rate must be greater than 0".to_string());
        }
        if config.batch_size > 4096 {
            log("TRAINING_SERVICE", "ERROR", Some(&format!("batch_size 不能超过 4096, 当前: {}", config.batch_size)));
            return Err("Batch size cannot exceed 4096".to_string());
        }
        if config.epochs > 10000 {
            log("TRAINING_SERVICE", "ERROR", Some(&format!("epochs 不能超过 10000, 当前: {}", config.epochs)));
            return Err("Epochs cannot exceed 10000".to_string());
        }
        if config.validation_split + config.test_split >= 1.0 {
            log("TRAINING_SERVICE", "ERROR", Some(&format!(
                "validation_split ({}) + test_split ({}) >= 1.0",
                config.validation_split, config.test_split
            )));
            return Err(format!(
                "Validation split ({}) + test split ({}) must be less than 1.0",
                config.validation_split, config.test_split
            ));
        }
        if config.validation_split < 0.0 || config.test_split < 0.0 {
            log("TRAINING_SERVICE", "ERROR", Some("validation_split 和 test_split 必须非负"));
            return Err("Validation split and test split must be non-negative".to_string());
        }
        if !config.data_path.is_empty() {
            let data_path = std::path::Path::new(&config.data_path);
            if !data_path.exists() {
                log("TRAINING_SERVICE", "ERROR", Some(&format!("数据路径不存在: '{}'", config.data_path)));
                return Err(format!("Data path does not exist: {}", config.data_path));
            }
            log("TRAINING_SERVICE", &format!("数据路径验证通过: '{}'", config.data_path), None);
        }
        log("TRAINING_SERVICE", "训练参数验证通过", None);

        log("TRAINING_SERVICE", "步骤3: 查找训练引擎...", None);
        log("TRAINING_SERVICE", &format!("查找引擎: '{}'", config.engine_id), None);
        let engine = match self.engine_registry.find_by_id_str(&config.engine_id).await {
            Some(e) => {
                log("TRAINING_SERVICE", &format!("引擎找到: '{}'", config.engine_id), None);
                e
            },
            None => {
                let available: Vec<String> = self.engine_registry.list().await.iter().map(|e| e.id.to_string()).collect();
                log("TRAINING_SERVICE", "ERROR", Some(&format!("引擎 '{}' 不存在。可用引擎: {:?}", config.engine_id, available)));
                return Err(format!("Engine '{}' not found in registry. Available: {:?}", config.engine_id, available));
            }
        };

        log("TRAINING_SERVICE", "步骤4: 创建训练会话...", None);
        log("TRAINING_SERVICE", &format!("会话配置: data_path='{}', epochs={}, batch_size={}, lr={}", 
            config.data_path, config.epochs, config.batch_size, config.learning_rate), None);
        let mut handle = engine.create_session(&config).await
            .map_err(|e| {
                log("TRAINING_SERVICE", "ERROR", Some(&format!("创建会话失败: {}", e)));
                format!("Failed to create session: {}", e)
            })?;

        handle.experiment_id = Some(experiment_id.to_string());

        let session_id = handle.session_id.clone();
        log("TRAINING_SERVICE", &format!("会话创建成功: session_id='{}'", session_id), None);

        log("TRAINING_SERVICE", "步骤5: 发送会话创建事件...", None);
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
        log("TRAINING_SERVICE", "会话创建事件已发送", None);

        log("TRAINING_SERVICE", "步骤6: 更新实验状态为Running...", None);
        self.experiment_handler.handle(ExperimentCommand::StartExperiment {
            experiment_id: experiment_id.clone(),
        }).await.map_err(|e| {
            log("TRAINING_SERVICE", "ERROR", Some(&format!("更新实验状态失败: {}", e)));
            e.to_string()
        })?;
        log("TRAINING_SERVICE", "实验状态已更新为Running", None);

        if !config.data_source_id.is_empty() {
            log("TRAINING_SERVICE", &format!("关联数据源: '{}'", config.data_source_id), None);
            let _ = self.experiment_handler.handle(ExperimentCommand::LinkDataset {
                experiment_id: experiment_id.clone(),
                dataset_id: config.data_source_id.clone(),
                dataset_version: None,
            }).await;
        }

        log("TRAINING_SERVICE", "步骤7: 检查并发会话限制...", None);
        let resource_monitor = Arc::new(ResourceMonitor::new(self.event_bus.clone()).with_interval(3));
        let engine_id = config.engine_id.clone();

        {
            let mut sessions = self.active_sessions.write().await;
            if sessions.contains_key(&exp_id_str) {
                log("TRAINING_SERVICE", "ERROR", Some(&format!("实验 '{}' 已有活跃的训练会话", exp_id_str)));
                let _ = self.experiment_handler.handle(ExperimentCommand::FailExperiment {
                    experiment_id: experiment_id.clone(),
                    error: "Duplicate training session detected".to_string(),
                }).await;
                return Err(format!("Training session already active for experiment: {}", experiment_id));
            }
            if sessions.len() >= self.max_concurrent_experiments {
                log("TRAINING_SERVICE", "ERROR", Some(&format!(
                    "达到最大并发实验限制: {}/{}",
                    sessions.len(), self.max_concurrent_experiments
                )));
                let _ = self.experiment_handler.handle(ExperimentCommand::FailExperiment {
                    experiment_id: experiment_id.clone(),
                    error: "Maximum concurrent experiments limit reached".to_string(),
                }).await;
                return Err(format!(
                    "Maximum concurrent experiments limit reached ({}/{}). Stop an existing experiment before starting a new one.",
                    sessions.len(), self.max_concurrent_experiments
                ));
            }
            log("TRAINING_SERVICE", &format!("当前活跃会话数: {}/{}", sessions.len(), self.max_concurrent_experiments), None);
            sessions.insert(exp_id_str.clone(), ActiveSession {
                experiment_id: experiment_id.clone(),
                session_id: session_id.clone(),
                handle: handle.clone(),
                config: config.clone(),
                engine_id: engine_id.clone(),
                resource_monitor: resource_monitor.clone(),
            });
            log("TRAINING_SERVICE", "会话已添加到活跃会话列表", None);
        }

        log("TRAINING_SERVICE", "步骤8: 启动训练引擎...", None);
        engine.start(handle.clone()).await
            .map_err(|e| {
                let error_msg = e.to_string();
                log("TRAINING_SERVICE", "ERROR", Some(&format!("启动引擎失败: {}", error_msg)));
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
        log("TRAINING_SERVICE", "训练引擎启动成功", None);

        log("TRAINING_SERVICE", "步骤9: 启动资源监控和事件循环...", None);
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

        log("TRAINING_SERVICE", "========== 训练启动完成 ==========", None);
        log("TRAINING_SERVICE", &format!("experiment_id='{}', session_id='{}'", exp_id_str, handle.session_id), None);
        Ok(())
    }

    pub async fn stop_training(&self, experiment_id: &ExperimentId) -> Result<(), String> {
        log("TRAINING_SERVICE", "========== TrainingService.stop_training ==========", None);
        log("TRAINING_SERVICE", &format!("experiment_id='{}'", experiment_id), None);

        let (engine_id, handle) = {
            log("TRAINING_SERVICE", "查找活跃会话...", None);
            let sessions = self.active_sessions.read().await;
            match sessions.get(&experiment_id.to_string()) {
                Some(session) => {
                    log("TRAINING_SERVICE", &format!("找到会话: engine='{}', session_id='{}'", session.engine_id, session.session_id), None);
                    (session.engine_id.clone(), session.handle.clone())
                }
                None => {
                    log("TRAINING_SERVICE", "WARN", Some(&format!("未找到活跃会话: {}，尝试标记为中断", experiment_id)));
                    if let Ok(Some(mut experiment)) = self.experiment_repo.load(experiment_id).await {
                        if experiment.status == crate::domain::experiment::aggregate::ExperimentStatus::Running {
                            experiment.status = crate::domain::experiment::aggregate::ExperimentStatus::Failed;
                            experiment.error_message = Some("Training session lost (app restart or crash). Marked as interrupted.".to_string());
                            experiment.completed_at = Some(chrono::Utc::now());
                            let _ = self.experiment_repo.save(&experiment).await;
                            log("TRAINING_SERVICE", &format!("已将孤儿实验标记为Failed: {}", experiment_id), None);
                        }
                    }
                    return Ok(());
                }
            }
        };

        log("TRAINING_SERVICE", &format!("查找引擎: '{}'", engine_id), None);
        let engine = self.engine_registry.find_by_id_str(&engine_id).await
            .ok_or_else(|| {
                log("TRAINING_SERVICE", "ERROR", Some(&format!("引擎不存在: '{}'", engine_id)));
                format!("Engine '{}' not found", engine_id)
            })?;

        log("TRAINING_SERVICE", "停止引擎...", None);
        engine.stop(handle).await.map_err(|e| {
            log("TRAINING_SERVICE", "ERROR", Some(&format!("停止引擎失败: {}", e)));
            e.to_string()
        })?;

        log("TRAINING_SERVICE", "========== 停止训练完成 ==========", None);
        Ok(())
    }

    pub async fn pause_training(&self, experiment_id: &ExperimentId) -> Result<(), String> {
        log("TRAINING_SERVICE", "========== TrainingService.pause_training ==========", None);
        log("TRAINING_SERVICE", &format!("experiment_id='{}'", experiment_id), None);

        let (engine_id, handle) = {
            log("TRAINING_SERVICE", "查找活跃会话...", None);
            let sessions = self.active_sessions.read().await;
            match sessions.get(&experiment_id.to_string()) {
                Some(session) => {
                    log("TRAINING_SERVICE", &format!("找到会话: engine='{}', session_id='{}'", session.engine_id, session.session_id), None);
                    (session.engine_id.clone(), session.handle.clone())
                }
                None => {
                    log("TRAINING_SERVICE", "WARN", Some(&format!("未找到活跃会话: {}，尝试标记为中断", experiment_id)));
                    if let Ok(Some(mut experiment)) = self.experiment_repo.load(experiment_id).await {
                        if experiment.status == crate::domain::experiment::aggregate::ExperimentStatus::Running {
                            experiment.status = crate::domain::experiment::aggregate::ExperimentStatus::Failed;
                            experiment.error_message = Some("Training session lost (app restart or crash). Marked as interrupted.".to_string());
                            experiment.completed_at = Some(chrono::Utc::now());
                            let _ = self.experiment_repo.save(&experiment).await;
                            log("TRAINING_SERVICE", &format!("已将孤儿实验标记为Failed: {}", experiment_id), None);
                        }
                    }
                    return Ok(());
                }
            }
        };

        log("TRAINING_SERVICE", &format!("查找引擎: '{}'", engine_id), None);
        let engine = self.engine_registry.find_by_id_str(&engine_id).await
            .ok_or_else(|| {
                log("TRAINING_SERVICE", "ERROR", Some(&format!("引擎不存在: '{}'", engine_id)));
                format!("Engine '{}' not found", engine_id)
            })?;

        log("TRAINING_SERVICE", "暂停引擎...", None);
        engine.pause(handle).await.map_err(|e| {
            log("TRAINING_SERVICE", "ERROR", Some(&format!("暂停引擎失败: {}", e)));
            e.to_string()
        })?;

        log("TRAINING_SERVICE", "更新实验状态为Paused...", None);
        if let Err(e) = self.experiment_handler.handle(ExperimentCommand::PauseExperiment {
            experiment_id: experiment_id.clone(),
        }).await {
            log("TRAINING_SERVICE", "ERROR", Some(&format!("更新实验状态失败: {}", e)));
        }

        log("TRAINING_SERVICE", "========== 暂停训练完成 ==========", None);
        Ok(())
    }

    pub async fn resume_training(&self, experiment_id: &ExperimentId) -> Result<(), String> {
        log("TRAINING_SERVICE", "========== TrainingService.resume_training ==========", None);
        log("TRAINING_SERVICE", &format!("experiment_id='{}'", experiment_id), None);

        let (engine_id, handle) = {
            log("TRAINING_SERVICE", "查找活跃会话...", None);
            let sessions = self.active_sessions.read().await;
            let session = sessions.get(&experiment_id.to_string())
                .ok_or_else(|| {
                    log("TRAINING_SERVICE", "ERROR", Some(&format!("未找到活跃会话: {}", experiment_id)));
                    format!("No active training session for experiment: {}", experiment_id)
                })?;
            log("TRAINING_SERVICE", &format!("找到会话: engine='{}', session_id='{}'", session.engine_id, session.session_id), None);
            (session.engine_id.clone(), session.handle.clone())
        };

        log("TRAINING_SERVICE", &format!("查找引擎: '{}'", engine_id), None);
        let engine = self.engine_registry.find_by_id_str(&engine_id).await
            .ok_or_else(|| {
                log("TRAINING_SERVICE", "ERROR", Some(&format!("引擎不存在: '{}'", engine_id)));
                format!("Engine '{}' not found", engine_id)
            })?;

        log("TRAINING_SERVICE", "恢复引擎...", None);
        engine.resume(handle).await.map_err(|e| {
            log("TRAINING_SERVICE", "ERROR", Some(&format!("恢复引擎失败: {}", e)));
            e.to_string()
        })?;

        log("TRAINING_SERVICE", "更新实验状态为Running...", None);
        if let Err(e) = self.experiment_handler.handle(ExperimentCommand::ResumeExperiment {
            experiment_id: experiment_id.clone(),
        }).await {
            log("TRAINING_SERVICE", "ERROR", Some(&format!("更新实验状态失败: {}", e)));
        }

        log("TRAINING_SERVICE", "========== 恢复训练完成 ==========", None);
        Ok(())
    }

    pub async fn resume_from_checkpoint(&self, experiment_id: &ExperimentId, checkpoint_epoch: usize) -> Result<(), String> {
        let exp_id_str = experiment_id.to_string();
        log("TRAINING_SERVICE", "========== TrainingService.resume_from_checkpoint ==========", None);
        log("TRAINING_SERVICE", &format!("experiment_id='{}', checkpoint_epoch={}", exp_id_str, checkpoint_epoch), None);

        log("TRAINING_SERVICE", "步骤1: 加载实验...", None);
        let experiment = self.experiment_repo.load(experiment_id).await
            .map_err(|e| {
                log("TRAINING_SERVICE", "ERROR", Some(&format!("加载实验失败: {}", e)));
                e.to_string()
            })?
            .ok_or_else(|| {
                log("TRAINING_SERVICE", "ERROR", Some(&format!("实验不存在: {}", experiment_id)));
                format!("Experiment not found: {}", experiment_id)
            })?;
        log("TRAINING_SERVICE", &format!("实验加载成功: name='{}', status={:?}", experiment.name, experiment.status), None);

        let config = experiment.config.clone();
        let engine_id = config.engine_id.clone();
        let artifact_dir = crate::core::config::get_artifact_dir(&exp_id_str);
        log("TRAINING_SERVICE", &format!("产物目录: '{}'", artifact_dir), None);

        log("TRAINING_SERVICE", "步骤2: 查找训练引擎...", None);
        log("TRAINING_SERVICE", &format!("查找引擎: '{}'", engine_id), None);
        let engine = self.engine_registry.find_by_id_str(&engine_id).await
            .ok_or_else(|| {
                log("TRAINING_SERVICE", "ERROR", Some(&format!("引擎不存在: '{}'", engine_id)));
                format!("Engine '{}' not found in registry", engine_id)
            })?;

        log("TRAINING_SERVICE", "步骤3: 创建训练会话...", None);
        let mut handle = engine.create_session(&config).await
            .map_err(|e| {
                log("TRAINING_SERVICE", "ERROR", Some(&format!("创建会话失败: {}", e)));
                format!("Failed to create session: {}", e)
            })?;

        handle.experiment_id = Some(experiment_id.to_string());

        let session_id = handle.session_id.clone();
        log("TRAINING_SERVICE", &format!("会话创建成功: session_id='{}'", session_id), None);

        log("TRAINING_SERVICE", "步骤4: 发送会话创建事件...", None);
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
        log("TRAINING_SERVICE", "会话创建事件已发送", None);

        log("TRAINING_SERVICE", "步骤5: 更新实验状态为Running...", None);
        self.experiment_handler.handle(ExperimentCommand::RestartExperiment {
            experiment_id: experiment_id.clone(),
        }).await.map_err(|e| {
            log("TRAINING_SERVICE", "ERROR", Some(&format!("更新实验状态失败: {}", e)));
            e.to_string()
        })?;
        log("TRAINING_SERVICE", "实验状态已更新为Running", None);

        log("TRAINING_SERVICE", "步骤6: 检查并发会话限制...", None);
        let resource_monitor = Arc::new(ResourceMonitor::new(self.event_bus.clone()).with_interval(3));

        {
            let mut sessions = self.active_sessions.write().await;
            if sessions.contains_key(&exp_id_str) {
                log("TRAINING_SERVICE", "ERROR", Some(&format!("实验 '{}' 已有活跃的训练会话", exp_id_str)));
                return Err(format!("Training session already active for experiment: {}", experiment_id));
            }
            if sessions.len() >= self.max_concurrent_experiments {
                log("TRAINING_SERVICE", "ERROR", Some(&format!(
                    "达到最大并发实验限制: {}/{}",
                    sessions.len(), self.max_concurrent_experiments
                )));
                return Err(format!(
                    "Maximum concurrent experiments limit reached ({}/{}). Stop an existing experiment before starting a new one.",
                    sessions.len(), self.max_concurrent_experiments
                ));
            }
            log("TRAINING_SERVICE", &format!("当前活跃会话数: {}/{}", sessions.len(), self.max_concurrent_experiments), None);
            sessions.insert(exp_id_str.clone(), ActiveSession {
                experiment_id: experiment_id.clone(),
                session_id: session_id.clone(),
                handle: handle.clone(),
                config: config.clone(),
                engine_id: engine_id.clone(),
                resource_monitor: resource_monitor.clone(),
            });
            log("TRAINING_SERVICE", "会话已添加到活跃会话列表", None);
        }

        log("TRAINING_SERVICE", "步骤7: 从检查点恢复训练...", None);
        resource_monitor.set_session_id(Some(session_id.clone())).await;

        engine.start_from_checkpoint(handle, checkpoint_epoch, &artifact_dir).await
            .map_err(|e| {
                let error_msg = e.to_string();
                log("TRAINING_SERVICE", "ERROR", Some(&format!("从检查点恢复失败: {}", error_msg)));
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

        log("TRAINING_SERVICE", "步骤8: 启动资源监控和事件循环...", None);
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

        log("TRAINING_SERVICE", "========== 从检查点恢复训练完成 ==========", None);
        log("TRAINING_SERVICE", &format!("experiment_id='{}', checkpoint_epoch={}", exp_id_str, checkpoint_epoch), None);
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
