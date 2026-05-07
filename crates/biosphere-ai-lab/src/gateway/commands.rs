use std::sync::Arc;

use crate::core::config::DataLoadConfig;
use crate::gateway::state::LabState;
use crate::hardware::{HardwareInfo, TrainingRecommendation};
use crate::types::TaskType;

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_get_state(state: tauri::State<'_, Arc<LabState>>) -> Result<crate::gateway::state::LabStateSnapshot, String> {
    Ok(state.snapshot().await)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_engines(state: tauri::State<'_, Arc<LabState>>) -> Result<Vec<crate::core::PluginInfo>, String> {
    Ok(state.engine_registry.list().await)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_tasks(state: tauri::State<'_, Arc<LabState>>) -> Result<Vec<crate::core::PluginInfo>, String> {
    Ok(state.task_registry.list().await)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_models(state: tauri::State<'_, Arc<LabState>>) -> Result<Vec<crate::core::PluginInfo>, String> {
    Ok(state.model_registry.list().await)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_list_data_sources(state: tauri::State<'_, Arc<LabState>>) -> Result<Vec<crate::core::PluginInfo>, String> {
    Ok(state.data_source_registry.list().await)
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_get_hardware_info(state: tauri::State<'_, Arc<LabState>>) -> Result<HardwareInfo, String> {
    state.hardware_detector.detect().map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_get_recommendations(
    hardware: HardwareInfo,
    task_type: String,
    data_size: usize,
    state: tauri::State<'_, Arc<LabState>>,
) -> Result<TrainingRecommendation, String> {
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
    Ok(state.config_recommender.recommend(&hardware, task, data_size))
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_load_data(
    config: DataLoadConfig,
    state: tauri::State<'_, Arc<LabState>>,
) -> Result<crate::data::DatasetInfo, String> {
    let source = state.data_source_registry.find_by_id_str(&config.format.to_string())
        .await
        .ok_or_else(|| format!("Data source not found for format: {}", config.format))?;

    source.load(&config).await.map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_preview_data(
    config: DataLoadConfig,
    offset: Option<usize>,
    limit: Option<usize>,
    state: tauri::State<'_, Arc<LabState>>,
) -> Result<crate::data::DataPreview, String> {
    let source = state.data_source_registry.find_by_id_str(&config.format.to_string())
        .await
        .ok_or_else(|| format!("Data source not found for format: {}", config.format))?;

    let off = offset.unwrap_or(0);
    let lim = limit.unwrap_or(50);
    source.preview(&config, off, lim).await.map_err(|e| e.to_string())
}

#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn lab_get_model_arch(
    model_id: String,
    state: tauri::State<'_, Arc<LabState>>,
) -> Result<crate::model::ModelArchDef, String> {
    let model = state.model_registry.find_by_id_str(&model_id)
        .await
        .ok_or_else(|| format!("Model not found: {}", model_id))?;

    model.serialize().map_err(|e| e.to_string())
}
