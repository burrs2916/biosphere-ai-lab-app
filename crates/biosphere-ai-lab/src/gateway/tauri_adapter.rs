use std::sync::Arc;
use std::collections::HashMap;

use crate::core::config::DataLoadConfig;
use crate::core::event::LabEvent;
use crate::gateway::state::AppState;
use crate::hardware::HardwareInfo;
use crate::types::TaskType;
use crate::domain::dataset::aggregate::{Dataset, DatasetId, DatasetFilter, ColumnProfile, ColumnType};
use crate::domain::dataset::handler::DatasetCommand;
use crate::domain::experiment::commands::ExperimentCommand;
use crate::domain::experiment::aggregate::ExperimentId;
use crate::domain::experiment::ExperimentFilter;
use crate::domain::experiment::metrics::MetricsTimeline;
use crate::domain::model::commands::ModelCommand;
use crate::domain::model::aggregate::{ModelId, ModelStatus};
use crate::domain::hardware::ResourceSnapshot;

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_get_resource_snapshot(state: tauri::State<'_, Arc<AppState>>) -> Result<ResourceSnapshot, String> {
    Ok(state.resource_monitor.snapshot().await)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_get_state(state: tauri::State<'_, Arc<AppState>>) -> Result<crate::gateway::state::AppStateSnapshot, String> {
    Ok(state.snapshot().await)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_get_dashboard_stats(state: tauri::State<'_, Arc<AppState>>) -> Result<crate::gateway::state::DashboardStats, String> {
    Ok(state.dashboard_stats().await)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_engines(state: tauri::State<'_, Arc<AppState>>) -> Result<Vec<crate::core::PluginInfo>, String> {
    Ok(state.engine_registry.list().await)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_tasks(state: tauri::State<'_, Arc<AppState>>) -> Result<Vec<crate::core::PluginInfo>, String> {
    Ok(state.task_registry.list().await)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_models(state: tauri::State<'_, Arc<AppState>>) -> Result<Vec<crate::core::PluginInfo>, String> {
    Ok(state.model_registry.list().await)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_data_sources(state: tauri::State<'_, Arc<AppState>>) -> Result<Vec<crate::core::PluginInfo>, String> {
    Ok(state.data_source_registry.list().await)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_get_hardware_info(state: tauri::State<'_, Arc<AppState>>) -> Result<HardwareInfo, String> {
    state.hardware_service.detect().map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_get_recommendations(
    hardware: HardwareInfo,
    task_type: String,
    data_size: usize,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::hardware::TrainingRecommendation, String> {
    let task = match task_type.as_str() {
        "classification" => TaskType::Classification,
        "regression" => TaskType::Regression,
        "clustering" => TaskType::Clustering,
        "detection" => TaskType::Detection,
        "segmentation" => TaskType::Segmentation,
        "generation" => TaskType::Generation,
        "nlp" => TaskType::Nlp,
        _ => TaskType::Custom,
    };
    Ok(state.hardware_service.recommend(&hardware, task, data_size))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_load_data(
    config: DataLoadConfig,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::DatasetInfo, String> {
    if config.path.contains("..") || config.path.contains('~') {
        return Err("Data path contains invalid traversal sequence".to_string());
    }
    let source = state.data_source_registry.find_by_id_str(&config.format.to_string())
        .await
        .ok_or_else(|| format!("不支持的数据格式 '{}'。支持的格式: csv, json, parquet, excel, text, image, binary, tfrecord, huggingface, database", config.format))?;

    let info = source.load(&config).await
        .map_err(|e| format!("数据加载失败 (文件: {}): {}", config.path, e))?;

    state.event_bus.emit(LabEvent::OperationCompleted {
        task_id: config.path.clone(),
        operation: "load_data".to_string(),
        result: serde_json::json!({
            "path": config.path,
            "format": config.format.to_string(),
            "rows": info.rows,
            "columns": info.columns,
        }),
    });

    Ok(info)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_preview_data(
    config: DataLoadConfig,
    offset: Option<usize>,
    limit: Option<usize>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::DataPreview, String> {
    if config.path.contains("..") || config.path.contains('~') {
        return Err("Data path contains invalid traversal sequence".to_string());
    }
    let source = state.data_source_registry.find_by_id_str(&config.format.to_string())
        .await
        .ok_or_else(|| format!("不支持的数据格式 '{}'。支持的格式: csv, json, parquet, excel, text, image, binary, tfrecord, huggingface, database", config.format))?;

    let off = offset.unwrap_or(0);
    let lim = limit.unwrap_or(50);
    source.preview(&config, off, lim).await
        .map_err(|e| format!("数据预览失败 (文件: {}, 偏移: {}, 限制: {}): {}", config.path, off, lim, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_get_model_arch(
    model_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::model::ModelArchDef, String> {
    let model = state.model_registry.find_by_id_str(&model_id)
        .await
        .ok_or_else(|| format!("Model not found: {}", model_id))?;

    model.serialize().map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_create_experiment(
    name: String,
    task_type: String,
    config: crate::core::config::TrainingConfig,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("Experiment name cannot be empty".to_string());
    }
    if !config.data_path.is_empty() && (config.data_path.contains("..") || config.data_path.contains('~')) {
        return Err("Data path contains invalid traversal sequence".to_string());
    }
    if let Err(e) = config.validate() {
        return Err(format!("Invalid training config: {}", e));
    }

    let tt = match task_type.as_str() {
        "classification" => TaskType::Classification,
        "regression" => TaskType::Regression,
        "clustering" => TaskType::Clustering,
        "detection" => TaskType::Detection,
        "segmentation" => TaskType::Segmentation,
        "generation" => TaskType::Generation,
        "nlp" => TaskType::Nlp,
        _ => TaskType::Custom,
    };

    let cmd = ExperimentCommand::CreateExperiment {
        name: name.clone(),
        task_type: tt,
        config,
    };

    let result = state.command_bus.dispatch_experiment(cmd).await
        .map_err(|e| format!("创建实验 '{}' 失败: {}。实验名称可能已存在", name, e))?;

    match result {
        Some(id) => Ok(id.to_string()),
        None => Err(format!("实验 '{}' 创建后无法获取ID，请刷新列表查看", name)),
    }
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_experiments(
    status: Option<String>,
    task_type: Option<String>,
    name_contains: Option<String>,
    group: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<crate::domain::experiment::ExperimentSummary>, String> {
    let mut filter = ExperimentFilter::default();
    if let Some(ref s) = status {
        filter.status = Some(s.parse().map_err(|e: String| format!("无效的实验状态 '{}': {}", s, e))?);
    }
    if let Some(ref tt) = task_type {
        filter.task_type = Some(tt.parse().map_err(|_| format!("无效的任务类型 '{}'。支持: classification, regression, clustering, detection, segmentation, generation, nlp", tt))?);
    }
    filter.name_contains = name_contains;
    filter.group = group;
    state.experiment_repo.list(&filter).await
        .map_err(|e| format!("查询实验列表失败: {}。请稍后重试，如持续失败请检查数据库状态", e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_get_experiment_detail(
    experiment_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::experiment::aggregate::Experiment, String> {
    let id = ExperimentId::from_str(&experiment_id);
    state.experiment_repo.load(&id).await
        .map_err(|e| format!("加载实验 '{}' 失败: {}。实验可能已被删除或数据库异常", experiment_id, e))?
        .ok_or_else(|| format!("实验 '{}' 不存在。可能已被删除或ID不正确", experiment_id))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_query_metrics(
    experiment_id: String,
    metric_names: Vec<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<MetricsTimeline, String> {
    let id = ExperimentId::from_str(&experiment_id);
    state.experiment_repo.query_metrics(&id, &metric_names).await
        .map_err(|e| format!("查询实验 '{}' 指标失败: {}。实验可能不存在或指标数据为空", experiment_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_query_metrics_downsampled(
    experiment_id: String,
    metric_names: Vec<String>,
    max_points: Option<usize>,
    smooth_alpha: Option<f64>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<MetricsTimeline, String> {
    let id = ExperimentId::from_str(&experiment_id);
    let mut timeline = state.experiment_repo.query_metrics(&id, &metric_names).await
        .map_err(|e| format!("查询实验 '{}' 指标失败: {}。实验可能不存在或指标数据为空", experiment_id, e))?;

    let max_pts = max_points.unwrap_or(500);
    let alpha = smooth_alpha.unwrap_or(0.0);

    let series_names: Vec<String> = timeline.series_names();
    for name in series_names {
        if let Some(series) = timeline.get_series_mut(&name) {
            let downsampled = series.downsample_lttb(max_pts);
            let final_points = if alpha > 0.0 && alpha < 1.0 {
                let temp_series = crate::domain::experiment::metrics::MetricSeries {
                    name: series.name.clone(),
                    values: downsampled,
                };
                temp_series.smooth_ema(alpha)
            } else {
                downsampled
            };
            series.values = final_points;
        }
    }

    Ok(timeline)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_load_logs(
    experiment_id: String,
    limit: Option<usize>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<crate::domain::experiment::aggregate::LogEntry>, String> {
    let id = ExperimentId::from_str(&experiment_id);
    let limit = limit.unwrap_or(1000);
    state.experiment_repo.load_logs(&id, limit).await
        .map_err(|e| format!("加载实验 '{}' 日志失败: {}。实验可能不存在", experiment_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_track_metric(
    experiment_id: String,
    metric_name: String,
    value: f64,
    step: u64,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ExperimentCommand::TrackMetric {
        experiment_id: ExperimentId::from_str(&experiment_id),
        metric_name: metric_name.clone(),
        value,
        step,
    };

    state.command_bus.dispatch_experiment(cmd).await
        .map_err(|e| format!("记录实验 '{}' 指标 '{}' 失败: {}。实验可能不存在或已结束", experiment_id, metric_name, e))?;
    Ok(())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_register_model(
    name: String,
    version: String,
    framework: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let cmd = ModelCommand::RegisterModel {
        name: name.clone(),
        version: version.clone(),
        framework,
    };

    state.command_bus.dispatch_model(cmd).await
        .map_err(|e| format!("注册模型 '{}' (版本: {}) 失败: {}。模型名称可能已存在", name, version, e))?;

    Ok("registered".to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_register_model_from_experiment(
    experiment_id: String,
    name: String,
    version: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ModelCommand::RegisterModelFromExperiment {
        experiment_id: crate::domain::experiment::aggregate::ExperimentId::from_str(&experiment_id),
        name: name.clone(),
        version,
    };

    state.command_bus.dispatch_model(cmd).await
        .map_err(|e| format!("从实验 '{}' 注册模型 '{}' 失败: {}。请确认实验已完成训练", experiment_id, name, e))?;

    Ok(())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_model_registrations(
    status_filter: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<crate::domain::model::aggregate::ModelRegistration>, String> {
    let status = match status_filter.as_deref() {
        Some("staging") => Some(ModelStatus::Staging),
        Some("production") => Some(ModelStatus::Production),
        Some("archived") => Some(ModelStatus::Archived),
        Some("none") => Some(ModelStatus::None),
        _ => None,
    };
    state.model_repo.list(status).await
        .map_err(|e| format!("查询模型列表失败: {}。请稍后重试，如持续失败请检查数据库状态", e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_get_model_registration(
    model_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::model::aggregate::ModelRegistration, String> {
    let id = ModelId::from_str(&model_id);
    state.model_repo.load(&id).await
        .map_err(|e| format!("加载模型 '{}' 失败: {}。模型可能已被删除或数据库异常", model_id, e))?
        .ok_or_else(|| format!("模型 '{}' 不存在。可能已被删除或ID不正确", model_id))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_promote_model_staging(
    model_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ModelCommand::PromoteToStaging {
        model_id: ModelId::from_str(&model_id),
    };
    state.command_bus.dispatch_model(cmd).await
        .map_err(|e| format!("将模型 '{}' 提升为 Staging 失败: {}。请确认模型状态正确", model_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_promote_model_production(
    model_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ModelCommand::PromoteToProduction {
        model_id: ModelId::from_str(&model_id),
    };
    state.command_bus.dispatch_model(cmd).await
        .map_err(|e| format!("将模型 '{}' 提升为 Production 失败: {}。请确认模型处于 Staging 状态", model_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_archive_model(
    model_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ModelCommand::ArchiveModel {
        model_id: ModelId::from_str(&model_id),
    };
    state.command_bus.dispatch_model(cmd).await
        .map_err(|e| format!("归档模型 '{}' 失败: {}。请确认模型未被部署", model_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_demote_model_staging(
    model_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ModelCommand::DemoteToStaging {
        model_id: ModelId::from_str(&model_id),
    };
    state.command_bus.dispatch_model(cmd).await
        .map_err(|e| format!("将模型 '{}' 降级为 Staging 失败: {}。请确认模型处于 Production 状态", model_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_add_model_alias(
    model_id: String,
    alias: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ModelCommand::AddModelAlias {
        model_id: ModelId::from_str(&model_id),
        alias,
    };
    state.command_bus.dispatch_model(cmd).await
        .map_err(|e| format!("为模型 '{}' 添加别名失败: {}。别名可能已存在", model_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_remove_model_alias(
    model_id: String,
    alias: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ModelCommand::RemoveModelAlias {
        model_id: ModelId::from_str(&model_id),
        alias,
    };
    state.command_bus.dispatch_model(cmd).await
        .map_err(|e| format!("为模型 '{}' 移除别名失败: {}。请确认别名存在", model_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_delete_model_registration(
    model_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let endpoints = state.model_server.list_endpoints().await;
    if endpoints.iter().any(|e| e.model_id == model_id) {
        return Err(format!("无法删除模型 '{}'：该模型当前正在部署中。请先取消部署后再删除", model_id));
    }

    let cmd = ModelCommand::DeleteModel {
        model_id: ModelId::from_str(&model_id),
    };
    state.command_bus.dispatch_model(cmd).await
        .map_err(|e| format!("删除模型 '{}' 失败: {}。请确认模型存在且未被引用", model_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_set_model_path(
    model_id: String,
    path: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if path.contains("..") || path.contains('~') {
        return Err("模型路径包含无效的遍历序列，请使用绝对路径".to_string());
    }
    let cmd = ModelCommand::SetModelPath {
        model_id: ModelId::from_str(&model_id),
        path,
    };
    state.command_bus.dispatch_model(cmd).await
        .map_err(|e| format!("设置模型 '{}' 路径失败: {}。请检查路径是否存在", model_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_add_model_version(
    model_id: String,
    path: String,
    description: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if path.contains("..") || path.contains('~') {
        return Err("模型路径包含无效的遍历序列，请使用绝对路径".to_string());
    }

    let mut existing = state.model_repo.load(&ModelId::from_str(&model_id))
        .await
        .map_err(|e| format!("加载模型 '{}' 失败: {}。模型可能已被删除", model_id, e))?
        .ok_or_else(|| format!("模型 '{}' 不存在，无法添加新版本", model_id))?;

    let new_version = existing.bump_version();

    let mut new_model = crate::domain::model::aggregate::ModelRegistration::new(
        existing.name.clone(),
        new_version,
        existing.framework.clone(),
    );
    new_model.set_path(path);
    if let Some(desc) = description {
        new_model.set_description(desc);
    }
    new_model.lineage = Some(crate::domain::model::aggregate::ModelLineage {
        parent_model_id: Some(model_id.clone()),
        ..crate::domain::model::aggregate::ModelLineage::default()
    });

    state.model_repo.save(&new_model).await
        .map_err(|e| format!("保存模型新版本失败: {}。请稍后重试", e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_model_versions(
    model_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let model = state.model_repo.load(&ModelId::from_str(&model_id))
        .await
        .map_err(|e| format!("加载模型 '{}' 失败: {}。模型可能已被删除", model_id, e))?;

    let model_name = match model {
        Some(m) => m.name.clone(),
        None => return Ok(Vec::new()),
    };

    let all_versions = state.model_repo.list_by_name(&model_name)
        .await
        .map_err(|e| format!("查询模型 '{}' 版本列表失败: {}。请稍后重试", model_name, e))?;

    let versions: Vec<serde_json::Value> = all_versions.iter().map(|m| {
        serde_json::json!({
            "version": m.version,
            "path": m.path,
            "description": m.description,
            "created_at": m.created_at.to_rfc3339(),
            "size_bytes": 0,
        })
    }).collect();

    Ok(versions)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_set_model_description(
    model_id: String,
    description: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ModelCommand::SetModelDescription {
        model_id: ModelId::from_str(&model_id),
        description,
    };
    state.command_bus.dispatch_model(cmd).await
        .map_err(|e| format!("设置模型 '{}' 描述失败: {}。请确认模型存在", model_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_add_model_tag(
    model_id: String,
    tag: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ModelCommand::AddModelTag {
        model_id: ModelId::from_str(&model_id),
        tag,
    };
    state.command_bus.dispatch_model(cmd).await
        .map_err(|e| format!("为模型 '{}' 添加标签失败: {}。请确认模型存在", model_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_remove_model_tag(
    model_id: String,
    tag: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ModelCommand::RemoveModelTag {
        model_id: ModelId::from_str(&model_id),
        tag,
    };
    state.command_bus.dispatch_model(cmd).await
        .map_err(|e| format!("为模型 '{}' 移除标签失败: {}。请确认标签存在", model_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_experiment_set_description(
    experiment_id: String,
    description: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ExperimentCommand::SetDescription {
        experiment_id: ExperimentId::from_str(&experiment_id),
        description,
    };
    state.command_bus.dispatch_experiment(cmd).await
        .map_err(|e| format!("设置实验 '{}' 描述失败: {}。请确认实验存在", experiment_id, e))?;
    Ok(())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_start_training(
    name: String,
    task_type: String,
    mut config: crate::core::config::TrainingConfig,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let tt = match task_type.as_str() {
        "classification" => TaskType::Classification,
        "regression" => TaskType::Regression,
        "clustering" => TaskType::Clustering,
        "detection" => TaskType::Detection,
        "segmentation" => TaskType::Segmentation,
        "generation" => TaskType::Generation,
        "nlp" => TaskType::Nlp,
        _ => TaskType::Custom,
    };

    if !config.data_path.is_empty() && (config.data_path.contains("..") || config.data_path.contains('~')) {
        return Err("数据路径包含无效的遍历序列，请使用绝对路径".to_string());
    }

    if let Err(e) = config.validate() {
        return Err(format!("训练配置无效: {}。请检查所有必填字段是否完整", e));
    }

    if let Ok(settings) = state.settings_repo.load_all() {
        if config.engine_id.is_empty() || config.engine_id == "burn" {
            let default_engine = settings.training.default_engine.clone();
            if !default_engine.is_empty() {
                config.engine_id = default_engine;
            }
        }
        if config.compute_backend == crate::types::ComputeBackend::Cpu {
            let default_backend = settings.training.default_compute_backend.clone();
            match default_backend.as_str() {
                "cuda" => config.compute_backend = crate::types::ComputeBackend::Cuda,
                "wgpu" => config.compute_backend = crate::types::ComputeBackend::Wgpu,
                "metal" => config.compute_backend = crate::types::ComputeBackend::Metal,
                "rocm" => config.compute_backend = crate::types::ComputeBackend::Rocm,
                _ => {}
            }
        }
        if settings.training.auto_checkpoint && config.checkpoint_interval.is_none() {
            config.checkpoint_interval = Some(settings.training.checkpoint_interval);
        }
    }

    if !config.data_path.is_empty() {
        let original = std::path::Path::new(&config.data_path);
        let stem = original.file_stem().and_then(|s| s.to_str()).unwrap_or("data");
        let ext = original.extension().and_then(|s| s.to_str()).unwrap_or("csv");
        let parent = original.parent().unwrap_or(std::path::Path::new("."));
        let preprocessed = parent.join(format!("{}_preprocessed.{}", stem, ext));
        if preprocessed.exists() {
            config.data_path = preprocessed.to_string_lossy().to_string();
        }
    }

    let create_cmd = ExperimentCommand::CreateExperiment {
        name: name.clone(),
        task_type: tt,
        config: config.clone(),
    };

    let result = state.command_bus.dispatch_experiment(create_cmd).await
        .map_err(|e| format!("创建训练实验 '{}' 失败: {}。实验名称可能已存在", name, e))?;

    let experiment_id = match result {
        Some(id) => id,
        None => return Err(format!("训练实验 '{}' 创建后无法获取ID，请刷新列表查看", name)),
    };

    state.training_service.start_training(experiment_id.clone(), config).await
        .map_err(|e| format!("启动训练 '{}' 失败: {}。请检查数据路径和配置是否正确", experiment_id, e))?;

    Ok(experiment_id.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_stop_training(
    experiment_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let id = ExperimentId::from_str(&experiment_id);
    state.training_service.stop_training(&id).await
        .map_err(|e| format!("停止训练 '{}' 失败: {}。训练可能已经结束", experiment_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_pause_training(
    experiment_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let id = ExperimentId::from_str(&experiment_id);
    state.training_service.pause_training(&id).await
        .map_err(|e| format!("暂停训练 '{}' 失败: {}。训练可能不在运行中", experiment_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_resume_training(
    experiment_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let id = ExperimentId::from_str(&experiment_id);
    state.training_service.resume_training(&id).await
        .map_err(|e| format!("恢复训练 '{}' 失败: {}。训练可能不在暂停状态", experiment_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_resume_from_checkpoint(
    experiment_id: String,
    checkpoint_epoch: usize,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let id = ExperimentId::from_str(&experiment_id);
    state.training_service.resume_from_checkpoint(&id, checkpoint_epoch).await
        .map_err(|e| format!("从检查点恢复训练 '{}' (epoch {}) 失败: {}。检查点可能不存在", experiment_id, checkpoint_epoch, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_get_settings(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::settings::AppSettings, String> {
    state.settings_repo.load_all()
        .map_err(|e| format!("加载设置失败: {}。配置文件可能损坏，请检查", e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_save_settings(
    settings: crate::domain::settings::AppSettings,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.settings_repo.save_all(&settings)
        .map_err(|e| format!("保存设置失败: {}。请检查磁盘空间和写入权限", e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_run_inference(
    experiment_id: String,
    input_data: Vec<Vec<f32>>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::engine::burn_training::InferenceResult, String> {
    if input_data.is_empty() {
        return Err("输入数据不能为空，请提供至少一条数据".to_string());
    }
    for (i, row) in input_data.iter().enumerate() {
        if row.is_empty() {
            return Err(format!("第 {} 行输入数据为空", i + 1));
        }
        if row.iter().any(|v| v.is_nan() || v.is_infinite()) {
            return Err(format!("第 {} 行输入数据包含NaN或无穷值", i + 1));
        }
    }

    let id = ExperimentId::from_str(&experiment_id);
    let experiment = state.experiment_repo.load(&id)
        .await
        .map_err(|e| format!("加载实验 '{}' 失败: {}。实验可能已被删除", experiment_id, e))?
        .ok_or_else(|| format!("实验 '{}' 不存在，无法运行推理", experiment_id))?;

    if !experiment.status.is_terminal() {
        return Err(format!("无法在实验 '{}' 状态为 '{}' 时运行推理，训练必须完成后再进行推理", experiment_id, experiment.status));
    }

    let artifact_dir = crate::core::config::get_artifact_dir(&experiment_id);

    run_inference_dispatch(&experiment.config, &artifact_dir, &input_data)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_data_recipe_create(
    recipe_json: String,
) -> Result<String, String> {
    let recipe: crate::data::data_recipe::DataRecipe = serde_json::from_str(&recipe_json)
        .map_err(|e| format!("数据配方JSON格式无效: {}。请检查JSON语法是否正确，数据集列表是否完整", e))?;
    recipe.validate()?;
    Ok(serde_json::to_string(&recipe).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_data_recipe_validate(
    recipe_json: String,
) -> Result<String, String> {
    let recipe: crate::data::data_recipe::DataRecipe = serde_json::from_str(&recipe_json)
        .map_err(|e| format!("数据配方JSON格式无效: {}。请检查JSON语法是否正确，数据集列表是否完整", e))?;
    match recipe.validate() {
        Ok(()) => Ok(serde_json::to_string(&serde_json::json!({
            "valid": true,
            "name": recipe.name,
            "datasets": recipe.datasets.len(),
            "total_weight": recipe.total_weight(),
        })).unwrap_or_default()),
        Err(e) => Ok(serde_json::to_string(&serde_json::json!({
            "valid": false,
            "error": e,
        })).unwrap_or_default()),
    }
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_data_recipe_execute(
    recipe_json: String,
    num_samples: Option<usize>,
) -> Result<String, String> {
    use crate::data::data_recipe::{DataRecipe, RecipeExecutor};

    let mut recipe: DataRecipe = serde_json::from_str(&recipe_json)
        .map_err(|e| format!("数据配方JSON格式无效: {}。请检查JSON语法是否正确，数据集列表是否完整", e))?;

    if let Some(n) = num_samples {
        recipe.total_samples_target = Some(n);
    }

    let mut executor = RecipeExecutor::new(recipe)?;
    let mut sequence = Vec::new();
    let limit = num_samples.unwrap_or(100);

    for _ in 0..limit {
        if let Some(name) = executor.next_dataset() {
            sequence.push(name);
        } else {
            break;
        }
    }

    let stats = executor.stats();
    Ok(serde_json::to_string(&serde_json::json!({
        "sequence": sequence,
        "stats": stats,
    })).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_data_recipe_presets(
    preset_type: String,
) -> Result<String, String> {
    use crate::data::data_recipe;

    let recipe = match preset_type.as_str() {
        "llm_pretraining" => data_recipe::create_llm_pretraining_recipe(),
        "sft" => data_recipe::create_sft_recipe(),
        "rlhf" => data_recipe::create_rlhf_preference_recipe(),
        _ => return Err(format!("未知的预设类型 '{}'。支持的预设: llm_pretraining, sft, rlhf", preset_type)),
    };

    Ok(serde_json::to_string(&recipe).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_hf_hub_search(
    query: String,
    limit: Option<usize>,
) -> Result<String, String> {
    use crate::data::hf_hub::{HfHubClient, HfHubConfig};

    let config = HfHubConfig::default();
    let client = HfHubClient::new(config)?;
    let results = client.search_datasets(&query, limit, None)?;
    Ok(serde_json::to_string(&results).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_hf_hub_info(
    dataset_id: String,
) -> Result<String, String> {
    use crate::data::hf_hub::{HfHubClient, HfHubConfig};

    let config = HfHubConfig::default();
    let client = HfHubClient::new(config)?;
    let info = client.dataset_info(&dataset_id)?;
    Ok(serde_json::to_string(&info).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_hf_hub_download(
    dataset_id: String,
    config_name: String,
    split: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    use crate::data::hf_hub::{HfHubClient, HfHubConfig};

    let task_id = format!("hf_download_{}_{}", dataset_id, split);

    state.event_bus.emit(LabEvent::DownloadProgress {
        task_id: task_id.clone(),
        progress: 0.0,
        downloaded_bytes: 0,
        total_bytes: None,
        speed_mbps: 0.0,
        message: format!("开始下载 {} / {} / {}", dataset_id, config_name, split),
    });

    let config = HfHubConfig::default();
    let client = HfHubClient::new(config)
        .map_err(|e| format!("无法连接 HuggingFace Hub: {}。请检查网络连接", e))?;
    let files = client.load_dataset(&dataset_id, &config_name, &split)
        .map_err(|e| format!("下载数据集 '{}' (配置: {}, 分割: {}) 失败: {}。请确认数据集ID、配置名和分割名是否正确", dataset_id, config_name, split, e))?;

    let file_count = files.len();
    state.event_bus.emit(LabEvent::DownloadProgress {
        task_id: task_id.clone(),
        progress: 100.0,
        downloaded_bytes: 0,
        total_bytes: None,
        speed_mbps: 0.0,
        message: format!("下载完成: {} 个文件", file_count),
    });

    state.event_bus.emit(LabEvent::OperationCompleted {
        task_id,
        operation: "hf_hub_download".to_string(),
        result: serde_json::json!({
            "dataset_id": dataset_id,
            "config": config_name,
            "split": split,
            "file_count": file_count,
        }),
    });

    Ok(serde_json::to_string(&serde_json::json!({
        "dataset_id": dataset_id,
        "config": config_name,
        "split": split,
        "files": files.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>(),
        "count": file_count,
    })).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_hf_hub_cached() -> Result<String, String> {
    use crate::data::hf_hub::{HfHubClient, HfHubConfig};

    let config = HfHubConfig::default();
    let client = HfHubClient::new(config)?;
    let datasets = client.cached_datasets()?;
    let size = client.cache_size()?;
    Ok(serde_json::to_string(&serde_json::json!({
        "cached_datasets": datasets,
        "cache_size_bytes": size,
    })).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_hf_hub_popular() -> Result<String, String> {
    use crate::data::hf_hub;

    let datasets = hf_hub::list_popular_datasets();
    let result: Vec<serde_json::Value> = datasets.iter().map(|(id, cfg, split, desc)| {
        serde_json::json!({
            "dataset_id": id,
            "config": cfg,
            "split": split,
            "description": desc,
        })
    }).collect();

    Ok(serde_json::to_string(&result).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_global_dedup_create(
    similarity_threshold: Option<f64>,
    num_permutations: Option<usize>,
) -> Result<String, String> {
    use crate::data::global_dedup::{GlobalDedupConfig, GlobalDeduper};

    let config = GlobalDedupConfig {
        similarity_threshold: similarity_threshold.unwrap_or(0.8),
        num_permutations: num_permutations.unwrap_or(128),
        ..Default::default()
    };

    let deduper = GlobalDeduper::new(config)?;
    let stats = deduper.stats();
    Ok(serde_json::to_string(&stats).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_global_dedup_process(
    documents_json: String,
    similarity_threshold: Option<f64>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    use crate::data::global_dedup::{GlobalDedupConfig, GlobalDeduper};

    let task_id = format!("dedup_{}", uuid::Uuid::new_v4());

    let config = GlobalDedupConfig {
        similarity_threshold: similarity_threshold.unwrap_or(0.8),
        ..Default::default()
    };

    let mut deduper = GlobalDeduper::new(config)?;

    let documents: Vec<serde_json::Value> = serde_json::from_str(&documents_json)
        .map_err(|e| format!("文档JSON格式无效: {}。请确保传入的是有效的JSON数组，每个元素包含 id、dataset、text 字段", e))?;

    let total = documents.len();
    state.event_bus.emit(LabEvent::DedupProgress {
        task_id: task_id.clone(),
        progress: 0.0,
        processed: 0,
        total,
        duplicates_found: 0,
        message: format!("开始去重处理 {} 条文档", total),
    });

    let mut duplicates_found = Vec::new();
    for (i, doc) in documents.iter().enumerate() {
        let doc_id = doc["id"].as_str().unwrap_or("unknown");
        let dataset = doc["dataset"].as_str().unwrap_or("default");
        let text = doc["text"].as_str().unwrap_or("");

        if let Some(dups) = deduper.process_document(doc_id, dataset, text) {
            duplicates_found.push(serde_json::json!({
                "doc_id": doc_id,
                "duplicates": dups,
            }));
        }

        if i % 100 == 0 || i == total - 1 {
            let progress = ((i + 1) as f64 / total as f64) * 100.0;
            state.event_bus.emit(LabEvent::DedupProgress {
                task_id: task_id.clone(),
                progress,
                processed: i + 1,
                total,
                duplicates_found: duplicates_found.len(),
                message: format!("去重进度: {}/{}", i + 1, total),
            });
        }
    }

    let report = deduper.generate_report();
    let dup_count = duplicates_found.len();

    state.event_bus.emit(LabEvent::OperationCompleted {
        task_id,
        operation: "global_dedup".to_string(),
        result: serde_json::json!({
            "total_documents": total,
            "duplicates_found": dup_count,
            "report": report,
        }),
    });

    Ok(serde_json::to_string(&serde_json::json!({
        "duplicates_found": duplicates_found,
        "report": report,
    })).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_global_dedup_report(
    documents_json: String,
) -> Result<String, String> {
    use crate::data::global_dedup::{GlobalDedupConfig, GlobalDeduper};

    let config = GlobalDedupConfig::default();
    let mut deduper = GlobalDeduper::new(config)?;

    let documents: Vec<serde_json::Value> = serde_json::from_str(&documents_json)
        .map_err(|e| format!("文档JSON格式无效: {}。请确保传入的是有效的JSON数组，每个元素包含 id、dataset、text 字段", e))?;

    for doc in &documents {
        let doc_id = doc["id"].as_str().unwrap_or("unknown");
        let dataset = doc["dataset"].as_str().unwrap_or("default");
        let text = doc["text"].as_str().unwrap_or("");
        deduper.process_document(doc_id, dataset, text);
    }

    let report = deduper.generate_report();
    Ok(serde_json::to_string(&report).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_training_plan_create(
    plan_json: String,
) -> Result<String, String> {
    let plan: crate::data::training_plan::TrainingPlan = serde_json::from_str(&plan_json)
        .map_err(|e| format!("训练计划JSON格式无效: {}。请检查JSON语法是否正确，所有必填字段是否完整", e))?;
    let result = plan.validate()?;
    Ok(serde_json::to_string(&result).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_training_plan_validate(
    plan_json: String,
) -> Result<String, String> {
    let plan: crate::data::training_plan::TrainingPlan = serde_json::from_str(&plan_json)
        .map_err(|e| format!("训练计划JSON格式无效: {}。请检查JSON语法是否正确，所有必填字段是否完整", e))?;
    let result = plan.validate()?;
    Ok(serde_json::to_string(&result).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_training_plan_summarize(
    plan_json: String,
) -> Result<String, String> {
    let plan: crate::data::training_plan::TrainingPlan = serde_json::from_str(&plan_json)
        .map_err(|e| format!("训练计划JSON格式无效: {}。请检查JSON语法是否正确，所有必填字段是否完整", e))?;
    let summary = plan.summarize();
    Ok(serde_json::to_string(&summary).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_training_plan_presets(
    preset_type: String,
) -> Result<String, String> {
    use crate::data::training_plan;

    let plan = match preset_type.as_str() {
        "llm_pretraining" => training_plan::create_standard_llm_pretraining_plan(),
        "sft" => training_plan::create_sft_training_plan(),
        "rlhf" => training_plan::create_rlhf_training_plan(),
        _ => return Err(format!("未知的预设类型 '{}'。支持的预设: llm_pretraining, sft, rlhf", preset_type)),
    };

    Ok(serde_json::to_string(&plan).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_training_plan_save(
    plan_json: String,
) -> Result<String, String> {
    let plan: crate::data::training_plan::TrainingPlan = serde_json::from_str(&plan_json)
        .map_err(|e| format!("训练计划JSON格式无效: {}。请检查JSON语法是否正确", e))?;

    let plans_dir = get_plans_dir()?;
    std::fs::create_dir_all(&plans_dir)
        .map_err(|e| format!("无法创建计划存储目录: {}。请检查磁盘权限", e))?;

    let plan_id = format!("{}-{}", plan.name.replace(' ', "_"), plan.version);
    let file_path = plans_dir.join(format!("{}.json", plan_id));
    let json = serde_json::to_string_pretty(&plan)
        .map_err(|e| format!("序列化训练计划失败: {}", e))?;
    std::fs::write(&file_path, &json)
        .map_err(|e| format!("保存训练计划 '{}' 失败: {}。请检查磁盘空间", plan.name, e))?;

    Ok(plan_id)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_training_plan_list(
) -> Result<Vec<serde_json::Value>, String> {
    let plans_dir = get_plans_dir()?;
    if !plans_dir.exists() {
        return Ok(Vec::new());
    }

    let mut plans = Vec::new();
    let entries = std::fs::read_dir(&plans_dir)
        .map_err(|e| format!("无法读取计划目录: {}。请检查目录权限", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("读取计划文件失败: {}", e))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(plan) = serde_json::from_str::<crate::data::training_plan::TrainingPlan>(&content) {
                    let file_name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("unknown");
                    let modified = std::fs::metadata(&path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            let duration = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                            chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0)
                                .map(|dt| dt.to_rfc3339())
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();

                    let summary = plan.summarize();
                    plans.push(serde_json::json!({
                        "id": file_name,
                        "name": plan.name,
                        "version": plan.version,
                        "description": plan.description,
                        "plan_type": plan.plan_type,
                        "phases_count": plan.phases.len(),
                        "datasets_count": plan.phases.iter().map(|p| p.recipe.datasets.len()).sum::<usize>(),
                        "total_estimated_tokens": summary.total_estimated_tokens,
                        "total_estimated_steps": summary.total_estimated_steps,
                        "estimated_gpu_hours": summary.estimated_gpu_hours,
                        "modified_at": modified,
                    }));
                }
            }
        }
    }

    plans.sort_by(|a, b| {
        b.get("modified_at").and_then(|v| v.as_str()).unwrap_or("")
            .cmp(&a.get("modified_at").and_then(|v| v.as_str()).unwrap_or(""))
    });

    Ok(plans)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_training_plan_load(
    plan_id: String,
) -> Result<String, String> {
    let plans_dir = get_plans_dir()?;
    let file_path = plans_dir.join(format!("{}.json", plan_id));

    if !file_path.exists() {
        return Err(format!("训练计划 '{}' 不存在。可能已被删除", plan_id));
    }

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("读取训练计划 '{}' 失败: {}。文件可能已损坏", plan_id, e))?;

    let _plan: crate::data::training_plan::TrainingPlan = serde_json::from_str(&content)
        .map_err(|e| format!("解析训练计划 '{}' 失败: {}。JSON格式可能已损坏", plan_id, e))?;

    Ok(content)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_training_plan_delete(
    plan_id: String,
) -> Result<(), String> {
    let plans_dir = get_plans_dir()?;
    let file_path = plans_dir.join(format!("{}.json", plan_id));

    if !file_path.exists() {
        return Err(format!("训练计划 '{}' 不存在，无法删除", plan_id));
    }

    std::fs::remove_file(&file_path)
        .map_err(|e| format!("删除训练计划 '{}' 失败: {}。请检查文件权限", plan_id, e))?;

    Ok(())
}

fn get_plans_dir() -> Result<std::path::PathBuf, String> {
    let dir = dirs::data_dir()
        .ok_or_else(|| "无法获取应用数据目录".to_string())?
        .join("biosphere-ai-lab")
        .join("plans");
    Ok(dir)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_tokenizer_load(
    tokenizer_path: String,
    max_length: Option<usize>,
) -> Result<crate::data::tokenizer::TokenizerInfo, String> {
    let config = crate::data::tokenizer::TokenizerConfig {
        tokenizer_path: tokenizer_path.clone(),
        max_length: max_length.unwrap_or(512),
        ..Default::default()
    };
    let pipeline = crate::data::tokenizer::TokenizerPipeline::from_file(&tokenizer_path, config)?;
    Ok(pipeline.tokenizer_info())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_tokenizer_encode(
    tokenizer_path: String,
    text: String,
    max_length: Option<usize>,
) -> Result<crate::data::tokenizer::BatchEncoding, String> {
    let config = crate::data::tokenizer::TokenizerConfig {
        tokenizer_path: tokenizer_path.clone(),
        max_length: max_length.unwrap_or(512),
        ..Default::default()
    };
    let pipeline = crate::data::tokenizer::TokenizerPipeline::from_file(&tokenizer_path, config)?;
    pipeline.encode(&text)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_tokenizer_encode_batch(
    tokenizer_path: String,
    texts: Vec<String>,
    max_length: Option<usize>,
) -> Result<crate::data::tokenizer::BatchEncoding, String> {
    let config = crate::data::tokenizer::TokenizerConfig {
        tokenizer_path: tokenizer_path.clone(),
        max_length: max_length.unwrap_or(512),
        ..Default::default()
    };
    let pipeline = crate::data::tokenizer::TokenizerPipeline::from_file(&tokenizer_path, config)?;
    pipeline.encode_batch(&texts, None)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_tokenizer_decode(
    tokenizer_path: String,
    ids: Vec<u32>,
    skip_special_tokens: Option<bool>,
) -> Result<String, String> {
    let config = crate::data::tokenizer::TokenizerConfig {
        tokenizer_path: tokenizer_path.clone(),
        ..Default::default()
    };
    let pipeline = crate::data::tokenizer::TokenizerPipeline::from_file(&tokenizer_path, config)?;
    pipeline.decode(&ids, skip_special_tokens.unwrap_or(true))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_chat_template_apply(
    template_name: String,
    messages: Vec<crate::data::chat_template::Message>,
    add_generation_prompt: Option<bool>,
) -> Result<String, String> {
    let template = match template_name.as_str() {
        "llama3" => crate::data::chat_template::ChatTemplate::llama3(),
        "chatml" => crate::data::chat_template::ChatTemplate::chatml(),
        "mistral" => crate::data::chat_template::ChatTemplate::mistral(),
        "zephyr" => crate::data::chat_template::ChatTemplate::zephyr(),
        "phi3" => crate::data::chat_template::ChatTemplate::phi3(),
        "gemma" => crate::data::chat_template::ChatTemplate::gemma(),
        _ => return Err(format!("Unknown template: {}", template_name)),
    };

    let conversation = crate::data::chat_template::Conversation {
        messages,
        metadata: None,
    };

    template.apply(&conversation, add_generation_prompt.unwrap_or(false))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_data_collate(
    batch: Vec<crate::data::tokenizer::BatchEncoding>,
    pad_token_id: u32,
    pad_to_multiple_of: Option<usize>,
) -> Result<crate::data::tokenizer::BatchEncoding, String> {
    let config = crate::data::data_collator::DataCollatorConfig {
        pad_to_multiple_of,
        ..Default::default()
    };
    let collator = crate::data::data_collator::DataCollator::new(pad_token_id, config);
    collator.collate(&batch)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_curation_config(
) -> Result<crate::data::curation::CurationConfig, String> {
    Ok(crate::data::curation::CurationConfig::default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_curation_mask_pii(
    text: String,
) -> Result<String, String> {
    let config = crate::data::curation::CurationConfig::default();
    let curator = crate::data::curation::DataCurator::new(config);
    Ok(curator.mask_pii(&text))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_prefetch_open_csv(
    name: String,
    path: String,
    chunk_size: Option<usize>,
    buffer_size: Option<usize>,
) -> Result<crate::data::prefetch::PrefetchStats, String> {
    let streaming_config = crate::data::streaming::StreamingConfig {
        chunk_size: chunk_size.unwrap_or(10000),
        ..Default::default()
    };
    let prefetch_config = crate::data::prefetch::PrefetchConfig {
        buffer_size: buffer_size.unwrap_or(4),
        ..Default::default()
    };
    let dataset = crate::data::prefetch::PrefetchStreamingDataset::open_csv(
        &name, &path, streaming_config, prefetch_config,
    )?;
    while let Ok(Some(_)) = dataset.next_chunk() {}
    Ok(dataset.stats())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_prefetch_open_jsonl(
    name: String,
    path: String,
    chunk_size: Option<usize>,
    buffer_size: Option<usize>,
) -> Result<crate::data::prefetch::PrefetchStats, String> {
    let streaming_config = crate::data::streaming::StreamingConfig {
        chunk_size: chunk_size.unwrap_or(10000),
        ..Default::default()
    };
    let prefetch_config = crate::data::prefetch::PrefetchConfig {
        buffer_size: buffer_size.unwrap_or(4),
        ..Default::default()
    };
    let dataset = crate::data::prefetch::PrefetchStreamingDataset::open_jsonl(
        &name, &path, streaming_config, prefetch_config,
    )?;
    while let Ok(Some(_)) = dataset.next_chunk() {}
    Ok(dataset.stats())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_prefetch_compression_info(
    path: String,
) -> Result<crate::data::prefetch::CompressionFormat, String> {
    Ok(crate::data::prefetch::CompressionFormat::from_extension(&path))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_sampler_distributed(
    total_size: usize,
    num_replicas: usize,
    rank: usize,
    shuffle: Option<bool>,
    seed: Option<u64>,
    drop_last: Option<bool>,
) -> Result<Vec<usize>, String> {
    let config = crate::data::sampler::DistributedSamplerConfig {
        num_replicas,
        rank,
        shuffle: shuffle.unwrap_or(true),
        seed: seed.unwrap_or(42),
        drop_last: drop_last.unwrap_or(false),
    };
    let sampler = crate::data::sampler::DistributedSampler::new(total_size, config);
    Ok(sampler.iter().collect())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_sampler_split(
    total_size: usize,
    train_ratio: f64,
    val_ratio: f64,
    test_ratio: f64,
    seed: Option<u64>,
) -> Result<(Vec<usize>, Vec<usize>, Vec<usize>), String> {
    Ok(crate::data::sampler::compute_split_indices(
        total_size, train_ratio, val_ratio, test_ratio, seed.unwrap_or(42),
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_sampler_kfold(
    total_size: usize,
    k: usize,
    seed: Option<u64>,
) -> Result<Vec<(Vec<usize>, Vec<usize>)>, String> {
    Ok(crate::data::sampler::compute_kfold_indices(
        total_size, k, seed.unwrap_or(42),
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_sampler_stratified(
    labels: Vec<usize>,
    num_classes: usize,
    train_ratio: f64,
    val_ratio: f64,
    test_ratio: f64,
    seed: Option<u64>,
) -> Result<(Vec<usize>, Vec<usize>, Vec<usize>), String> {
    Ok(crate::data::sampler::compute_stratified_split_indices(
        &labels, num_classes, train_ratio, val_ratio, test_ratio, seed.unwrap_or(42),
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_monitor_start(
    window_size: Option<usize>,
) -> Result<crate::data::monitor::MonitorSnapshot, String> {
    let config = crate::data::monitor::MonitorConfig {
        window_size: window_size.unwrap_or(1000),
        ..Default::default()
    };
    let monitor = crate::data::monitor::DataMonitor::new(config);
    monitor.start();
    Ok(monitor.snapshot())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_monitor_snapshot(
) -> Result<crate::data::monitor::MonitorSnapshot, String> {
    let config = crate::data::monitor::MonitorConfig::default();
    let monitor = crate::data::monitor::DataMonitor::new(config);
    Ok(monitor.snapshot())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_start_hyperparameter_tuning(
    tune_config_json: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::training::TuneResult, String> {
    if tune_config_json.trim().is_empty() {
        return Err("Tune config cannot be empty".to_string());
    }

    let tune_config: crate::domain::training::TuneConfig = serde_json::from_str(&tune_config_json)
        .map_err(|e| format!("Invalid tune config: {}", e))?;

    if tune_config.metric_to_optimize.trim().is_empty() {
        return Err("metric_to_optimize cannot be empty".to_string());
    }

    if tune_config.hparam_space.params.is_empty() {
        return Err("Hyperparameter space must contain at least one parameter".to_string());
    }

    if tune_config.max_concurrent > 10 {
        return Err("max_concurrent cannot exceed 10".to_string());
    }

    let tune_id = uuid::Uuid::new_v4().to_string();
    let mut tune_result = crate::domain::training::TuneResult::new(
        tune_id.clone(),
        tune_config.strategy.clone(),
    );

    let trials = crate::domain::training::generate_trials(&tune_config);
    let metric_name = tune_config.metric_to_optimize.clone();
    let maximize = tune_config.maximize;
    let max_concurrent = tune_config.max_concurrent.max(1);

    crate::infrastructure::log("TUNING", &format!(
        "启动超参数调优: strategy={:?}, {} 个试验, max_concurrent={}",
        tune_config.strategy,
        trials.len(),
        max_concurrent
    ), None);

    let mut running_trials: Vec<(usize, ExperimentId)> = Vec::new();
    let mut next_trial = 0;

    while next_trial < trials.len() || !running_trials.is_empty() {
        while running_trials.len() < max_concurrent && next_trial < trials.len() {
            let i = next_trial;
            let params = &trials[i];
            let trial_config = crate::domain::training::apply_params_to_config(&tune_config.base_config, params);

            let experiment_name = format!("{}_trial_{}", tune_config.base_config.session_name, i);
            let mut trial_config = trial_config;
            trial_config.session_name = experiment_name.clone();

            let experiment_id = ExperimentId::new();
            let mut experiment = crate::domain::experiment::Experiment::create(
                experiment_name,
                trial_config.clone(),
            );

            for (key, value) in params {
                experiment.set_param(key.clone(), serde_json::to_value(value).unwrap_or_default());
            }
            experiment.add_tag("hyperparameter_tuning".to_string());
            experiment.add_tag(format!("tune_id_{}", tune_id));

            state.experiment_repo.save(&experiment).await
                .map_err(|e| e.to_string())?;

            tune_result.trials.push(crate::domain::training::TrialResult {
                experiment_id: experiment_id.to_string(),
                params: params.clone(),
                metric_value: None,
                status: crate::domain::training::TrialStatus::Pending,
            });

            match state.training_service.start_training(experiment_id.clone(), trial_config).await {
                Ok(_) => {
                    if let Some(last) = tune_result.trials.last_mut() {
                        last.status = crate::domain::training::TrialStatus::Running;
                    }
                    crate::infrastructure::log("TUNING", &format!(
                        "试验 {}/{} 已启动: experiment={}",
                        i + 1,
                        trials.len(),
                        experiment_id
                    ), None);
                    running_trials.push((i, experiment_id));
                }
                Err(e) => {
                    if let Some(last) = tune_result.trials.last_mut() {
                        last.status = crate::domain::training::TrialStatus::Failed;
                    }
                    crate::infrastructure::log("TUNING", &format!(
                        "试验 {}/{} 启动失败: {}",
                        i + 1,
                        trials.len(),
                        e
                    ), None);
                }
            }

            next_trial += 1;
        }

        if !running_trials.is_empty() {
            wait_for_any_trial_completion(&running_trials, &mut tune_result, &metric_name, &state).await;
            running_trials.retain(|(idx, _)| {
                tune_result.trials.get(*idx).map_or(true, |t| !matches!(t.status, crate::domain::training::TrialStatus::Completed | crate::domain::training::TrialStatus::Failed))
            });
        }
    }

    tune_result.update_best(maximize);
    crate::infrastructure::log("TUNING", &format!(
        "超参数调优完成: best_params={:?}",
        tune_result.best_params
    ), None);

    Ok(tune_result)
}

#[cfg(feature = "tauri")]
async fn wait_for_any_trial_completion(
    running_trials: &[(usize, ExperimentId)],
    tune_result: &mut crate::domain::training::TuneResult,
    metric_name: &str,
    state: &tauri::State<'_, Arc<AppState>>,
) {
    let max_wait_secs = 3600;
    let poll_interval = std::time::Duration::from_secs(3);
    let start = std::time::Instant::now();

    loop {
        if start.elapsed().as_secs() >= max_wait_secs as u64 {
            for (trial_idx, _) in running_trials {
                if let Some(trial) = tune_result.trials.get_mut(*trial_idx) {
                    if !matches!(trial.status, crate::domain::training::TrialStatus::Completed | crate::domain::training::TrialStatus::Failed) {
                        trial.status = crate::domain::training::TrialStatus::Failed;
                    }
                }
            }
            return;
        }

        let mut any_completed = false;

        for (trial_idx, experiment_id) in running_trials {
            if let Some(trial) = tune_result.trials.get(*trial_idx) {
                if matches!(trial.status, crate::domain::training::TrialStatus::Completed | crate::domain::training::TrialStatus::Failed) {
                    continue;
                }
            }

            match state.experiment_repo.load(experiment_id).await {
                Ok(Some(experiment)) => {
                    let status = experiment.status;
                    if status.is_terminal() {
                        any_completed = true;

                        if let Some(trial) = tune_result.trials.get_mut(*trial_idx) {
                            if status == crate::domain::experiment::aggregate::ExperimentStatus::Completed {
                                trial.status = crate::domain::training::TrialStatus::Completed;

                                if let Ok(metrics) = state.experiment_repo.query_metrics(experiment_id, &[metric_name.to_string()]).await {
                                    if let Some(series) = metrics.get_series(metric_name) {
                                        if !series.values.is_empty() {
                                            let is_loss = metric_name.contains("loss") || metric_name.contains("error");
                                            let best = if is_loss {
                                                series.values.iter().map(|p| p.value).fold(f64::INFINITY, f64::min)
                                            } else {
                                                series.values.iter().map(|p| p.value).fold(f64::NEG_INFINITY, f64::max)
                                            };
                                            trial.metric_value = Some(best);
                                        }
                                    }
                                }
                            } else {
                                trial.status = crate::domain::training::TrialStatus::Failed;
                            }
                        }

                        crate::infrastructure::log("TUNING", &format!(
                            "试验 {} 完成: experiment={}, metric={:?}",
                            trial_idx + 1,
                            experiment_id,
                            tune_result.trials.get(*trial_idx).and_then(|t| t.metric_value)
                        ), None);
                    }
                }
                Ok(None) => {
                    any_completed = true;
                    if let Some(trial) = tune_result.trials.get_mut(*trial_idx) {
                        trial.status = crate::domain::training::TrialStatus::Failed;
                    }
                }
                Err(_) => {}
            }
        }

        if any_completed {
            return;
        }

        tokio::time::sleep(poll_interval).await;
    }
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_generate_hparam_combinations(
    hparam_space_json: String,
    strategy_json: String,
) -> Result<Vec<HashMap<String, crate::domain::training::HparamValue>>, String> {
    let space: crate::domain::training::HparamSpace = serde_json::from_str(&hparam_space_json)
        .map_err(|e| format!("Invalid hparam space: {}", e))?;

    let strategy: crate::domain::training::TuneStrategy = serde_json::from_str(&strategy_json)
        .map_err(|e| format!("Invalid strategy: {}", e))?;

    let combinations = match strategy {
        crate::domain::training::TuneStrategy::Grid => space.grid_combinations(),
        crate::domain::training::TuneStrategy::Random { n_trials } => space.random_combinations(n_trials),
    };

    Ok(combinations)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_export_model(
    experiment_id: String,
    format: String,
    output_path: Option<String>,
    opset_version: Option<i64>,
    input_shapes: Vec<Vec<i64>>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::model::ExportResult, String> {
    if let Some(ref p) = output_path {
        if p.contains("..") || p.contains('~') {
            return Err("Output path contains invalid traversal sequence".to_string());
        }
    }

    let id = ExperimentId::from_str(&experiment_id);
    let experiment = state.experiment_repo.load(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Experiment not found".to_string())?;

    if !experiment.status.is_terminal() {
        return Err(format!("Cannot export model from experiment in {} state, training must be completed first", experiment.status));
    }

    let export_format = match format.as_str() {
        "torchscript" => crate::domain::model::ExportFormat::TorchScript,
        "onnx" => crate::domain::model::ExportFormat::Onnx,
        "burn_record" => crate::domain::model::ExportFormat::BurnRecord,
        _ => return Err(format!("Unknown export format: {}", format)),
    };

    let request = crate::domain::model::ExportRequest {
        experiment_id: experiment_id.clone(),
        format: export_format,
        output_path,
        opset_version,
        input_shapes,
    };

    let result = crate::domain::model::export_model(&request, &experiment.config);
    Ok(result)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_export_formats(
    engine_id: String,
) -> Result<Vec<String>, String> {
    let formats = match engine_id.as_str() {
        "tch" => vec!["torchscript".to_string(), "onnx".to_string()],
        "burn" => vec!["burn_record".to_string()],
        _ => vec![],
    };
    Ok(formats)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_model_deploy(
    model_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.model_server.deploy(&model_id).await
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_model_undeploy(
    model_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.model_server.undeploy(&model_id).await
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_model_predict(
    model_id: String,
    inputs: Vec<Vec<f32>>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::model::ServeResponse, String> {
    let request = crate::domain::model::ServeRequest {
        model_id,
        inputs,
    };
    state.model_server.predict(request).await
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_model_list_endpoints(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<crate::domain::model::ServeEndpoint>, String> {
    Ok(state.model_server.list_endpoints().await)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_model_serve_stats(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::model::ServeStats, String> {
    Ok(state.model_server.get_stats().await)
}

#[cfg(feature = "tauri")]
fn run_inference_dispatch(
    config: &crate::core::config::TrainingConfig,
    artifact_dir: &str,
    input_data: &[Vec<f32>],
) -> Result<crate::engine::burn_training::InferenceResult, String> {
    let engine_id = &config.engine_id;

    if engine_id == "tch" {
        #[cfg(feature = "tch-engine")]
        {
            let result = crate::engine::tch_engine::run_tch_inference(config, artifact_dir, input_data)
                .map_err(|e| e.to_string())?;
            Ok(crate::engine::burn_training::InferenceResult {
                predictions: result.predictions,
                predicted_classes: result.predicted_classes,
                probabilities: result.probabilities,
            })
        }
        #[cfg(not(feature = "tch-engine"))]
        {
            Err("TchEngine not available (tch-engine feature not enabled)".to_string())
        }
    } else {
        crate::engine::burn_training::run_inference(config, artifact_dir, input_data)
            .map_err(|e| e.to_string())
    }
}

#[cfg(feature = "tauri")]
fn compute_column_profiles(headers: &[String], records: &[csv::StringRecord]) -> Vec<ColumnProfile> {
    let num_cols = headers.len();
    let mut profiles = Vec::with_capacity(num_cols);

    for col_idx in 0..num_cols {
        let col_name = headers[col_idx].clone();
        let mut null_count = 0usize;
        let mut int_values = Vec::new();
        let mut float_values = Vec::new();
        let mut string_values = Vec::new();
        let mut bool_count = 0usize;
        let mut date_count = 0usize;
        let mut distinct_set = std::collections::HashSet::new();

        for record in records {
            let val = record.get(col_idx).unwrap_or("").trim();
            if val.is_empty() {
                null_count += 1;
                continue;
            }
            distinct_set.insert(val.to_string());

            if let Ok(v) = val.parse::<f64>() {
                if val.parse::<i64>().is_ok() {
                    int_values.push(v);
                } else {
                    float_values.push(v);
                }
            } else if val == "true" || val == "false" || val == "True" || val == "False" || val == "1" || val == "0" {
                bool_count += 1;
            } else if val.parse::<chrono::NaiveDateTime>().is_ok() || val.parse::<chrono::NaiveDate>().is_ok() {
                date_count += 1;
            }
            string_values.push(val.to_string());
        }

        let total_count = records.len();
        let distinct_count = distinct_set.len();
        let numeric_values: Vec<f64> = int_values.iter().chain(float_values.iter()).cloned().collect();
        let numeric_count = numeric_values.len();

        let column_type = if total_count == 0 || null_count == total_count {
            ColumnType::Unknown
        } else if numeric_count as f64 / (total_count - null_count) as f64 > 0.8 {
            if int_values.len() > float_values.len() && int_values.len() as f64 / (total_count - null_count) as f64 > 0.8 {
                ColumnType::Integer
            } else {
                ColumnType::Float
            }
        } else if bool_count as f64 / (total_count - null_count) as f64 > 0.8 {
            ColumnType::Boolean
        } else if date_count as f64 / (total_count - null_count) as f64 > 0.5 {
            ColumnType::DateTime
        } else if distinct_count <= 20 && distinct_count < (total_count - null_count) / 2 {
            ColumnType::Categorical
        } else {
            ColumnType::String
        };

        let (min_value, max_value, mean_value, std_value, median_value) = if !numeric_values.is_empty() {
            let mut sorted = numeric_values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mean = sorted.iter().sum::<f64>() / sorted.len() as f64;
            let variance = sorted.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / sorted.len() as f64;
            let std = variance.sqrt();
            let median = if sorted.len() % 2 == 0 {
                (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
            } else {
                sorted[sorted.len() / 2]
            };
            (
                Some(sorted.first().unwrap().to_string()),
                Some(sorted.last().unwrap().to_string()),
                Some(mean),
                Some(std),
                Some(median),
            )
        } else {
            let mut sorted_str: Vec<&str> = distinct_set.iter().map(|s| s.as_str()).collect();
            sorted_str.sort();
            (
                sorted_str.first().map(|s| s.to_string()),
                sorted_str.last().map(|s| s.to_string()),
                None,
                None,
                None,
            )
        };

        let mut top_values_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for val in &string_values {
            *top_values_map.entry(val.clone()).or_insert(0) += 1;
        }
        let mut top_values: Vec<(String, usize)> = top_values_map.into_iter().collect();
        top_values.sort_by(|a, b| b.1.cmp(&a.1));
        top_values.truncate(10);

        profiles.push(ColumnProfile {
            name: col_name,
            column_type,
            null_count,
            distinct_count,
            total_count,
            min_value,
            max_value,
            mean_value,
            std_value,
            median_value,
            top_values,
        });
    }

    profiles
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_connectors(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<crate::data::ConnectorInfo>, String> {
    Ok(state.data_connector_registry.list().await)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_scan_data_sources(
    uri: String,
    recursive: Option<bool>,
    max_depth: Option<usize>,
    extensions: Option<Vec<String>>,
    max_results: Option<usize>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<crate::data::DiscoveredItem>, String> {
    if uri.contains("..") || uri.contains('~') {
        return Err("URI contains invalid traversal sequence".to_string());
    }
    let options = crate::data::ScanOptions {
        recursive: recursive.unwrap_or(true),
        max_depth,
        extensions: extensions.unwrap_or_default(),
        exclude_patterns: vec![
            ".git".into(),
            "node_modules".into(),
            ".DS_Store".into(),
            "__pycache__".into(),
            ".venv".into(),
            "target".into(),
        ],
        max_results,
    };

    state.data_connector_registry.scan(&uri, &options).await.map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_test_data_connection(
    uri: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<bool, String> {
    if uri.contains("..") || uri.contains('~') {
        return Err("URI contains invalid traversal sequence".to_string());
    }
    state.data_connector_registry.test_connection(&uri).await.map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_resolve_data_item(
    item: crate::data::DiscoveredItem,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::ResolvedDataSource, String> {
    if item.path.contains("..") || item.path.contains('~') {
        return Err("Path contains invalid traversal sequence".to_string());
    }
    state.data_connector_registry.resolve_item(&item).await.map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_quick_register_dataset(
    item: crate::data::DiscoveredItem,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::dataset::aggregate::Dataset, String> {
    if item.path.contains("..") || item.path.contains('~') {
        return Err("Path contains invalid traversal sequence".to_string());
    }

    let resolved = state.data_connector_registry.resolve_item(&item).await
        .map_err(|e| format!("解析数据项 '{}' 失败: {}。请检查数据源连接是否正常", item.name, e))?;

    let data_format = resolved.format;

    let source = state.data_source_registry.find_by_id_str(&data_format.to_string())
        .await
        .ok_or_else(|| format!("Data source loader not found for format: {}", data_format))?;

    let config = DataLoadConfig {
        path: resolved.path.clone(),
        format: data_format,
        has_header: true,
        delimiter: None,
        encoding: None,
        max_rows: None,
        custom_params: resolved.load_config_params.clone(),
    };

    let info = source.load(&config).await
        .map_err(|e| format!("加载数据文件 '{}' 失败: {}。请检查文件格式是否正确、文件是否损坏", resolved.path, e))?;

    let file_content = std::fs::read(&resolved.path)
        .map_err(|e| format!("无法读取文件 '{}': {}。请检查文件路径是否正确、文件是否存在、是否有读取权限", resolved.path, e))?;
    let digest = Dataset::compute_digest(&file_content);
    let memory_size_mb = file_content.len() as f64 / (1024.0 * 1024.0);

    let column_profiles = if matches!(data_format, crate::types::DataFormat::Csv) {
        let content = std::fs::read_to_string(&resolved.path)
            .map_err(|e| format!("无法读取CSV文件 '{}': {}。请确认文件编码为UTF-8", resolved.path, e))?;
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(content.as_bytes());
        let headers = reader.headers()
            .map_err(|e| format!("CSV表头解析错误 (文件: {}): {}。请检查第一行是否为有效的列名", resolved.path, e))?
            .clone();
        let header_vec: Vec<String> = headers.iter().map(|h| h.to_string()).collect();
        let records: Vec<csv::StringRecord> = reader.records()
            .filter_map(|r| r.ok())
            .take(10000)
            .collect();
        compute_column_profiles(&header_vec, &records)
    } else {
        info.column_names.iter().zip(info.column_types.iter()).map(|(name, ct)| {
            ColumnProfile {
                name: name.clone(),
                column_type: ct.parse::<ColumnType>().unwrap_or(ColumnType::Unknown),
                null_count: 0,
                distinct_count: 0,
                total_count: info.rows,
                min_value: None,
                max_value: None,
                mean_value: None,
                std_value: None,
                median_value: None,
                top_values: Vec::new(),
            }
        }).collect()
    };

    state.dataset_handler.handle(DatasetCommand::RegisterDataset {
        name: resolved.name,
        format: data_format,
        path: resolved.path.clone(),
        digest: digest.clone(),
        rows: info.rows,
        columns: info.columns,
        column_profiles,
        memory_size_mb,
    }).await.map_err(|e| format!("注册数据集失败: {}。数据集可能已存在（重复注册）或数据库异常", e))?;

    let saved_dataset = state.dataset_repo.find_by_digest(&digest).await
        .map_err(|e| format!("查询已注册数据集失败: {}。数据集已注册但无法检索，请刷新列表", e))?
        .ok_or_else(|| "数据集已注册但无法检索到记录，请刷新数据集列表".to_string())?;

    let ds = &saved_dataset;
    state.event_bus.emit(LabEvent::DatasetRegistered {
        dataset_id: ds.id.to_string(),
        name: ds.name.clone(),
        format: data_format.to_string(),
        rows: ds.rows,
        columns: ds.columns,
    });

    Ok(saved_dataset)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_register_dataset(
    name: String,
    format: String,
    path: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::dataset::aggregate::Dataset, String> {
    if path.contains("..") || path.contains('~') {
        return Err("Path contains invalid traversal sequence".to_string());
    }

    let data_format = match format.as_str() {
        "csv" => crate::types::DataFormat::Csv,
        "json" => crate::types::DataFormat::Json,
        "image" => crate::types::DataFormat::Image,
        "text" => crate::types::DataFormat::Text,
        "binary" => crate::types::DataFormat::Binary,
        "parquet" => crate::types::DataFormat::Parquet,
        "excel" => crate::types::DataFormat::Excel,
        "tfrecord" => crate::types::DataFormat::TfRecord,
        "huggingface" => crate::types::DataFormat::HuggingFace,
        "database" => crate::types::DataFormat::Database,
        _ => crate::types::DataFormat::Csv,
    };

    let file_content = std::fs::read(&path)
        .map_err(|e| format!("无法读取文件 '{}': {}。请检查文件路径是否正确、文件是否存在、是否有读取权限", path, e))?;
    let digest = Dataset::compute_digest(&file_content);
    let memory_size_mb = file_content.len() as f64 / (1024.0 * 1024.0);

    let source = state.data_source_registry.find_by_id_str(&format)
        .await
        .ok_or_else(|| format!("不支持的数据格式 '{}'。支持的格式: csv, json, parquet, excel, text, image, binary, tfrecord, huggingface, database", format))?;

    let config = crate::core::config::DataLoadConfig {
        path: path.clone(),
        format: data_format,
        has_header: true,
        delimiter: None,
        encoding: None,
        max_rows: None,
        custom_params: std::collections::HashMap::new(),
    };

    let info = source.load(&config).await
        .map_err(|e| format!("加载数据集文件 '{}' 失败: {}。请检查文件格式是否正确、文件是否损坏", path, e))?;

    let column_profiles = if format == "csv" {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("无法读取CSV文件 '{}': {}。请确认文件编码为UTF-8", path, e))?;
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(content.as_bytes());
        let headers = reader.headers()
            .map_err(|e| format!("CSV表头解析错误 (文件: {}): {}。请检查第一行是否为有效的列名", path, e))?
            .clone();
        let header_vec: Vec<String> = headers.iter().map(|h| h.to_string()).collect();
        let records: Vec<csv::StringRecord> = reader.records()
            .filter_map(|r| r.ok())
            .take(10000)
            .collect();
        compute_column_profiles(&header_vec, &records)
    } else {
        info.column_names.iter().zip(info.column_types.iter()).map(|(name, ct)| {
            ColumnProfile {
                name: name.clone(),
                column_type: ct.parse::<ColumnType>().unwrap_or(ColumnType::Unknown),
                null_count: 0,
                distinct_count: 0,
                total_count: info.rows,
                min_value: None,
                max_value: None,
                mean_value: None,
                std_value: None,
                median_value: None,
                top_values: Vec::new(),
            }
        }).collect()
    };

    state.dataset_handler.handle(DatasetCommand::RegisterDataset {
        name,
        format: data_format,
        path,
        digest: digest.clone(),
        rows: info.rows,
        columns: info.columns,
        column_profiles,
        memory_size_mb,
    }).await.map_err(|e| format!("注册数据集失败: {}。数据集可能已存在（重复注册）或数据库异常", e))?;

    let saved_dataset = state.dataset_repo.find_by_digest(&digest).await
        .map_err(|e| format!("查询已注册数据集失败: {}。数据集已注册但无法检索，请刷新列表", e))?
        .ok_or_else(|| "数据集已注册但无法检索到记录，请刷新数据集列表".to_string())?;

    let ds = &saved_dataset;
    state.event_bus.emit(LabEvent::DatasetRegistered {
        dataset_id: ds.id.to_string(),
        name: ds.name.clone(),
        format: format.clone(),
        rows: ds.rows,
        columns: ds.columns,
    });

    Ok(saved_dataset)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_datasets(
    status_filter: Option<String>,
    format_filter: Option<String>,
    name_contains: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<crate::domain::dataset::aggregate::DatasetSummary>, String> {
    let filter = DatasetFilter {
        status: status_filter.and_then(|s| s.parse().ok()),
        format: format_filter.and_then(|f| match f.as_str() {
            "csv" => Some(crate::types::DataFormat::Csv),
            "json" => Some(crate::types::DataFormat::Json),
            "image" => Some(crate::types::DataFormat::Image),
            "text" => Some(crate::types::DataFormat::Text),
            "binary" => Some(crate::types::DataFormat::Binary),
            "parquet" => Some(crate::types::DataFormat::Parquet),
            "excel" => Some(crate::types::DataFormat::Excel),
            "tfrecord" => Some(crate::types::DataFormat::TfRecord),
            "huggingface" => Some(crate::types::DataFormat::HuggingFace),
            "database" => Some(crate::types::DataFormat::Database),
            _ => None,
        }),
        name_contains,
        ..DatasetFilter::default()
    };

    state.dataset_repo.list(&filter).await
        .map_err(|e| format!("查询数据集列表失败: {}。请稍后重试，如持续失败请检查数据库状态", e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_get_dataset(
    dataset_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::dataset::aggregate::Dataset, String> {
    let id = DatasetId::from_str(&dataset_id);
    state.dataset_repo.load(&id).await
        .map_err(|e| format!("加载数据集 '{}' 失败: {}。数据集可能已被删除或数据库异常", dataset_id, e))?
        .ok_or_else(|| format!("数据集 '{}' 不存在。可能已被删除或ID不正确", dataset_id))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_delete_dataset(
    dataset_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.dataset_handler.handle(DatasetCommand::DeleteDataset {
        dataset_id: DatasetId::from_str(&dataset_id),
    }).await.map_err(|e| format!("删除数据集 '{}' 失败: {}。请确认数据集未被实验引用", dataset_id, e))?;

    state.event_bus.emit(LabEvent::DatasetDeleted {
        dataset_id: dataset_id.clone(),
    });

    Ok(())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_archive_dataset(
    dataset_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.dataset_handler.handle(DatasetCommand::ArchiveDataset {
        dataset_id: DatasetId::from_str(&dataset_id),
    }).await.map_err(|e| format!("归档数据集 '{}' 失败: {}。请确认数据集状态为活跃", dataset_id, e))?;

    state.event_bus.emit(LabEvent::DatasetArchived {
        dataset_id: dataset_id.clone(),
    });

    Ok(())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_restore_dataset(
    dataset_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.dataset_handler.handle(DatasetCommand::RestoreDataset {
        dataset_id: DatasetId::from_str(&dataset_id),
    }).await.map_err(|e| format!("恢复数据集 '{}' 失败: {}。请确认数据集状态为已归档", dataset_id, e))?;

    state.event_bus.emit(LabEvent::DatasetRestored {
        dataset_id: dataset_id.clone(),
    });

    Ok(())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_add_tag(
    dataset_id: String,
    tag: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.dataset_handler.handle(DatasetCommand::AddTag {
        dataset_id: DatasetId::from_str(&dataset_id),
        tag,
    }).await.map_err(|e| format!("为数据集 '{}' 添加标签失败: {}。请确认数据集存在", dataset_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_remove_tag(
    dataset_id: String,
    tag: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.dataset_handler.handle(DatasetCommand::RemoveTag {
        dataset_id: DatasetId::from_str(&dataset_id),
        tag,
    }).await.map_err(|e| format!("为数据集 '{}' 移除标签失败: {}。请确认标签存在", dataset_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_set_description(
    dataset_id: String,
    description: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.dataset_handler.handle(DatasetCommand::SetDescription {
        dataset_id: DatasetId::from_str(&dataset_id),
        description,
    }).await.map_err(|e| format!("为数据集 '{}' 设置描述失败: {}。请确认数据集存在", dataset_id, e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_link_experiment(
    dataset_id: String,
    experiment_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let ds_id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&ds_id).await
        .map_err(|e| format!("加载数据集 '{}' 失败: {}。数据集可能已被删除", dataset_id, e))?
        .ok_or_else(|| format!("数据集 '{}' 不存在，无法关联实验", dataset_id))?;

    let exp_id = ExperimentId::from_str(&experiment_id);
    let mut experiment = state.experiment_repo.load(&exp_id).await
        .map_err(|e| format!("加载实验 '{}' 失败: {}。实验可能已被删除", experiment_id, e))?
        .ok_or_else(|| format!("实验 '{}' 不存在，无法关联数据集", experiment_id))?;

    experiment.link_dataset(dataset_id.clone(), dataset.version.to_string());
    if let Err(e) = state.experiment_repo.save(&experiment).await {
        return Err(format!("保存实验关联失败: {}。请稍后重试", e));
    }

    if let Err(e) = state.dataset_handler.handle(DatasetCommand::LinkExperiment {
        dataset_id: ds_id,
        experiment_id,
    }).await {
        if let Ok(Some(mut exp)) = state.experiment_repo.load(&exp_id).await.map(|o| o) {
            exp.dataset_id = None;
            exp.dataset_version = None;
            let _ = state.experiment_repo.save(&exp).await;
        }
        return Err(format!("关联数据集到实验失败: {}。请确认数据集和实验都存在", e));
    }

    Ok(())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_new_version(
    dataset_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::dataset::aggregate::Dataset, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| format!("加载数据集 '{}' 失败: {}。数据集可能已被删除", dataset_id, e))?
        .ok_or_else(|| format!("数据集 '{}' 不存在，无法创建新版本", dataset_id))?;

    if dataset.status != crate::domain::dataset::aggregate::DatasetStatus::Active {
        return Err(format!("无法为状态为 '{}' 的数据集创建新版本，只有活跃状态的数据集才能创建版本。请先恢复数据集", dataset.status));
    }

    let file_content = std::fs::read(&dataset.path)
        .map_err(|e| format!("无法读取数据集文件 '{}': {}。请检查文件是否存在、是否有读取权限", dataset.path, e))?;
    let new_digest = Dataset::compute_digest(&file_content);
    let new_size_mb = file_content.len() as f64 / (1024.0 * 1024.0);

    let source = state.data_source_registry.find_by_id_str(&dataset.format.to_string())
        .await
        .ok_or_else(|| format!("不支持的数据格式 '{}'。请确认数据源加载器已注册", dataset.format))?;

    let config = crate::core::config::DataLoadConfig {
        path: dataset.path.clone(),
        format: dataset.format,
        has_header: true,
        delimiter: None,
        encoding: None,
        max_rows: None,
        custom_params: std::collections::HashMap::new(),
    };

    let info = source.load(&config).await
        .map_err(|e| format!("加载数据集文件 '{}' 失败: {}。请检查文件格式是否正确、文件是否损坏", dataset.path, e))?;

    let new_profiles = if dataset.format == crate::types::DataFormat::Csv {
        let content = std::fs::read_to_string(&dataset.path)
            .map_err(|e| format!("无法读取CSV文件 '{}': {}。请确认文件编码为UTF-8", dataset.path, e))?;
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(content.as_bytes());
        let headers = reader.headers()
            .map_err(|e| format!("CSV表头解析错误 (文件: {}): {}。请检查第一行是否为有效的列名", dataset.path, e))?
            .clone();
        let header_vec: Vec<String> = headers.iter().map(|h| h.to_string()).collect();
        let records: Vec<csv::StringRecord> = reader.records()
            .filter_map(|r| r.ok())
            .take(10000)
            .collect();
        compute_column_profiles(&header_vec, &records)
    } else {
        dataset.column_profiles.clone()
    };

    state.dataset_handler.handle(DatasetCommand::NewVersion {
        dataset_id: id,
        new_digest,
        new_rows: info.rows,
        new_columns: info.columns,
        new_profiles,
        new_size_mb,
    }).await.map_err(|e| e.to_string())?;

    state.dataset_repo.load(&DatasetId::from_str(&dataset_id)).await
        .map_err(|e| format!("加载更新后的数据集 '{}' 失败: {}。版本已创建但无法检索", dataset_id, e))?
        .ok_or_else(|| format!("数据集 '{}' 版本更新后无法检索到记录", dataset_id))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_version_history(
    dataset_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<crate::domain::dataset::DatasetVersionRecord>, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| format!("加载数据集 '{}' 版本历史失败: {}。数据集可能已被删除", dataset_id, e))?
        .ok_or_else(|| format!("数据集 '{}' 不存在，无法查看版本历史", dataset_id))?;
    Ok(dataset.version_history)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_create_dataset_split(
    dataset_id: String,
    name: String,
    strategy: String,
    train_ratio: f64,
    val_ratio: f64,
    test_ratio: f64,
    seed: u64,
    stratify_column: Option<String>,
    group_column: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::dataset::aggregate::DatasetSplit, String> {
    if name.trim().is_empty() {
        return Err("Split name cannot be empty".to_string());
    }
    let split_strategy = strategy.parse::<crate::domain::dataset::aggregate::SplitStrategy>()
        .map_err(|e| e.to_string())?;

    let ds_id = DatasetId::from_str(&dataset_id);
    let existing = state.dataset_repo.load_split(&ds_id, &name).await
        .map_err(|e| e.to_string())?;
    if existing.is_some() {
        return Err(format!("Split '{}' already exists for this dataset", name));
    }

    state.dataset_handler.handle(DatasetCommand::CreateSplit {
        dataset_id: ds_id.clone(),
        name: name.clone(),
        strategy: split_strategy,
        train_ratio,
        val_ratio,
        test_ratio,
        seed,
        stratify_column,
        group_column,
    }).await.map_err(|e| e.to_string())?;

    state.dataset_repo.load_split(&ds_id, &name).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Split not found after creation".to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_dataset_splits(
    dataset_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<crate::domain::dataset::aggregate::DatasetSplit>, String> {
    let ds_id = DatasetId::from_str(&dataset_id);
    state.dataset_repo.load_splits(&ds_id).await
        .map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_get_dataset_split(
    dataset_id: String,
    name: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::dataset::aggregate::DatasetSplit, String> {
    let ds_id = DatasetId::from_str(&dataset_id);
    state.dataset_repo.load_split(&ds_id, &name).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Split '{}' not found", name))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_delete_dataset_split(
    dataset_id: String,
    name: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.dataset_handler.handle(DatasetCommand::DeleteSplit {
        dataset_id: DatasetId::from_str(&dataset_id),
        name,
    }).await.map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_validate_dataset(
    dataset_id: String,
    expectations: Vec<crate::domain::dataset::aggregate::DataExpectation>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::dataset::aggregate::DataQualityReport, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    Ok(crate::domain::dataset::quality::QualityEngine::validate(&dataset, &expectations))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_auto_validate_dataset(
    dataset_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::dataset::aggregate::DataQualityReport, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    let expectations = crate::domain::dataset::quality::QualityEngine::auto_generate_expectations(&dataset);
    Ok(crate::domain::dataset::quality::QualityEngine::validate(&dataset, &expectations))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_version_diff(
    dataset_id: String,
    from_version: String,
    to_version: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::dataset::aggregate::DatasetVersionDiff, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    let from_record = dataset.version_history.iter()
        .find(|v| v.version == from_version)
        .ok_or_else(|| format!("Version '{}' not found in history", from_version))?;

    let to_record = dataset.version_history.iter()
        .find(|v| v.version == to_version)
        .ok_or_else(|| format!("Version '{}' not found in history", to_version))?;

    Ok(crate::domain::dataset::aggregate::DatasetVersionDiff::new(
        from_version,
        to_version,
        from_record.rows,
        to_record.rows,
        &from_record.column_profiles,
        &to_record.column_profiles,
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_new_version_with_note(
    dataset_id: String,
    change_note: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::domain::dataset::aggregate::Dataset, String> {
    let id = DatasetId::from_str(&dataset_id);
    let mut dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    if dataset.status != crate::domain::dataset::aggregate::DatasetStatus::Active {
        return Err(format!("Cannot create new version for dataset in {} status, only Active datasets can be versioned", dataset.status));
    }

    let file_content = std::fs::read(&dataset.path)
        .map_err(|e| format!("Cannot read file: {}", e))?;
    let new_digest = Dataset::compute_digest(&file_content);
    let new_size_mb = file_content.len() as f64 / (1024.0 * 1024.0);

    let source = state.data_source_registry.find_by_id_str(&dataset.format.to_string())
        .await
        .ok_or_else(|| format!("Data source not found: {}", dataset.format))?;

    let config = crate::core::config::DataLoadConfig {
        path: dataset.path.clone(),
        format: dataset.format,
        has_header: true,
        delimiter: None,
        encoding: None,
        max_rows: None,
        custom_params: std::collections::HashMap::new(),
    };

    let info = source.load(&config).await.map_err(|e| e.to_string())?;

    let new_profiles = if dataset.format == crate::types::DataFormat::Csv {
        let content = std::fs::read_to_string(&dataset.path)
            .map_err(|e| format!("Cannot read CSV: {}", e))?;
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(content.as_bytes());
        let headers = reader.headers()
            .map_err(|e| format!("CSV header error: {}", e))?
            .clone();
        let header_vec: Vec<String> = headers.iter().map(|h| h.to_string()).collect();
        let records: Vec<csv::StringRecord> = reader.records()
            .filter_map(|r| r.ok())
            .take(10000)
            .collect();
        compute_column_profiles(&header_vec, &records)
    } else {
        dataset.column_profiles.clone()
    };

    dataset.new_version_with_note(new_digest, info.rows, info.columns, new_profiles, new_size_mb, change_note)
        .map_err(|e| e.to_string())?;
    state.dataset_repo.save(&dataset).await.map_err(|e| e.to_string())?;

    state.dataset_repo.load(&DatasetId::from_str(&dataset_id)).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found after update".to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_set_card(
    dataset_id: String,
    card: crate::domain::dataset::aggregate::DatasetCard,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.dataset_handler.handle(
        crate::domain::dataset::handler::DatasetCommand::SetCard {
            dataset_id: DatasetId::from_str(&dataset_id),
            card,
        },
    ).await.map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_get_card(
    dataset_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Option<crate::domain::dataset::aggregate::DatasetCard>, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;
    Ok(dataset.card)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_analyze_imbalance(
    dataset_id: String,
    column_name: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::analysis::ClassImbalanceReport, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    let splits = state.dataset_repo.load_splits(&id).await
        .map_err(|e| e.to_string())?;

    Ok(crate::data::analysis::ImbalanceAnalyzer::analyze(&dataset, &column_name, &splits))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_analyze_drift(
    dataset_id: String,
    from_version: String,
    to_version: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::analysis::DataDriftReport, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    let from = dataset.version_history.iter().find(|v| v.version == from_version)
        .ok_or_else(|| format!("Version {} not found", from_version))?;
    let to = dataset.version_history.iter().find(|v| v.version == to_version)
        .ok_or_else(|| format!("Version {} not found", to_version))?;

    Ok(crate::data::analysis::DataDriftAnalyzer::analyze(from, to))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_analyze_correlation(
    dataset_id: String,
    target_column: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::analysis::FeatureCorrelationReport, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    Ok(crate::data::analysis::FeatureCorrelationAnalyzer::analyze(
        &dataset.id.to_string(),
        &dataset.column_profiles,
        target_column.as_deref(),
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_check_leakage(
    dataset_id: String,
    split_name: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::analysis::LeakageReport, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    let split = state.dataset_repo.load_split(&id, &split_name).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Split '{}' not found", split_name))?;

    Ok(crate::data::analysis::SafetyAnalyzer::detect_split_leakage(&dataset, &split))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_data_loader_create(
    dataset_path: String,
    batch_size: usize,
    num_workers: usize,
    drop_last: Option<bool>,
    pin_memory: Option<bool>,
    prefetch_factor: Option<usize>,
) -> Result<String, String> {
    use crate::data::data_loader::DataLoaderConfig;

    let config = DataLoaderConfig {
        batch_size,
        num_workers,
        drop_last: drop_last.unwrap_or(true),
        pin_memory: pin_memory.unwrap_or(false),
        prefetch_factor: prefetch_factor.unwrap_or(2),
        ..Default::default()
    };

    Ok(serde_json::to_string(&serde_json::json!({
        "dataset_path": dataset_path,
        "config": {
            "batch_size": config.batch_size,
            "num_workers": config.num_workers,
            "drop_last": config.drop_last,
            "pin_memory": config.pin_memory,
            "prefetch_factor": config.prefetch_factor,
        }
    })).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_global_shuffle_create(
    total_samples: usize,
    num_workers: usize,
    worker_rank: usize,
    seed: Option<u64>,
    shuffle_across_epochs: Option<bool>,
    drop_last: Option<bool>,
) -> Result<String, String> {
    use crate::data::global_shuffle::{GlobalShuffleSampler, GlobalShuffleConfig};

    let config = GlobalShuffleConfig {
        num_workers,
        worker_rank,
        seed: seed.unwrap_or(42),
        shuffle_across_epochs: shuffle_across_epochs.unwrap_or(true),
        drop_last: drop_last.unwrap_or(true),
        buffer_size: 10000,
        elastic: false,
    };

    let sampler = GlobalShuffleSampler::new(total_samples, config);
    let indices: Vec<usize> = sampler.iter().collect();
    Ok(serde_json::to_string(&indices).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_cloud_streaming_open(
    name: String,
    key: String,
    provider: String,
    bucket: Option<String>,
    region: Option<String>,
    endpoint: Option<String>,
    access_key: Option<String>,
    secret_key: Option<String>,
    chunk_size: Option<usize>,
) -> Result<String, String> {
    use crate::data::cloud_streaming::{CloudStorageConfig, CloudStreamingDataset};
    use crate::data::streaming::StreamingConfig;

    let storage_config = CloudStorageConfig {
        provider: match provider.as_str() {
            "s3" => crate::data::cloud_streaming::CloudProvider::S3,
            "gcs" => crate::data::cloud_streaming::CloudProvider::GCS,
            "oss" => crate::data::cloud_streaming::CloudProvider::OSS,
            "minio" => crate::data::cloud_streaming::CloudProvider::MinIO,
            _ => return Err(format!("Unknown provider: {}", provider)),
        },
        bucket: bucket.unwrap_or_default(),
        region: region.unwrap_or_else(|| "us-east-1".to_string()),
        endpoint,
        access_key,
        secret_key,
        ..Default::default()
    };

    let streaming_config = StreamingConfig {
        chunk_size: chunk_size.unwrap_or(65536),
        ..Default::default()
    };

    let dataset = CloudStreamingDataset::open_csv(
        &name,
        &key,
        storage_config,
        streaming_config,
        None,
    )?;

    Ok(serde_json::to_string(&dataset.info).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_data_version_init(
    repo_path: String,
) -> Result<String, String> {
    use crate::data::data_version::{DataVersionConfig, DataVersionRepo};

    let config = DataVersionConfig {
        repo_path: std::path::PathBuf::from(&repo_path),
        ..Default::default()
    };

    let repo = DataVersionRepo::init(config)?;
    Ok(format!("Repository initialized at {}", repo_path))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_data_version_commit(
    repo_path: String,
    dataset_name: String,
    files_json: String,
    message: String,
    author: String,
    tags_json: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    use crate::data::data_version::{DataVersionConfig, DataVersionRepo};

    let task_id = format!("version_commit_{}", dataset_name);

    let config = DataVersionConfig {
        repo_path: std::path::PathBuf::from(&repo_path),
        ..Default::default()
    };

    let mut repo = DataVersionRepo::open(config)?;

    let files: Vec<std::path::PathBuf> = serde_json::from_str(&files_json)
        .map_err(|e| format!("Invalid files JSON: {}", e))?;

    let tags: Vec<String> = tags_json
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();

    let hash = repo.commit(&dataset_name, &files, &message, &author, tags)?;

    state.event_bus.emit(LabEvent::OperationCompleted {
        task_id,
        operation: "version_commit".to_string(),
        result: serde_json::json!({
            "dataset_name": dataset_name,
            "commit_hash": hash.as_str(),
            "message": message,
            "files_count": files.len(),
        }),
    });

    Ok(hash.as_str().to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_data_version_log(
    repo_path: String,
    max_count: Option<usize>,
) -> Result<String, String> {
    use crate::data::data_version::{DataVersionConfig, DataVersionRepo};

    let config = DataVersionConfig {
        repo_path: std::path::PathBuf::from(&repo_path),
        ..Default::default()
    };

    let repo = DataVersionRepo::open(config)?;
    let log = repo.log(max_count);
    Ok(serde_json::to_string(&log).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_data_version_checkout(
    repo_path: String,
    hash: String,
) -> Result<(), String> {
    use crate::data::data_version::{ContentHash, DataVersionConfig, DataVersionRepo};

    let config = DataVersionConfig {
        repo_path: std::path::PathBuf::from(&repo_path),
        ..Default::default()
    };

    let mut repo = DataVersionRepo::open(config)?;
    let content_hash: ContentHash = hash.parse()?;
    repo.checkout(&content_hash)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_data_version_diff(
    repo_path: String,
    from_hash: String,
    to_hash: String,
) -> Result<String, String> {
    use crate::data::data_version::{ContentHash, DataVersionConfig, DataVersionRepo};

    let config = DataVersionConfig {
        repo_path: std::path::PathBuf::from(&repo_path),
        ..Default::default()
    };

    let repo = DataVersionRepo::open(config)?;
    let from: ContentHash = from_hash.parse()?;
    let to: ContentHash = to_hash.parse()?;
    let diff = repo.diff(&from, &to)?;
    Ok(serde_json::to_string(&diff).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_data_version_branches(
    repo_path: String,
) -> Result<String, String> {
    use crate::data::data_version::{DataVersionConfig, DataVersionRepo};

    let config = DataVersionConfig {
        repo_path: std::path::PathBuf::from(&repo_path),
        ..Default::default()
    };

    let repo = DataVersionRepo::open(config)?;
    let branches = repo.branches();
    Ok(serde_json::to_string(&branches).unwrap_or_default())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_data_version_create_branch(
    repo_path: String,
    branch_name: String,
    description: Option<String>,
) -> Result<(), String> {
    use crate::data::data_version::{DataVersionConfig, DataVersionRepo};

    let config = DataVersionConfig {
        repo_path: std::path::PathBuf::from(&repo_path),
        ..Default::default()
    };

    let mut repo = DataVersionRepo::open(config)?;
    repo.create_branch(&branch_name, description.as_deref())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_check_feature_leakage(
    dataset_id: String,
    target_column: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::analysis::FeatureLeakageReport, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    Ok(crate::data::analysis::SafetyAnalyzer::detect_feature_leakage(&dataset, &target_column))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_check_sufficiency(
    dataset_id: String,
    model_type: String,
    estimated_params: usize,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::analysis::DataSufficiencyReport, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    Ok(crate::data::analysis::SafetyAnalyzer::assess_data_sufficiency(
        &dataset, &model_type, estimated_params,
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_check_split_consistency(
    dataset_id: String,
    split_name: String,
    target_column: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::analysis::SplitConsistencyReport, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    let split = state.dataset_repo.load_split(&id, &split_name).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Split '{}' not found", split_name))?;

    Ok(crate::data::analysis::SafetyAnalyzer::check_split_consistency(&dataset, &split, &target_column))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_readiness_score(
    dataset_id: String,
    split_name: Option<String>,
    target_column: Option<String>,
    model_type: Option<String>,
    estimated_params: Option<usize>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::analysis::DataReadinessScore, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    let leakage_report = if let Some(ref sn) = split_name {
        let split = state.dataset_repo.load_split(&id, sn).await
            .map_err(|e| e.to_string())?;
        split.map(|s| crate::data::analysis::SafetyAnalyzer::detect_split_leakage(&dataset, &s))
    } else {
        None
    };

    let feature_leakage_report = if let Some(ref tc) = target_column {
        Some(crate::data::analysis::SafetyAnalyzer::detect_feature_leakage(&dataset, tc))
    } else {
        None
    };

    let sufficiency_report = if let (Some(ref mt), Some(ep)) = (model_type, estimated_params) {
        Some(crate::data::analysis::SafetyAnalyzer::assess_data_sufficiency(&dataset, mt, ep))
    } else {
        None
    };

    let consistency_report = if let (Some(ref sn), Some(ref tc)) = (split_name, target_column) {
        let split = state.dataset_repo.load_split(&id, sn).await
            .map_err(|e| e.to_string())?;
        split.map(|s| crate::data::analysis::SafetyAnalyzer::check_split_consistency(&dataset, &s, tc))
    } else {
        None
    };

    Ok(crate::data::analysis::SafetyAnalyzer::compute_readiness_score(
        &dataset,
        leakage_report.as_ref(),
        feature_leakage_report.as_ref(),
        sufficiency_report.as_ref(),
        consistency_report.as_ref(),
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_create_kfold(
    dataset_id: String,
    k: usize,
    strategy: String,
    seed: u64,
    stratify_column: Option<String>,
    group_column: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::analysis::KFoldResult, String> {
    if k < 2 {
        return Err("K must be at least 2".to_string());
    }
    if k > 20 {
        return Err("K must be at most 20".to_string());
    }

    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    let kfold_strategy = strategy.parse::<crate::data::analysis::KFoldStrategy>()
        .map_err(|e| e.to_string())?;

    let result = match kfold_strategy {
        crate::data::analysis::KFoldStrategy::Standard => {
            crate::data::analysis::KFoldSplitter::create_standard(
                &dataset_id, k, dataset.rows, seed,
            )
        }
        crate::data::analysis::KFoldStrategy::Stratified => {
            let col_name = stratify_column.as_deref()
                .ok_or_else(|| "stratify_column is required for stratified K-Fold".to_string())?;
            let col_values = crate::domain::dataset::handler::DefaultDatasetCommandHandler::get_column_values(&dataset, col_name)
                .map_err(|e| e.to_string())?;
            crate::data::analysis::KFoldSplitter::create_stratified(
                &dataset_id, k, dataset.rows, seed, &col_values,
            )
        }
        crate::data::analysis::KFoldStrategy::Group => {
            let col_name = group_column.as_deref()
                .ok_or_else(|| "group_column is required for group K-Fold".to_string())?;
            let col_values = crate::domain::dataset::handler::DefaultDatasetCommandHandler::get_column_values(&dataset, col_name)
                .map_err(|e| e.to_string())?;
            crate::data::analysis::KFoldSplitter::create_group(
                &dataset_id, k, dataset.rows, seed, &col_values,
            )
        }
    };

    for fold in &result.folds {
        state.dataset_repo.save_split(&id, &fold.split).await
            .map_err(|e| e.to_string())?;
    }

    Ok(result)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_row_diff(
    dataset_id: String,
    from_version: String,
    to_version: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::analysis::RowDiffReport, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    let accessor = state.data_accessor_registry.get(&dataset.format).await
        .ok_or_else(|| format!("No data accessor for format: {}", dataset.format))?;

    let row_count = accessor.row_count(&dataset.path).await
        .map_err(|e| e.to_string())?;

    let data_page = accessor.page(&dataset.path, 0, row_count).await
        .map_err(|e| e.to_string())?;

    let rows_as_strings: Vec<Vec<String>> = data_page.rows.iter()
        .map(|row| row.iter()
            .map(|v| match v {
                serde_json::Value::Null => String::new(),
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect())
        .collect();

    let column_names: Vec<String> = dataset.column_profiles.iter()
        .map(|p| p.name.clone())
        .collect();

    Ok(crate::data::analysis::RowDiffAnalyzer::diff(
        &dataset_id, &from_version, &to_version,
        &rows_as_strings, &rows_as_strings, &column_names,
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_list_augmentation_presets(
    target: Option<String>,
) -> Result<Vec<crate::data::augmentation::AugmentationPreset>, String> {
    if let Some(t) = target {
        let aug_target = t.parse::<crate::data::augmentation::AugmentationTarget>()
            .map_err(|e| e.to_string())?;
        Ok(crate::data::augmentation::AugmentationPresets::presets_for_target(aug_target))
    } else {
        Ok(crate::data::augmentation::AugmentationPresets::all_presets())
    }
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_lazy_inspect(
    path: String,
    chunk_size: Option<usize>,
) -> Result<crate::data::lazy_loader::LazyDatasetInfo, String> {
    let config = crate::data::lazy_loader::LazyLoadConfig {
        chunk_size: chunk_size.unwrap_or(10000),
        ..Default::default()
    };
    crate::data::lazy_loader::LazyDataLoader::inspect(&path, &config)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_lazy_read_chunk(
    path: String,
    chunk_index: usize,
    chunk_size: Option<usize>,
) -> Result<crate::data::lazy_loader::DataChunk, String> {
    let config = crate::data::lazy_loader::LazyLoadConfig {
        chunk_size: chunk_size.unwrap_or(10000),
        ..Default::default()
    };

    let extension = std::path::Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "csv" => crate::data::lazy_loader::LazyDataLoader::read_chunk_csv(&path, chunk_index, &config),
        "parquet" => crate::data::lazy_loader::LazyDataLoader::read_chunk_parquet(&path, chunk_index, &config),
        _ => Err(format!("Unsupported format for lazy loading: {}", extension)),
    }
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_recommend_chunk_size(
    file_size_mb: f64,
    max_memory_mb: Option<f64>,
) -> Result<usize, String> {
    Ok(crate::data::lazy_loader::LazyDataLoader::recommend_chunk_size(
        file_size_mb,
        max_memory_mb.unwrap_or(512.0),
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_pipeline_create_standard_dag(
    dataset_name: String,
    model_name: String,
) -> Result<crate::data::pipeline_dag::PipelineDag, String> {
    crate::data::pipeline_dag::create_standard_training_dag(&dataset_name, &model_name)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_pipeline_build_dag(
    name: String,
    nodes: Vec<crate::data::pipeline_dag::DagNode>,
    edges: Vec<crate::data::pipeline_dag::DagEdge>,
) -> Result<crate::data::pipeline_dag::PipelineDag, String> {
    let mut builder = crate::data::pipeline_dag::PipelineDagBuilder::new(&name);
    for node in &nodes {
        builder = builder.add_node(
            &node.id, &node.name, node.node_type,
            node.input_artifacts.clone(), node.output_artifacts.clone(),
        );
    }
    for edge in &edges {
        builder = builder.add_edge(&edge.from, &edge.to, &edge.artifact, edge.edge_type);
    }
    builder.build()
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_pipeline_detect_changes(
    dag: crate::data::pipeline_dag::PipelineDag,
    node_digests: std::collections::HashMap<String, String>,
) -> Result<Vec<crate::data::pipeline_dag::ChangeDetectionResult>, String> {
    Ok(dag.detect_changes(&node_digests))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_pipeline_plan_execution(
    dag: crate::data::pipeline_dag::PipelineDag,
) -> Result<crate::data::pipeline_dag::ExecutionPlan, String> {
    Ok(dag.plan_execution())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_pipeline_to_mermaid(
    dag: crate::data::pipeline_dag::PipelineDag,
) -> Result<String, String> {
    Ok(dag.to_mermaid())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_pipeline_validate(
    dag: crate::data::pipeline_dag::PipelineDag,
) -> Result<(), String> {
    dag.validate().map_err(|errors| errors.join("\n"))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_lineage_create_training(
    raw_data_id: String,
    raw_data_name: String,
    dataset_id: String,
    dataset_name: String,
    split_id: String,
    experiment_id: String,
    experiment_name: String,
    model_id: String,
    model_name: String,
    raw_digest: String,
    dataset_digest: String,
) -> Result<crate::data::lineage::LineageGraph, String> {
    crate::data::lineage::create_training_lineage(
        &raw_data_id, &raw_data_name,
        &dataset_id, &dataset_name,
        &split_id,
        &experiment_id, &experiment_name,
        &model_id, &model_name,
        &raw_digest, &dataset_digest,
    )
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_lineage_build(
    nodes: Vec<crate::data::lineage::LineageNode>,
    edges: Vec<crate::data::lineage::LineageEdge>,
) -> Result<crate::data::lineage::LineageGraph, String> {
    let mut tracker = crate::data::lineage::LineageTracker::new();
    for node in &nodes {
        tracker.add_node(&node.id, &node.name, node.node_type, &node.version, node.digest.as_deref());
    }
    for edge in &edges {
        tracker.add_edge(&edge.from, &edge.to, edge.relation, edge.transform.as_deref(), edge.params.clone());
    }
    tracker.build()
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_lineage_trace(
    graph: crate::data::lineage::LineageGraph,
    node_id: String,
) -> Result<crate::data::lineage::LineageTrace, String> {
    Ok(graph.trace_upstream(&node_id))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_lineage_impact(
    graph: crate::data::lineage::LineageGraph,
    node_id: String,
) -> Result<crate::data::lineage::ImpactAnalysis, String> {
    Ok(graph.analyze_impact(&node_id))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_lineage_reproducibility(
    graph: crate::data::lineage::LineageGraph,
    experiment_id: String,
    available_data: Vec<String>,
    available_code: bool,
    available_env: bool,
) -> Result<crate::data::lineage::ReproducibilityReport, String> {
    let data_set: std::collections::HashSet<String> = available_data.into_iter().collect();
    Ok(graph.check_reproducibility(&experiment_id, &data_set, available_code, available_env))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_lineage_to_mermaid(
    graph: crate::data::lineage::LineageGraph,
) -> Result<String, String> {
    Ok(graph.to_mermaid())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_lineage_graph(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::lineage::LineageGraph, String> {
    use crate::data::lineage::{LineageNodeType, LineageRelation};

    let mut tracker = crate::data::lineage::LineageTracker::new();

    let datasets = state.dataset_repo.list(&crate::domain::dataset::aggregate::DatasetFilter::default())
        .await.map_err(|e| e.to_string())?;
    for ds in &datasets {
        tracker.add_node(
            &ds.id.to_string(),
            &ds.name,
            LineageNodeType::Dataset,
            &ds.version,
            None,
        );
    }

    let experiments = state.experiment_repo.list(&crate::domain::experiment::ExperimentFilter::default())
        .await.map_err(|e| e.to_string())?;
    for exp in &experiments {
        tracker.add_node(
            &exp.id.to_string(),
            &exp.name,
            LineageNodeType::Experiment,
            "v1",
            None,
        );
        if let Some(ref ds_id) = exp.dataset_id {
            tracker.add_edge(
                &exp.id.to_string(),
                ds_id,
                LineageRelation::TrainedOn,
                Some("training"),
                None,
            );
        }
    }

    let models = state.model_repo.list(None).await.map_err(|e| e.to_string())?;
    for model in &models {
        tracker.add_node(
            &model.id.to_string(),
            &model.name,
            LineageNodeType::Model,
            &model.version,
            None,
        );
        if let Some(ref lineage) = model.lineage {
            if let Some(ref exp_id) = lineage.experiment_id {
                tracker.add_edge(
                    &model.id.to_string(),
                    exp_id,
                    LineageRelation::DerivedFrom,
                    Some("checkpoint"),
                    None,
                );
            }
            for dl in &lineage.datasets {
                tracker.add_edge(
                    &model.id.to_string(),
                    &dl.dataset_id,
                    LineageRelation::TrainedOn,
                    Some("training"),
                    None,
                );
            }
        }
    }

    tracker.build()
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_dedup(
    dataset_id: String,
    rows: Vec<Vec<String>>,
    column_names: Vec<String>,
    exact_dedup: Option<bool>,
    near_dedup: Option<bool>,
    similarity_threshold: Option<f64>,
) -> Result<crate::data::dedup::DedupReport, String> {
    let config = crate::data::dedup::DedupConfig {
        exact_dedup: exact_dedup.unwrap_or(true),
        near_dedup: near_dedup.unwrap_or(true),
        similarity_threshold: similarity_threshold.unwrap_or(0.8),
        ..Default::default()
    };
    Ok(crate::data::dedup::DedupAnalyzer::analyze(&dataset_id, &rows, &column_names, &config))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_label_quality(
    dataset_id: String,
    label_column: String,
    annotations: Vec<crate::data::label_quality::AnnotationRecord>,
    num_annotators: usize,
) -> Result<crate::data::label_quality::LabelQualityReport, String> {
    Ok(crate::data::label_quality::LabelQualityAnalyzer::analyze(
        &dataset_id, &label_column, &annotations, num_annotators,
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_slice_analysis(
    dataset_id: String,
    rows: Vec<Vec<String>>,
    column_names: Vec<String>,
    label_column: Option<String>,
    slice_definitions: Vec<crate::data::slice_analysis::SliceDefinition>,
) -> Result<crate::data::slice_analysis::SliceAnalysisReport, String> {
    Ok(crate::data::slice_analysis::SliceAnalyzer::analyze(
        &dataset_id, &rows, &column_names, label_column.as_deref(), &slice_definitions,
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_bias_detection(
    dataset_id: String,
    rows: Vec<Vec<String>>,
    column_names: Vec<String>,
    protected_attributes: Vec<String>,
    label_column: Option<String>,
    positive_label: Option<String>,
) -> Result<crate::data::bias_detection::BiasDetectionReport, String> {
    let config = crate::data::bias_detection::BiasDetectionConfig {
        protected_attributes,
        label_column: label_column.unwrap_or_else(|| "label".to_string()),
        positive_label: positive_label.unwrap_or_else(|| "1".to_string()),
        ..Default::default()
    };
    Ok(crate::data::bias_detection::BiasDetector::analyze(&dataset_id, &rows, &column_names, &config))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_discovery_search(
    datasets: Vec<crate::data::discovery::DiscoveredDataset>,
    query: crate::data::discovery::SearchQuery,
) -> Result<crate::data::discovery::DataDiscoveryIndex, String> {
    Ok(crate::data::discovery::DataDiscoveryEngine::search(&datasets, &query))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_usage_stats(
    dataset_id: String,
    experiment_count: usize,
    model_count: usize,
    last_used: Option<String>,
    most_common_task: Option<String>,
) -> Result<crate::data::discovery::DatasetUsageStats, String> {
    Ok(crate::data::discovery::DataDiscoveryEngine::compute_usage_stats(
        &dataset_id, experiment_count, model_count,
        last_used.as_deref(), most_common_task.as_deref(),
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_confident_learning(
    dataset_id: String,
    labels: Vec<usize>,
    probabilities: Vec<Vec<f64>>,
    class_names: Option<Vec<String>>,
    threshold_method: Option<String>,
    prune_method: Option<String>,
    min_confidence: Option<f64>,
) -> Result<crate::data::confident_learning::LabelErrorReport, String> {
    let tm = match threshold_method.as_deref() {
        Some("calibrated") => crate::data::confident_learning::ThresholdMethod::Calibrated,
        Some("self_confidence") => crate::data::confident_learning::ThresholdMethod::SelfConfidence,
        _ => crate::data::confident_learning::ThresholdMethod::ConfidentJoint,
    };
    let pm = match prune_method.as_deref() {
        Some("by_class") => crate::data::confident_learning::PruneMethod::ByClass,
        Some("both") => crate::data::confident_learning::PruneMethod::Both,
        _ => crate::data::confident_learning::PruneMethod::ByNoiseRate,
    };
    let config = crate::data::confident_learning::ConfidentLearningConfig {
        label_column: "label".to_string(),
        class_names,
        threshold_method: tm,
        prune_method: pm,
        min_confidence: min_confidence.unwrap_or(0.5),
        ..Default::default()
    };
    Ok(crate::data::confident_learning::ConfidentLearning::analyze(
        &dataset_id, &labels, &probabilities, &config,
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_label_quality_summary(
    labels: Vec<usize>,
    num_classes: usize,
) -> Result<crate::data::confident_learning::LabelQualitySummary, String> {
    Ok(crate::data::confident_learning::ConfidentLearning::compute_label_quality_summary(
        &labels, num_classes,
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_curation(
    dataset_id: String,
    rows: Vec<Vec<String>>,
    column_names: Vec<String>,
    _column_types: Vec<String>,
    text_column: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::curation::CurationReport, String> {
    use arrow::array::{StringBuilder};
    use arrow::datatypes::{DataType, Field, Schema};

    if rows.is_empty() || column_names.is_empty() {
        return Err("Empty dataset".to_string());
    }

    let task_id = format!("curation_{}", dataset_id);

    state.event_bus.emit(LabEvent::CurationProgress {
        task_id: task_id.clone(),
        step: "build_arrow".to_string(),
        progress: 10.0,
        message: format!("构建 Arrow 表: {} 行 x {} 列", rows.len(), column_names.len()),
    });

    let fields: Vec<Field> = column_names.iter()
        .map(|name| Field::new(name.as_str(), DataType::Utf8, true))
        .collect();
    let schema = std::sync::Arc::new(Schema::new(fields));

    let mut builders: Vec<StringBuilder> = (0..column_names.len())
        .map(|_| StringBuilder::new())
        .collect();

    for row in &rows {
        for (col_idx, builder) in builders.iter_mut().enumerate() {
            if col_idx < row.len() {
                builder.append_value(&row[col_idx]);
            } else {
                builder.append_null();
            }
        }
    }

    let arrays: Vec<std::sync::Arc<dyn arrow::array::Array>> = builders
        .into_iter()
        .map(|mut b| std::sync::Arc::new(b.finish()) as std::sync::Arc<dyn arrow::array::Array>)
        .collect();

    let batch = arrow::record_batch::RecordBatch::try_new(schema.clone(), arrays)
        .map_err(|e| format!("Failed to create record batch: {}", e))?;

    let mut table = crate::data::arrow_table::ArrowTable::new(&dataset_id, schema);
    table.add_batch(batch)?;

    state.event_bus.emit(LabEvent::CurationProgress {
        task_id: task_id.clone(),
        step: "curate".to_string(),
        progress: 50.0,
        message: "执行数据策展分析...".to_string(),
    });

    let text_col = text_column.unwrap_or_else(|| column_names[0].clone());
    let config = crate::data::curation::CurationConfig::default();
    let curator = crate::data::curation::DataCurator::new(config);
    let report = curator.curate_dataset(&table, &text_col)?;

    state.event_bus.emit(LabEvent::OperationCompleted {
        task_id,
        operation: "curation".to_string(),
        result: serde_json::json!({
            "dataset_id": dataset_id,
            "total_rows": rows.len(),
            "total_columns": column_names.len(),
        }),
    });

    Ok(report)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_quality_score(
    dataset_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::quality_score::QualityScore, String> {
    let ds_id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&ds_id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("数据集不存在: {}", dataset_id))?;

    Ok(crate::data::quality_score::QualityScorer::score(
        &dataset.id.to_string(),
        &dataset.name,
        dataset.rows,
        dataset.columns,
        &dataset.column_profiles,
        dataset.has_missing_values(),
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_recommend_for_plan(
    plan_json: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::dataset_recommender::RecommendationResult, String> {
    let plan: crate::data::training_plan::TrainingPlan = serde_json::from_str(&plan_json)
        .map_err(|e| format!("解析训练计划失败: {}", e))?;

    let datasets = state.dataset_repo.list(&crate::domain::dataset::aggregate::DatasetFilter::default())
        .await.map_err(|e| e.to_string())?;

    let mut quality_scores = std::collections::HashMap::new();
    for ds in &datasets {
        let score = crate::data::quality_score::QualityScorer::score(
            &ds.id.to_string(),
            &ds.name,
            ds.rows,
            ds.columns,
            &[],
            false,
        );
        quality_scores.insert(ds.id.to_string(), score.overall_score);
    }

    Ok(crate::data::dataset_recommender::DatasetRecommender::recommend(
        &plan,
        &datasets,
        &quality_scores,
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_multimodal_images(
    dataset_id: String,
    image_metadata: Vec<crate::data::multimodal::ImageMetadata>,
) -> Result<crate::data::multimodal::MultimodalAnalysisReport, String> {
    Ok(crate::data::multimodal::MultimodalAnalyzer::analyze_images(&dataset_id, &image_metadata))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_multimodal_texts(
    dataset_id: String,
    texts: Vec<String>,
) -> Result<crate::data::multimodal::MultimodalAnalysisReport, String> {
    Ok(crate::data::multimodal::MultimodalAnalyzer::analyze_texts(&dataset_id, &texts))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_remote_storage_validate(
    config: crate::data::remote_storage::RemoteStorageConfig,
) -> Result<(), Vec<String>> {
    crate::data::remote_storage::RemoteStorageManager::validate_config(&config)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_remote_storage_build_url(
    config: crate::data::remote_storage::RemoteStorageConfig,
    key: String,
) -> Result<String, String> {
    Ok(crate::data::remote_storage::RemoteStorageManager::build_object_url(&config, &key))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_remote_storage_estimate_transfer(
    total_bytes: u64,
    bandwidth_mbps: f64,
) -> Result<crate::data::remote_storage::TransferEstimate, String> {
    Ok(crate::data::remote_storage::RemoteStorageManager::estimate_transfer_time(total_bytes, bandwidth_mbps))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_remote_storage_recommend_class(
    access_pattern: String,
    total_size_bytes: u64,
) -> Result<Vec<crate::data::remote_storage::StorageClassRecommendation>, String> {
    let pattern = match access_pattern.as_str() {
        "infrequent" => crate::data::remote_storage::AccessPattern::Infrequent,
        "archive" => crate::data::remote_storage::AccessPattern::Archive,
        "training" => crate::data::remote_storage::AccessPattern::Training,
        _ => crate::data::remote_storage::AccessPattern::Frequent,
    };
    Ok(crate::data::remote_storage::RemoteStorageManager::recommend_storage_class(pattern, total_size_bytes))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_remote_storage_sync_plan(
    local_files: Vec<crate::data::remote_storage::LocalFileInfo>,
    remote_objects: Vec<crate::data::remote_storage::RemoteObjectInfo>,
) -> Result<crate::data::remote_storage::SyncPlan, String> {
    Ok(crate::data::remote_storage::RemoteStorageManager::generate_sync_plan(&local_files, &remote_objects))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_influence_tracin(
    dataset_id: String,
    model_id: Option<String>,
    sample_gradients: Vec<Vec<f64>>,
    checkpoint_gradients: Vec<Vec<Vec<f64>>>,
    labels: Vec<String>,
    predictions: Vec<String>,
    top_k: Option<usize>,
) -> Result<crate::data::influence::InfluenceAnalysisReport, String> {
    let config = crate::data::influence::InfluenceConfig {
        top_k: top_k.unwrap_or(50),
        ..Default::default()
    };
    Ok(crate::data::influence::InfluenceAnalyzer::analyze_tracin(
        &dataset_id, model_id.as_deref(), &sample_gradients, &checkpoint_gradients,
        &labels, &predictions, &config,
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_influence_loo(
    dataset_id: String,
    model_id: Option<String>,
    full_model_loss: f64,
    loo_losses: Vec<f64>,
    labels: Vec<String>,
    predictions: Vec<String>,
    top_k: Option<usize>,
) -> Result<crate::data::influence::InfluenceAnalysisReport, String> {
    let config = crate::data::influence::InfluenceConfig {
        top_k: top_k.unwrap_or(50),
        ..Default::default()
    };
    Ok(crate::data::influence::InfluenceAnalyzer::analyze_loo(
        &dataset_id, model_id.as_deref(), full_model_loss, &loo_losses,
        &labels, &predictions, &config,
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_influence_loss_diff(
    dataset_id: String,
    model_id: Option<String>,
    per_sample_losses: Vec<f64>,
    labels: Vec<String>,
    predictions: Vec<String>,
    top_k: Option<usize>,
) -> Result<crate::data::influence::InfluenceAnalysisReport, String> {
    let config = crate::data::influence::InfluenceConfig {
        top_k: top_k.unwrap_or(50),
        ..Default::default()
    };
    Ok(crate::data::influence::InfluenceAnalyzer::analyze_loss_difference(
        &dataset_id, model_id.as_deref(), &per_sample_losses,
        &labels, &predictions, &config,
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dashboard_build(
    snapshots: Vec<crate::data::dashboard::QualitySnapshot>,
    alerts: Vec<crate::data::dashboard::QualityAlert>,
    lineage: Option<crate::data::dashboard::LineageGraphData>,
) -> Result<crate::data::dashboard::DataQualityDashboard, String> {
    Ok(crate::data::dashboard::DashboardBuilder::build(&snapshots, &alerts, lineage.as_ref()))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dashboard_create_alert(
    dataset_id: String,
    dataset_name: String,
    alert_type: String,
    severity: String,
    message: String,
) -> Result<crate::data::dashboard::QualityAlert, String> {
    let at = match alert_type.as_str() {
        "drift_detected" => crate::data::dashboard::AlertType::DriftDetected,
        "quality_drop" => crate::data::dashboard::AlertType::QualityDrop,
        "bias_detected" => crate::data::dashboard::AlertType::BiasDetected,
        "label_noise" => crate::data::dashboard::AlertType::LabelNoise,
        "missing_data" => crate::data::dashboard::AlertType::MissingData,
        "schema_change" => crate::data::dashboard::AlertType::SchemaChange,
        "duplicate_data" => crate::data::dashboard::AlertType::DuplicateData,
        "outlier_surge" => crate::data::dashboard::AlertType::OutlierSurge,
        _ => return Err(format!("Unknown alert type: {}", alert_type)),
    };
    let sev = match severity.as_str() {
        "critical" => crate::data::dashboard::AlertSeverity::Critical,
        "warning" => crate::data::dashboard::AlertSeverity::Warning,
        _ => crate::data::dashboard::AlertSeverity::Info,
    };
    Ok(crate::data::dashboard::DashboardBuilder::create_alert(
        &dataset_id, &dataset_name, at, sev, &message,
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dashboard_create_snapshot(
    dataset_id: String,
    dataset_name: String,
    quality_score: f64,
    completeness: f64,
    consistency: f64,
    uniqueness: f64,
    label_quality: Option<f64>,
    drift_detected: Option<bool>,
    bias_level: Option<String>,
    sample_count: usize,
    issue_count: usize,
) -> Result<crate::data::dashboard::QualitySnapshot, String> {
    Ok(crate::data::dashboard::DashboardBuilder::create_snapshot(
        &dataset_id, &dataset_name, quality_score, completeness, consistency,
        uniqueness, label_quality, drift_detected.unwrap_or(false),
        bias_level.as_deref(), sample_count, issue_count,
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dashboard_lineage_from_graph(
    graph: crate::data::lineage::LineageGraph,
) -> Result<crate::data::dashboard::LineageGraphData, String> {
    Ok(crate::data::dashboard::DashboardBuilder::build_lineage_from_graph(&graph))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_arrow_table_info(
    name: String,
    csv_path: String,
    has_header: Option<bool>,
) -> Result<crate::data::arrow_table::ArrowTableInfo, String> {
    let table = crate::data::arrow_table::csv_to_arrow_table(
        &name, &csv_path, has_header.unwrap_or(true),
    )?;
    Ok(table.info())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_arrow_table_column_stats(
    name: String,
    csv_path: String,
    column_name: String,
) -> Result<crate::data::arrow_table::ColumnStats, String> {
    let table = crate::data::arrow_table::csv_to_arrow_table(&name, &csv_path, true)?;
    table.column_stats(&column_name)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_arrow_table_slice(
    name: String,
    csv_path: String,
    offset: usize,
    length: usize,
) -> Result<crate::data::arrow_table::ArrowTableInfo, String> {
    let table = crate::data::arrow_table::csv_to_arrow_table(&name, &csv_path, true)?;
    let sliced = table.slice(offset, length)?;
    Ok(sliced.info())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_arrow_table_select(
    name: String,
    csv_path: String,
    columns: Vec<String>,
) -> Result<crate::data::arrow_table::ArrowTableInfo, String> {
    let table = crate::data::arrow_table::csv_to_arrow_table(&name, &csv_path, true)?;
    let col_refs: Vec<&str> = columns.iter().map(|s| s.as_str()).collect();
    let projected = table.select_columns(&col_refs)?;
    Ok(projected.info())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_streaming_open_csv(
    name: String,
    path: String,
    chunk_size: Option<usize>,
) -> Result<crate::data::streaming::StreamingDatasetInfo, String> {
    let config = crate::data::streaming::StreamingConfig {
        chunk_size: chunk_size.unwrap_or(10000),
        ..Default::default()
    };
    let ds = crate::data::streaming::StreamingDataset::open_csv(&name, &path, config)?;
    Ok(ds.info)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_streaming_open_jsonl(
    name: String,
    path: String,
    chunk_size: Option<usize>,
) -> Result<crate::data::streaming::StreamingDatasetInfo, String> {
    let config = crate::data::streaming::StreamingConfig {
        chunk_size: chunk_size.unwrap_or(10000),
        ..Default::default()
    };
    let ds = crate::data::streaming::StreamingDataset::open_jsonl(&name, &path, config)?;
    Ok(ds.info)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_streaming_recommend_chunk(
    file_size_bytes: u64,
    target_memory_mb: Option<f64>,
) -> Result<usize, String> {
    Ok(crate::data::streaming::auto_recommend_chunk_size(
        file_size_bytes, target_memory_mb.unwrap_or(512.0),
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_sharding_plan(
    total_rows: usize,
    num_shards: usize,
    strategy: String,
) -> Result<crate::data::sharding::ShardPlan, String> {
    let strat = match strategy.as_str() {
        "interleaved" => crate::data::sharding::ShardStrategy::Interleaved,
        "hashed" => crate::data::sharding::ShardStrategy::Hashed,
        "stratified" => crate::data::sharding::ShardStrategy::Stratified,
        "weighted" => crate::data::sharding::ShardStrategy::Weighted,
        _ => crate::data::sharding::ShardStrategy::Contiguous,
    };
    Ok(crate::data::sharding::DataSharder::compute_shard_plan(total_rows, num_shards, strat, None))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_sharding_indices(
    total_rows: usize,
    num_shards: usize,
    shard_index: usize,
    strategy: String,
) -> Result<Vec<usize>, String> {
    let strat = match strategy.as_str() {
        "interleaved" => crate::data::sharding::ShardStrategy::Interleaved,
        "hashed" => crate::data::sharding::ShardStrategy::Hashed,
        _ => crate::data::sharding::ShardStrategy::Contiguous,
    };
    let config = crate::data::sharding::ShardingConfig {
        num_shards,
        shard_index,
        strategy: strat,
        ..Default::default()
    };
    crate::data::sharding::DataSharder::shard_indices(total_rows, &config)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_query_execute(
    csv_path: String,
    plan: crate::data::query_engine::QueryPlan,
) -> Result<crate::data::query_engine::QueryResult, String> {
    let table = crate::data::arrow_table::csv_to_arrow_table("query_input", &csv_path, true)?;
    crate::data::query_engine::QueryEngine::execute(&table, &plan)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_query_estimate_cost(
    plan: crate::data::query_engine::QueryPlan,
    input_rows: usize,
) -> Result<f64, String> {
    Ok(crate::data::query_engine::QueryEngine::estimate_cost(&plan, input_rows))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_packing_pack(
    sequences: Vec<Vec<i64>>,
    max_sequence_length: usize,
    strategy: Option<String>,
    pad_token_id: Option<i64>,
    eos_token_id: Option<i64>,
    shuffle: Option<bool>,
) -> Result<crate::data::packing::PackingReport, String> {
    let strat = match strategy.as_deref() {
        Some("greedy") => crate::data::packing::PackingStrategy::Greedy,
        Some("best_fit") => crate::data::packing::PackingStrategy::BestFit,
        Some("first_fit") => crate::data::packing::PackingStrategy::FirstFit,
        Some("no_pack") => crate::data::packing::PackingStrategy::NoPack,
        _ => crate::data::packing::PackingStrategy::BinPacking,
    };
    let config = crate::data::packing::PackingConfig {
        max_sequence_length,
        pad_token_id: pad_token_id.unwrap_or(0),
        eos_token_id,
        strategy: strat,
        shuffle_before_pack: shuffle.unwrap_or(true),
        ..Default::default()
    };
    let (_, report) = crate::data::packing::SequencePacker::pack(&sequences, &config)?;
    Ok(report)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_packing_estimate(
    sequence_lengths: Vec<usize>,
    max_sequence_length: usize,
) -> Result<f64, String> {
    Ok(crate::data::packing::SequencePacker::estimate_packing_efficiency(
        &sequence_lengths, max_sequence_length,
    ))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_interleave_compute(
    config: crate::data::interleave::InterleaveConfig,
) -> Result<crate::data::interleave::InterleaveReport, String> {
    crate::data::interleave::DatasetInterleaver::validate_config(&config)?;
    let report = crate::data::interleave::InterleaveReport {
        total_samples: 0,
        dataset_contributions: config.datasets.iter().map(|d| {
            crate::data::interleave::DatasetContribution {
                dataset_id: d.dataset_id.clone(),
                dataset_name: d.dataset_name.clone(),
                target_weight: d.weight,
                actual_weight: 0.0,
                samples_used: 0,
                samples_available: 0,
            }
        }).collect(),
        strategy: config.strategy,
        distribution_match: 1.0,
    };
    Ok(report)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_provenance_report(
    dataset_id: String,
    records: Vec<crate::data::provenance::ProvenanceRecord>,
) -> Result<crate::data::provenance::ProvenanceReport, String> {
    let mut tracker = crate::data::provenance::ProvenanceTracker::new();
    for record in records {
        tracker.add_record(record);
    }
    Ok(tracker.generate_report(&dataset_id))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_provenance_check_commercial(
    records: Vec<crate::data::provenance::ProvenanceRecord>,
) -> Result<bool, Vec<String>> {
    let mut tracker = crate::data::provenance::ProvenanceTracker::new();
    for record in records {
        tracker.add_record(record);
    }
    tracker.check_commercial_use()
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_checkpoint_create(
    training_id: String,
    dataset_id: String,
    epoch: usize,
    global_step: usize,
    samples_seen: usize,
    cursors: Vec<crate::data::checkpoint::DatasetCursor>,
    save_directory: Option<String>,
) -> Result<crate::data::checkpoint::DataLoaderCheckpoint, String> {
    let config = crate::data::checkpoint::CheckpointConfig {
        save_directory: save_directory.unwrap_or_else(|| "./checkpoints".to_string()),
        ..Default::default()
    };
    let mut manager = crate::data::checkpoint::CheckpointManager::new(config);
    let cursor_map: std::collections::HashMap<String, crate::data::checkpoint::DatasetCursor> = cursors
        .into_iter()
        .map(|c| (c.dataset_id.clone(), c))
        .collect();
    let ckpt = manager.create_checkpoint(
        &training_id, &dataset_id, epoch, global_step, samples_seen, cursor_map, None,
    );
    manager.save_to_disk(&ckpt)?;
    Ok(ckpt)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_checkpoint_list(
    save_directory: Option<String>,
) -> Result<Vec<crate::data::checkpoint::DataLoaderCheckpoint>, String> {
    let config = crate::data::checkpoint::CheckpointConfig {
        save_directory: save_directory.unwrap_or_else(|| "./checkpoints".to_string()),
        ..Default::default()
    };
    let manager = crate::data::checkpoint::CheckpointManager::new(config);
    manager.list_checkpoints()
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_checkpoint_resume(
    save_directory: Option<String>,
) -> Result<Option<crate::data::checkpoint::DataLoaderCheckpoint>, String> {
    let config = crate::data::checkpoint::CheckpointConfig {
        save_directory: save_directory.unwrap_or_else(|| "./checkpoints".to_string()),
        auto_resume: true,
        ..Default::default()
    };
    let manager = crate::data::checkpoint::CheckpointManager::new(config);
    manager.auto_resume()
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_validate_dataset_integrity(
    dataset_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::DataValidationResult, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    let accessor = state.data_accessor_registry.get(&dataset.format).await
        .ok_or_else(|| format!("No data accessor for format: {}", dataset.format))?;

    let column_names: Vec<String> = dataset.column_profiles.iter().map(|p| p.name.clone()).collect();
    accessor.validate(
        &dataset.path,
        Some(&dataset.digest),
        Some(dataset.rows),
        Some(dataset.columns),
        Some(&column_names),
    ).await.map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_preview(
    dataset_id: String,
    offset: Option<usize>,
    limit: Option<usize>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::DataPage, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    let accessor = state.data_accessor_registry.get(&dataset.format).await
        .ok_or_else(|| format!("No data accessor for format: {}", dataset.format))?;

    accessor.page(&dataset.path, offset.unwrap_or(0), limit.unwrap_or(50))
        .await.map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_sample(
    dataset_id: String,
    n: Option<usize>,
    seed: Option<u64>,
    strategy: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::DataPage, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    let accessor = state.data_accessor_registry.get(&dataset.format).await
        .ok_or_else(|| format!("No data accessor for format: {}", dataset.format))?;

    let sample_strategy = match strategy.as_deref().unwrap_or("first") {
        "random" => crate::data::SampleStrategy::Random,
        "stratified" => crate::data::SampleStrategy::Stratified,
        _ => crate::data::SampleStrategy::First,
    };

    let config = crate::data::DataSampleConfig {
        n: n.unwrap_or(100),
        seed,
        strategy: sample_strategy,
    };

    accessor.sample(&dataset.path, &config).await.map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_column_stats(
    dataset_id: String,
    column_name: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::DataStatistics, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    let accessor = state.data_accessor_registry.get(&dataset.format).await
        .ok_or_else(|| format!("No data accessor for format: {}", dataset.format))?;

    accessor.statistics(&dataset.path, &column_name).await.map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_read_split(
    dataset_id: String,
    split_name: String,
    split_type: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::DataPage, String> {
    let id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    let split = state.dataset_repo.load_split(&id, &split_name).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Split '{}' not found", split_name))?;

    let accessor = state.data_accessor_registry.get(&dataset.format).await
        .ok_or_else(|| format!("No data accessor for format: {}", dataset.format))?;

    let indices = match split_type.as_deref().unwrap_or("train") {
        "val" | "validation" => &split.val_indices,
        "test" => &split.test_indices,
        _ => &split.train_indices,
    };

    let offset_val = offset.unwrap_or(0);
    let limit_val = limit.unwrap_or(50);
    let sliced: Vec<usize> = indices.iter().skip(offset_val).take(limit_val).copied().collect();

    accessor.read_rows_by_indices(&dataset.path, &sliced).await.map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_dataset_lineage(
    dataset_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let ds_id = DatasetId::from_str(&dataset_id);
    let dataset = state.dataset_repo.load(&ds_id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Dataset not found".to_string())?;

    let mut experiments: Vec<serde_json::Value> = Vec::new();
    let filter = crate::domain::experiment::ExperimentFilter::default();
    let exp_list = state.experiment_repo.list(&filter).await.map_err(|e| e.to_string())?;
    for exp_summary in &exp_list {
        if exp_summary.dataset_id.as_deref() == Some(&dataset_id) {
            experiments.push(serde_json::json!({
                "experiment_id": exp_summary.id.to_string(),
                "experiment_name": exp_summary.name.clone(),
                "status": format!("{}", exp_summary.status),
                "dataset_version": exp_summary.dataset_version,
            }));
        }
    }

    let mut models: Vec<serde_json::Value> = Vec::new();
    let model_list = state.model_repo.list(None).await.map_err(|e| e.to_string())?;
    for model in &model_list {
        if let Some(ref lineage) = model.lineage {
            let uses_dataset = lineage.dataset.as_deref() == Some(&dataset.path)
                || lineage.datasets.iter().any(|d| d.dataset_id == dataset_id);
            if uses_dataset {
                models.push(serde_json::json!({
                    "model_id": model.id.to_string(),
                    "model_name": model.name.clone(),
                    "version": model.version.clone(),
                    "framework": model.framework.clone(),
                    "split_name": lineage.split_name,
                }));
            }
        }
    }

    Ok(serde_json::json!({
        "dataset_id": dataset_id,
        "dataset_name": dataset.name,
        "dataset_version": dataset.version.to_string(),
        "experiments": experiments,
        "models": models,
        "experiment_count": experiments.len(),
        "model_count": models.len(),
    }))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_model_lineage(
    model_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let m_id = ModelId::from_str(&model_id);
    let model = state.model_repo.load(&m_id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Model not found".to_string())?;

    let lineage = model.lineage.as_ref();

    let experiment_info = if let Some(ref exp_id) = lineage.and_then(|l| l.experiment_id.clone()) {
        let id = ExperimentId::from_str(exp_id);
        state.experiment_repo.load(&id).await
            .map_err(|e| e.to_string())?
            .map(|exp| serde_json::json!({
                "experiment_id": exp_id,
                "experiment_name": exp.name,
                "status": format!("{}", exp.status),
                "created_at": exp.created_at.to_rfc3339(),
            }))
    } else {
        None
    };

    let dataset_infos: Vec<serde_json::Value> = if let Some(ref lineage_val) = lineage {
        let mut infos = Vec::new();
        for ds_lineage in &lineage_val.datasets {
            let ds_id = DatasetId::from_str(&ds_lineage.dataset_id);
            if let Ok(Some(ds)) = state.dataset_repo.load(&ds_id).await {
                infos.push(serde_json::json!({
                    "dataset_id": ds_lineage.dataset_id,
                    "dataset_name": ds.name,
                    "dataset_version": ds_lineage.dataset_version,
                    "split_name": ds_lineage.split_name,
                    "data_path": ds_lineage.data_path,
                    "rows": ds.rows,
                    "columns": ds.columns,
                }));
            } else {
                infos.push(serde_json::json!({
                    "dataset_id": ds_lineage.dataset_id,
                    "dataset_version": ds_lineage.dataset_version,
                    "split_name": ds_lineage.split_name,
                    "data_path": ds_lineage.data_path,
                }));
            }
        }
        infos
    } else {
        Vec::new()
    };

    Ok(serde_json::json!({
        "model_id": model_id,
        "model_name": model.name,
        "version": model.version,
        "experiment": experiment_info,
        "datasets": dataset_infos,
        "parent_model_id": lineage.and_then(|l| l.parent_model_id.clone()),
        "split_name": lineage.and_then(|l| l.split_name.clone()),
        "preprocessing_pipeline": lineage.and_then(|l| l.preprocessing_pipeline.clone()),
    }))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_save_evaluation(
    experiment_id: String,
    evaluation_result: serde_json::Value,
    test_data_path: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    if test_data_path.contains("..") || test_data_path.contains('~') {
        return Err("Test data path contains invalid traversal sequence".to_string());
    }

    let id = ExperimentId::from_str(&experiment_id);

    let eval_filename = format!("evaluation_{}.json", chrono::Utc::now().format("%Y%m%d_%H%M%S"));

    let artifact = crate::domain::experiment::ArtifactRef::new(
        "evaluation".to_string(),
        eval_filename.clone(),
        0,
    )
    .with_description(format!("Evaluation on {}", test_data_path))
    .with_metadata(evaluation_result.clone());

    state.experiment_handler.handle(
        crate::domain::experiment::commands::ExperimentCommand::AddArtifact {
            experiment_id: id,
            artifact,
        }
    ).await.map_err(|e| e.to_string())?;

    let artifact_dir = crate::core::config::get_artifact_dir(&experiment_id);
    let eval_path = std::path::Path::new(&artifact_dir).join(&eval_filename);
    std::fs::write(&eval_path, serde_json::to_string_pretty(&evaluation_result).unwrap_or_default())
        .map_err(|e| format!("Failed to save evaluation: {}", e))?;

    Ok(eval_path.to_string_lossy().to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_evaluations(
    experiment_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let id = ExperimentId::from_str(&experiment_id);
    let experiment = state.experiment_repo.load(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Experiment not found".to_string())?;

    let evaluations: Vec<serde_json::Value> = experiment.artifacts.iter()
        .filter(|a| a.artifact_type == "evaluation")
        .map(|a| {
            let mut result = serde_json::json!({
                "path": a.path,
                "description": a.description,
                "created_at": a.created_at.to_rfc3339(),
            });
            if let Some(ref meta) = a.metadata {
                result.as_object_mut().unwrap().insert("result".to_string(), meta.clone());
            }
            result
        })
        .collect();

    Ok(evaluations)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_experiment_add_tag(
    experiment_id: String,
    tag: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ExperimentCommand::AddTag {
        experiment_id: ExperimentId::from_str(&experiment_id),
        tag,
    };
    state.command_bus.dispatch_experiment(cmd).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_experiment_set_param(
    experiment_id: String,
    key: String,
    value: serde_json::Value,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ExperimentCommand::SetParam {
        experiment_id: ExperimentId::from_str(&experiment_id),
        key,
        value,
    };
    state.command_bus.dispatch_experiment(cmd).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_delete_experiment(
    experiment_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let id = ExperimentId::from_str(&experiment_id);
    let experiment = state.experiment_repo.load(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Experiment not found".to_string())?;

    if experiment.status == crate::domain::experiment::aggregate::ExperimentStatus::Running {
        return Err("Cannot delete a running experiment, please stop it first".to_string());
    }

    state.experiment_repo.delete(&id).await.map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_archive_experiment(
    experiment_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ExperimentCommand::ArchiveExperiment {
        experiment_id: ExperimentId::from_str(&experiment_id),
    };
    state.command_bus.dispatch_experiment(cmd).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_restore_experiment(
    experiment_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ExperimentCommand::RestoreExperiment {
        experiment_id: ExperimentId::from_str(&experiment_id),
    };
    state.command_bus.dispatch_experiment(cmd).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_clone_experiment(
    experiment_id: String,
    new_name: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    if new_name.trim().is_empty() {
        return Err("Cloned experiment name cannot be empty".to_string());
    }
    let cmd = ExperimentCommand::CloneExperiment {
        experiment_id: ExperimentId::from_str(&experiment_id),
        new_name,
    };
    match state.command_bus.dispatch_experiment(cmd).await.map_err(|e| e.to_string())? {
        Some(id) => Ok(id.to_string()),
        None => Err("Clone experiment failed: no id returned".to_string()),
    }
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_batch_delete_experiments(
    experiment_ids: Vec<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<usize, String> {
    if experiment_ids.is_empty() {
        return Ok(0);
    }
    if experiment_ids.len() > 100 {
        return Err("Cannot delete more than 100 experiments at once".to_string());
    }

    let mut deleted = 0usize;
    let mut errors = Vec::new();
    for eid in &experiment_ids {
        let id = ExperimentId::from_str(eid);
        if let Ok(Some(exp)) = state.experiment_repo.load(&id).await {
            if exp.status == crate::domain::experiment::aggregate::ExperimentStatus::Running {
                errors.push(format!("{}: running, skipped", eid));
                continue;
            }
            if exp.status == crate::domain::experiment::aggregate::ExperimentStatus::Paused {
                errors.push(format!("{}: paused, skipped", eid));
                continue;
            }
        }
        match state.experiment_repo.delete(&id).await {
            Ok(()) => deleted += 1,
            Err(e) => errors.push(format!("{}: {}", eid, e)),
        }
    }
    if !errors.is_empty() && deleted == 0 {
        return Err(format!("All deletions failed: {}", errors.join("; ")));
    }
    Ok(deleted)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_set_experiment_group(
    experiment_id: String,
    group: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let cmd = ExperimentCommand::SetGroup {
        experiment_id: ExperimentId::from_str(&experiment_id),
        group,
    };
    state.command_bus.dispatch_experiment(cmd).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_experiment_groups(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<String>, String> {
    let filter = ExperimentFilter::default();
    let experiments = state.experiment_repo.list(&filter).await.map_err(|e| e.to_string())?;
    let mut groups: Vec<String> = experiments.iter()
        .filter_map(|e| e.group.clone())
        .collect();
    groups.sort();
    groups.dedup();
    Ok(groups)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_preprocess_data(
    data_path: String,
    _data_format: String,
    steps: Vec<crate::data::pipeline::PipelineStep>,
    _state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::data::data_trait::DataPreview, String> {
    if data_path.contains("..") || data_path.contains('~') {
        return Err("Path contains invalid traversal sequence".to_string());
    }

    let content = std::fs::read_to_string(&data_path)
        .map_err(|e| format!("Cannot read file: {}", e))?;

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(content.as_bytes());

    let headers = reader.headers()
        .map_err(|e| format!("Cannot parse headers: {}", e))?
        .clone();

    let mut column_names: Vec<String> = headers.iter().map(|h| h.to_string()).collect();
    let mut records: Vec<Vec<String>> = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| format!("Cannot parse record: {}", e))?;
        let row: Vec<String> = record.iter().map(|v| v.to_string()).collect();
        records.push(row);
    }

    let pipeline = crate::data::DataPipeline { steps };
    pipeline.execute_on_records(&mut column_names, &mut records).map_err(|e| e.to_string())?;

    let processed_path = {
        let original = std::path::Path::new(&data_path);
        let stem = original.file_stem().and_then(|s| s.to_str()).unwrap_or("data");
        let ext = original.extension().and_then(|s| s.to_str()).unwrap_or("csv");
        let parent = original.parent().unwrap_or(std::path::Path::new("."));
        parent.join(format!("{}_preprocessed.{}", stem, ext)).to_string_lossy().to_string()
    };

    {
        let mut wtr = csv::Writer::from_path(&processed_path)
            .map_err(|e| format!("Cannot create preprocessed file: {}", e))?;
        wtr.write_record(&column_names)
            .map_err(|e| format!("Cannot write headers: {}", e))?;
        for row in &records {
            wtr.write_record(row.iter().map(|v| if v.is_empty() { "" } else { v.as_str() }))
                .map_err(|e| format!("Cannot write record: {}", e))?;
        }
        wtr.flush().map_err(|e| format!("Cannot flush: {}", e))?;
    }

    let column_types: Vec<String> = column_names.iter().enumerate().map(|(col_idx, _)| {
        let is_numeric = records.iter().any(|row| {
            row.get(col_idx).map(|v| !v.trim().is_empty() && v.parse::<f64>().is_ok()).unwrap_or(false)
        });
        if is_numeric { "float".to_string() } else { "categorical".to_string() }
    }).collect();

    let preview_rows: Vec<Vec<serde_json::Value>> = records.iter().take(50)
        .map(|row| row.iter().enumerate().map(|(_ci, v)| {
            let trimmed = v.trim();
            if trimmed.is_empty() {
                serde_json::Value::Null
            } else if let Ok(n) = trimmed.parse::<i64>() {
                serde_json::json!(n)
            } else if let Ok(f) = trimmed.parse::<f64>() {
                serde_json::json!(f)
            } else {
                serde_json::json!(trimmed)
            }
        }).collect())
        .collect();

    Ok(crate::data::DataPreview {
        columns: column_names,
        column_types,
        rows: preview_rows,
        total_rows: records.len(),
        offset: 0,
    })
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_artifacts(
    experiment_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<crate::domain::experiment::ArtifactRef>, String> {
    let id = ExperimentId::from_str(&experiment_id);
    let experiment = state.experiment_repo.load(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Experiment not found".to_string())?;
    Ok(experiment.artifacts)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_get_artifact_content(
    experiment_id: String,
    artifact_path: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<u8>, String> {
    let id = ExperimentId::from_str(&experiment_id);
    let experiment = state.experiment_repo.load(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Experiment not found".to_string())?;

    let found = experiment.artifacts.iter()
        .any(|a| a.path == artifact_path);
    if !found {
        return Err("Artifact not found in experiment".to_string());
    }

    let artifact_dir = crate::core::config::get_artifact_dir(&experiment_id);
    let path = std::path::Path::new(&artifact_path);

    let canonical_artifact = path
        .canonicalize()
        .map_err(|_| "Invalid or non-existent artifact path".to_string())?;
    let canonical_dir = std::path::Path::new(&artifact_dir)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&artifact_dir));

    if !canonical_artifact.starts_with(&canonical_dir) {
        return Err("Artifact path is outside the allowed directory".to_string());
    }

    std::fs::read(&canonical_artifact)
        .map_err(|e| format!("Cannot read artifact: {}", e))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_scan_artifacts(
    experiment_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<crate::domain::experiment::ArtifactRef>, String> {
    let artifact_dir = crate::core::config::get_artifact_dir(&experiment_id);
    let dir = std::path::Path::new(&artifact_dir);

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut artifacts = Vec::new();
    scan_dir_recursive(dir, dir, &mut artifacts)?;

    let id = ExperimentId::from_str(&experiment_id);
    let mut experiment = state.experiment_repo.load(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Experiment not found".to_string())?;

    for artifact in &artifacts {
        if !experiment.artifacts.iter().any(|a| a.path == artifact.path) {
            experiment.add_artifact(artifact.clone());
        }
    }
    state.experiment_repo.save(&experiment).await.map_err(|e| e.to_string())?;

    Ok(artifacts)
}

fn scan_dir_recursive(
    base: &std::path::Path,
    current: &std::path::Path,
    artifacts: &mut Vec<crate::domain::experiment::ArtifactRef>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(current)
        .map_err(|e| format!("Cannot read dir: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
        let path = entry.path();

        let is_symlink = path.symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);
        if is_symlink {
            continue;
        }

        if path.is_dir() {
            let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
            let canonical_base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
            if !canonical.starts_with(&canonical_base) {
                continue;
            }
            scan_dir_recursive(base, &path, artifacts)?;
        } else {
            let metadata = std::fs::metadata(&path)
                .map_err(|e| format!("Cannot read file metadata: {}", e))?;

            let relative = path.strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            let extension = path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string();

            let artifact_type = if file_name.ends_with(".mpk.gz") {
                "checkpoint"
            } else {
                match extension.as_str() {
                    "mpk" | "bin" => "checkpoint",
                    "json" => "config",
                    "log" | "txt" => "log",
                    "csv" => "data",
                    _ => "other",
                }
            };

            let checksum = if metadata.len() <= 10 * 1024 * 1024 {
                let data = std::fs::read(&path).unwrap_or_default();
                crate::domain::experiment::ArtifactRef::compute_checksum(&data)
            } else {
                String::new()
            };

            artifacts.push(
                crate::domain::experiment::ArtifactRef::new(
                    artifact_type.to_string(),
                    path.to_string_lossy().to_string(),
                    metadata.len(),
                )
                .with_description(relative)
                .with_checksum(checksum)
            );
        }
    }

    Ok(())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_checkpoints(
    experiment_id: String,
    _state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let artifact_dir = crate::core::config::get_artifact_dir(&experiment_id);
    let dir = std::path::Path::new(&artifact_dir);

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut checkpoints = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

            if name.starts_with("checkpoint-") || name.starts_with("model-") {
                let epoch = name.split('-').last()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0);

                let size_bytes: u64 = if path.is_dir() {
                    let mut total: u64 = 0;
                    if let Ok(dir_entries) = std::fs::read_dir(&path) {
                        for de in dir_entries.flatten() {
                            if let Ok(meta) = de.metadata() {
                                total += meta.len();
                            }
                        }
                    }
                    total
                } else {
                    std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
                };

                let modified = path.metadata().ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs());

                let checkpoint_file = if path.is_dir() {
                    crate::engine::burn_training::find_checkpoint_in_dir(&path)
                        .map(|p| p.to_string_lossy().to_string())
                } else {
                    Some(path.to_string_lossy().to_string())
                };

                checkpoints.push(serde_json::json!({
                    "name": name,
                    "epoch": epoch,
                    "path": path.to_string_lossy().to_string(),
                    "checkpoint_file": checkpoint_file,
                    "size_bytes": size_bytes,
                    "modified_timestamp": modified,
                }));
            }
        }
    }

    checkpoints.sort_by(|a, b| {
        let ea = a.get("epoch").and_then(|v| v.as_u64()).unwrap_or(0);
        let eb = b.get("epoch").and_then(|v| v.as_u64()).unwrap_or(0);
        eb.cmp(&ea)
    });

    Ok(checkpoints)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_delete_checkpoint(
    experiment_id: String,
    checkpoint_name: String,
    _state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if checkpoint_name.contains("..") || checkpoint_name.contains('/') || checkpoint_name.contains('\\') {
        return Err("Invalid checkpoint name".to_string());
    }

    let artifact_dir = crate::core::config::get_artifact_dir(&experiment_id);
    let checkpoint_path = std::path::Path::new(&artifact_dir).join(&checkpoint_name);

    let canonical_checkpoint = checkpoint_path.canonicalize()
        .map_err(|e| format!("Invalid checkpoint path: {}", e))?;
    let canonical_dir = std::path::Path::new(&artifact_dir)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&artifact_dir));

    if !canonical_checkpoint.starts_with(&canonical_dir) {
        return Err("Checkpoint path is outside the allowed directory".to_string());
    }

    if !checkpoint_path.exists() {
        return Err(format!("Checkpoint not found: {}", checkpoint_name));
    }

    if checkpoint_path.is_dir() {
        std::fs::remove_dir_all(&checkpoint_path)
            .map_err(|e| format!("Cannot delete checkpoint directory: {}", e))?;
    } else {
        std::fs::remove_file(&checkpoint_path)
            .map_err(|e| format!("Cannot delete checkpoint file: {}", e))?;
    }

    Ok(())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_evaluate_model(
    experiment_id: String,
    test_data_path: String,
    _state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    if test_data_path.contains("..") || test_data_path.contains('~') {
        return Err("Test data path contains invalid traversal sequence".to_string());
    }

    let test_path = std::path::Path::new(&test_data_path);
    if !test_path.exists() {
        return Err(format!("Test data file not found: {}", test_data_path));
    }

    let id = ExperimentId::from_str(&experiment_id);
    let experiment = _state.experiment_repo.load(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Experiment not found".to_string())?;

    if !experiment.status.is_terminal() {
        return Err(format!("Cannot evaluate model for experiment in {} state, training must be completed first", experiment.status));
    }

    let artifact_dir = crate::core::config::get_artifact_dir(&experiment_id);

    let content = std::fs::read_to_string(&test_data_path)
        .map_err(|e| format!("Cannot read test data: {}", e))?;

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(content.as_bytes());

    let headers = reader.headers()
        .map_err(|e| format!("Cannot parse headers: {}", e))?
        .clone();

    let mut all_data: Vec<Vec<f32>> = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| format!("Cannot parse record: {}", e))?;
        let row: Vec<f32> = record.iter()
            .map(|v| v.parse::<f32>().unwrap_or(0.0))
            .collect();
        let has_nan = row.iter().any(|v| v.is_nan() || v.is_infinite());
        if !has_nan {
            all_data.push(row);
        }
    }

    let target_col_names = &experiment.config.target_columns;
    let feature_col_names = &experiment.config.feature_columns;

    let header_vec: Vec<String> = headers.iter().map(|h| h.to_string()).collect();

    let target_col_idx: Vec<usize> = target_col_names.iter()
        .filter_map(|name| header_vec.iter().position(|h| h == name))
        .collect();
    let feature_col_idx: Vec<usize> = feature_col_names.iter()
        .filter_map(|name| header_vec.iter().position(|h| h == name))
        .collect();

    let target_idx = target_col_idx.first().copied().unwrap_or(0);

    let num_features = if feature_col_idx.is_empty() {
        all_data.first()
            .map(|row| row.len().saturating_sub(target_col_idx.len().max(1)))
            .unwrap_or(1)
    } else {
        feature_col_idx.len()
    };

    let mut inputs: Vec<Vec<f32>> = Vec::new();
    let mut targets: Vec<f32> = Vec::new();

    for row in &all_data {
        if target_idx < row.len() {
            let features: Vec<f32> = if feature_col_idx.is_empty() {
                row.iter().enumerate()
                    .filter(|(i, _)| *i != target_idx)
                    .map(|(_, v)| *v)
                    .collect()
            } else {
                feature_col_idx.iter()
                    .filter_map(|&i| row.get(i).copied())
                    .collect()
            };
            if features.len() == num_features || feature_col_idx.is_empty() {
                inputs.push(features);
                targets.push(row[target_idx]);
            }
        }
    }

    if inputs.is_empty() {
        return Err("No valid test data found".to_string());
    }

    let inference_result = run_inference_dispatch(&experiment.config, &artifact_dir, &inputs)?;

    let is_classification = experiment.task_type == crate::types::TaskType::Classification;

    if is_classification {
        let target_ints: Vec<usize> = targets.iter().map(|t| t.round() as usize).collect();

        let max_reasonable_classes = 10000usize;
        let raw_max_class = target_ints.iter().max().copied().unwrap_or(0);
        if raw_max_class > max_reasonable_classes {
            return Err(format!("Target class value {} exceeds maximum reasonable class count ({}). Check your target column.", raw_max_class, max_reasonable_classes));
        }

        let mut correct = 0usize;
        let total = target_ints.len();
        let mut class_counts: HashMap<usize, usize> = HashMap::new();

        for (i, target) in target_ints.iter().enumerate() {
            *class_counts.entry(*target).or_insert(0) += 1;
            if let Some(&pc) = inference_result.predicted_classes.get(i) {
                if pc == *target {
                    correct += 1;
                }
            }
        }

        let num_classes = class_counts.keys().max().map(|m| m + 1).unwrap_or(2).min(max_reasonable_classes);
        let mut confusion_matrix: Vec<Vec<usize>> = vec![vec![0; num_classes]; num_classes];

        for (i, target) in target_ints.iter().enumerate() {
            if let Some(&pc) = inference_result.predicted_classes.get(i) {
                if pc < num_classes && *target < num_classes {
                    confusion_matrix[*target][pc] += 1;
                }
            }
        }

        let mut class_metrics = Vec::new();
        for c in 0..num_classes {
            let tp = confusion_matrix[c][c] as f64;
            let fp: f64 = (0..num_classes).filter(|&r| r != c).map(|r| confusion_matrix[r][c] as f64).sum();
            let fn_: f64 = (0..num_classes).filter(|&p| p != c).map(|p| confusion_matrix[c][p] as f64).sum();

            let precision = if tp + fp > 0.0 { tp / (tp + fp) } else { 0.0 };
            let recall = if tp + fn_ > 0.0 { tp / (tp + fn_) } else { 0.0 };
            let f1 = if precision + recall > 0.0 { 2.0 * precision * recall / (precision + recall) } else { 0.0 };

            class_metrics.push(serde_json::json!({
                "class": c,
                "precision": (precision * 10000.0).round() / 10000.0,
                "recall": (recall * 10000.0).round() / 10000.0,
                "f1_score": (f1 * 10000.0).round() / 10000.0,
                "support": class_counts.get(&c).copied().unwrap_or(0),
            }));
        }

        let accuracy = if total > 0 { correct as f64 / total as f64 } else { 0.0 };

        let macro_precision: f64 = if !class_metrics.is_empty() {
            class_metrics.iter().map(|m| m["precision"].as_f64().unwrap_or(0.0)).sum::<f64>() / class_metrics.len() as f64
        } else { 0.0 };
        let macro_recall: f64 = if !class_metrics.is_empty() {
            class_metrics.iter().map(|m| m["recall"].as_f64().unwrap_or(0.0)).sum::<f64>() / class_metrics.len() as f64
        } else { 0.0 };
        let macro_f1: f64 = if !class_metrics.is_empty() {
            class_metrics.iter().map(|m| m["f1_score"].as_f64().unwrap_or(0.0)).sum::<f64>() / class_metrics.len() as f64
        } else { 0.0 };

        Ok(serde_json::json!({
            "task_type": "classification",
            "total_samples": total,
            "accuracy": (accuracy * 10000.0).round() / 10000.0,
            "confusion_matrix": confusion_matrix,
            "class_metrics": class_metrics,
            "macro_precision": (macro_precision * 10000.0).round() / 10000.0,
            "macro_recall": (macro_recall * 10000.0).round() / 10000.0,
            "macro_f1": (macro_f1 * 10000.0).round() / 10000.0,
        }))
    } else {
        let mut total_se = 0.0f64;
        let mut total_ae = 0.0f64;
        let total = targets.len() as f64;

        for (i, target) in targets.iter().enumerate() {
            if let Some(pred) = inference_result.predictions.get(i) {
                let diff = (*pred as f64) - (*target as f64);
                total_se += diff * diff;
                total_ae += diff.abs();
            }
        }

        let mse = if total > 0.0 { total_se / total } else { 0.0 };
        let rmse = mse.sqrt();
        let mae = if total > 0.0 { total_ae / total } else { 0.0 };

        let mean_target: f64 = targets.iter().map(|t| *t as f64).sum::<f64>() / total;
        let total_ss: f64 = targets.iter().map(|t| (*t as f64 - mean_target).powi(2)).sum();
        let r_squared = if total_ss > 0.0 { 1.0 - total_se / total_ss } else { 0.0 };

        Ok(serde_json::json!({
            "task_type": "regression",
            "total_samples": targets.len(),
            "mse": (mse * 10000.0).round() / 10000.0,
            "rmse": (rmse * 10000.0).round() / 10000.0,
            "mae": (mae * 10000.0).round() / 10000.0,
            "r_squared": (r_squared * 10000.0).round() / 10000.0,
        }))
    }
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_batch_inference(
    experiment_id: String,
    input_data: Vec<Vec<f32>>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<crate::engine::burn_training::InferenceResult, String> {
    if input_data.is_empty() {
        return Err("Input data cannot be empty".to_string());
    }
    for (i, row) in input_data.iter().enumerate() {
        if row.is_empty() {
            return Err(format!("Input row {} is empty", i));
        }
        if row.iter().any(|v| v.is_nan() || v.is_infinite()) {
            return Err(format!("Input row {} contains NaN or infinite values", i));
        }
    }

    let id = ExperimentId::from_str(&experiment_id);
    let experiment = state.experiment_repo.load(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Experiment not found".to_string())?;

    if !experiment.status.is_terminal() {
        return Err(format!("Cannot run inference on experiment in {} state, training must be completed first", experiment.status));
    }

    let artifact_dir = crate::core::config::get_artifact_dir(&experiment_id);

    run_inference_dispatch(&experiment.config, &artifact_dir, &input_data)
}
