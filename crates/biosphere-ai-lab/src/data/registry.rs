use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::PluginInfo;
use crate::core::PluginKind;
use crate::types::PluginId;
use super::DataSource;

pub struct DataSourceRegistry {
    sources: Arc<RwLock<HashMap<PluginId, Arc<dyn DataSource>>>>,
}

impl DataSourceRegistry {
    pub fn new() -> Self {
        Self {
            sources: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, source: impl DataSource + 'static) {
        let id = source.id().clone();
        self.sources.write().await.insert(id, Arc::new(source));
    }

    pub async fn get(&self, id: &PluginId) -> Option<Arc<dyn DataSource>> {
        self.sources.read().await.get(id).cloned()
    }

    pub async fn find_by_id_str(&self, id: &str) -> Option<Arc<dyn DataSource>> {
        let sources = self.sources.read().await;
        for (pid, source) in sources.iter() {
            if pid.as_str() == id {
                return Some(source.clone());
            }
        }
        None
    }

    pub async fn list(&self) -> Vec<PluginInfo> {
        self.sources.read().await.values().map(|s| {
            PluginInfo {
                id: s.id().clone(),
                name: s.name().to_string(),
                version: s.version().to_string(),
                description: s.description().to_string(),
                plugin_kind: PluginKind::DataSource,
            }
        }).collect()
    }
}

impl Default for DataSourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
