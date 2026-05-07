use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub general: GeneralSettings,
    pub training: TrainingSettings,
    pub storage: StorageSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub language: String,
    pub theme: String,
    pub log_level: String,
    pub auto_refresh_interval: u64,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language: "zh-CN".to_string(),
            theme: "dark".to_string(),
            log_level: "info".to_string(),
            auto_refresh_interval: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSettings {
    pub default_compute_backend: String,
    pub default_engine: String,
    pub max_concurrent_experiments: usize,
    pub auto_checkpoint: bool,
    pub checkpoint_interval: usize,
}

impl Default for TrainingSettings {
    fn default() -> Self {
        Self {
            default_compute_backend: "wgpu".to_string(),
            default_engine: "burn".to_string(),
            max_concurrent_experiments: 1,
            auto_checkpoint: false,
            checkpoint_interval: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSettings {
    pub data_directory: String,
    pub model_directory: String,
    pub checkpoint_directory: String,
    pub max_storage_gb: f64,
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            data_directory: "./data".to_string(),
            model_directory: "./models".to_string(),
            checkpoint_directory: "./checkpoints".to_string(),
            max_storage_gb: 10.0,
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            general: GeneralSettings::default(),
            training: TrainingSettings::default(),
            storage: StorageSettings::default(),
        }
    }
}

impl AppSettings {
    pub fn to_flat_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("general.language".to_string(), self.general.language.clone());
        map.insert("general.theme".to_string(), self.general.theme.clone());
        map.insert("general.log_level".to_string(), self.general.log_level.clone());
        map.insert("general.auto_refresh_interval".to_string(), self.general.auto_refresh_interval.to_string());
        map.insert("training.default_compute_backend".to_string(), self.training.default_compute_backend.clone());
        map.insert("training.default_engine".to_string(), self.training.default_engine.clone());
        map.insert("training.max_concurrent_experiments".to_string(), self.training.max_concurrent_experiments.to_string());
        map.insert("training.auto_checkpoint".to_string(), self.training.auto_checkpoint.to_string());
        map.insert("training.checkpoint_interval".to_string(), self.training.checkpoint_interval.to_string());
        map.insert("storage.data_directory".to_string(), self.storage.data_directory.clone());
        map.insert("storage.model_directory".to_string(), self.storage.model_directory.clone());
        map.insert("storage.checkpoint_directory".to_string(), self.storage.checkpoint_directory.clone());
        map.insert("storage.max_storage_gb".to_string(), self.storage.max_storage_gb.to_string());
        map
    }

    pub fn from_flat_map(map: &HashMap<String, String>) -> Self {
        let mut settings = Self::default();

        if let Some(v) = map.get("general.language") { settings.general.language = v.clone(); }
        if let Some(v) = map.get("general.theme") { settings.general.theme = v.clone(); }
        if let Some(v) = map.get("general.log_level") { settings.general.log_level = v.clone(); }
        if let Some(v) = map.get("general.auto_refresh_interval") { settings.general.auto_refresh_interval = v.parse().unwrap_or(5); }
        if let Some(v) = map.get("training.default_compute_backend") { settings.training.default_compute_backend = v.clone(); }
        if let Some(v) = map.get("training.default_engine") { settings.training.default_engine = v.clone(); }
        if let Some(v) = map.get("training.max_concurrent_experiments") { settings.training.max_concurrent_experiments = v.parse().unwrap_or(1); }
        if let Some(v) = map.get("training.auto_checkpoint") { settings.training.auto_checkpoint = v == "true"; }
        if let Some(v) = map.get("training.checkpoint_interval") { settings.training.checkpoint_interval = v.parse().unwrap_or(10); }
        if let Some(v) = map.get("storage.data_directory") { settings.storage.data_directory = v.clone(); }
        if let Some(v) = map.get("storage.model_directory") { settings.storage.model_directory = v.clone(); }
        if let Some(v) = map.get("storage.checkpoint_directory") { settings.storage.checkpoint_directory = v.clone(); }
        if let Some(v) = map.get("storage.max_storage_gb") { settings.storage.max_storage_gb = v.parse().unwrap_or(10.0); }

        settings
    }
}
