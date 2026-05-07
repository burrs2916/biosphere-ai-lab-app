use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::PluginInfo;
use crate::types::PluginId;
use super::Task;

pub struct TaskRegistry {
    tasks: Arc<RwLock<HashMap<PluginId, Arc<dyn Task>>>>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, task: impl Task + 'static) {
        let id = task.id().clone();
        self.tasks.write().await.insert(id, Arc::new(task));
    }

    pub async fn get(&self, id: &PluginId) -> Option<Arc<dyn Task>> {
        self.tasks.read().await.get(id).cloned()
    }

    pub async fn find_by_id_str(&self, id: &str) -> Option<Arc<dyn Task>> {
        let tasks = self.tasks.read().await;
        for (pid, task) in tasks.iter() {
            if pid.as_str() == id {
                return Some(task.clone());
            }
        }
        None
    }

    pub async fn list(&self) -> Vec<PluginInfo> {
        self.tasks.read().await.values().map(|t| {
            PluginInfo {
                id: t.id().clone(),
                name: t.name().to_string(),
                version: t.version().to_string(),
                description: t.description().to_string(),
                plugin_kind: PluginKind::Task,
            }
        }).collect()
    }
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}

use crate::core::PluginKind;
