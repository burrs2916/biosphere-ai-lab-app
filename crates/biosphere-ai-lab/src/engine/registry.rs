use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::PluginInfo;
use crate::types::PluginId;
use super::Engine;

pub struct EngineRegistry {
    engines: Arc<RwLock<HashMap<PluginId, Arc<dyn Engine>>>>,
}

impl EngineRegistry {
    pub fn new() -> Self {
        Self {
            engines: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, engine: impl Engine + 'static) {
        let id = engine.id().clone();
        self.engines.write().await.insert(id, Arc::new(engine));
    }

    pub async fn get(&self, id: &PluginId) -> Option<Arc<dyn Engine>> {
        self.engines.read().await.get(id).cloned()
    }

    pub async fn find_by_id_str(&self, id: &str) -> Option<Arc<dyn Engine>> {
        let engines = self.engines.read().await;
        for (pid, engine) in engines.iter() {
            if pid.as_str() == id {
                return Some(engine.clone());
            }
        }
        None
    }

    pub async fn list(&self) -> Vec<PluginInfo> {
        self.engines.read().await.values().map(|e| e.info()).collect()
    }
}

impl Default for EngineRegistry {
    fn default() -> Self {
        Self::new()
    }
}
