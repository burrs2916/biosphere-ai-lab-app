use serde::{Deserialize, Serialize};

use crate::types::PluginId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: PluginId,
    pub name: String,
    pub version: String,
    pub description: String,
    pub plugin_kind: PluginKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginKind {
    Engine,
    Task,
    Model,
    DataSource,
    Remote,
}

impl std::fmt::Display for PluginKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginKind::Engine => write!(f, "engine"),
            PluginKind::Task => write!(f, "task"),
            PluginKind::Model => write!(f, "model"),
            PluginKind::DataSource => write!(f, "data_source"),
            PluginKind::Remote => write!(f, "remote"),
        }
    }
}

pub trait Plugin: Send + Sync {
    fn id(&self) -> &PluginId;
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    fn plugin_kind(&self) -> PluginKind;

    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: self.id().clone(),
            name: self.name().to_string(),
            version: self.version().to_string(),
            description: self.description().to_string(),
            plugin_kind: self.plugin_kind(),
        }
    }
}
