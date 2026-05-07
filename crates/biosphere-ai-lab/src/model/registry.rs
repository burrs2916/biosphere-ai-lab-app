use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::PluginInfo;
use crate::core::PluginKind;
use crate::types::PluginId;
use super::ModelArch;

pub struct ModelRegistry {
    models: Arc<RwLock<HashMap<PluginId, Arc<dyn ModelArch>>>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, model: impl ModelArch + 'static) {
        let id = model.id().clone();
        self.models.write().await.insert(id, Arc::new(model));
    }

    pub async fn get(&self, id: &PluginId) -> Option<Arc<dyn ModelArch>> {
        self.models.read().await.get(id).cloned()
    }

    pub async fn find_by_id_str(&self, id: &str) -> Option<Arc<dyn ModelArch>> {
        let models = self.models.read().await;
        for (pid, model) in models.iter() {
            if pid.as_str() == id {
                return Some(model.clone());
            }
        }
        None
    }

    pub async fn list(&self) -> Vec<PluginInfo> {
        self.models.read().await.values().map(|m| {
            PluginInfo {
                id: m.id().clone(),
                name: m.name().to_string(),
                version: m.version().to_string(),
                description: m.description().to_string(),
                plugin_kind: PluginKind::Model,
            }
        }).collect()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}
