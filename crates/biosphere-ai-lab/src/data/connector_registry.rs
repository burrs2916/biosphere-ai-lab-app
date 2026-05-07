use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::{LabError, Result};
use super::connector::{
    ConnectConfig, ConnectorInfo, DataConnector, DiscoveredItem, ResolvedDataSource, ScanOptions,
};

pub struct DataConnectorRegistry {
    connectors: Arc<RwLock<HashMap<String, Arc<dyn DataConnector>>>>,
}

impl DataConnectorRegistry {
    pub fn new() -> Self {
        Self {
            connectors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, connector: impl DataConnector + 'static) {
        let id = connector.id().to_string();
        self.connectors.write().await.insert(id, Arc::new(connector));
    }

    pub async fn get(&self, id: &str) -> Option<Arc<dyn DataConnector>> {
        self.connectors.read().await.get(id).cloned()
    }

    pub async fn list(&self) -> Vec<ConnectorInfo> {
        self.connectors.read().await.values().map(|c| c.info()).collect()
    }

    pub async fn resolve_connector(&self, uri: &str) -> Option<Arc<dyn DataConnector>> {
        let connectors = self.connectors.read().await;
        for connector in connectors.values() {
            if connector.can_handle(uri) {
                return Some(connector.clone());
            }
        }
        None
    }

    pub async fn scan(&self, uri: &str, options: &ScanOptions) -> Result<Vec<DiscoveredItem>> {
        let connector = self.resolve_connector(uri).await;
        let available = self.connectors.read().await.keys().cloned().collect::<Vec<_>>().join(", ");
        let connector = connector
            .ok_or_else(|| LabError::Custom(format!(
                "No connector found for URI: {}. Available connectors: {}",
                uri, available
            )))?;

        let config = ConnectConfig {
            uri: uri.to_string(),
            params: HashMap::new(),
        };

        connector.scan(&config, options).await
    }

    pub async fn test_connection(&self, uri: &str) -> Result<bool> {
        let connector = self.resolve_connector(uri).await
            .ok_or_else(|| LabError::Custom(format!("No connector found for URI: {}", uri)))?;

        let config = ConnectConfig {
            uri: uri.to_string(),
            params: HashMap::new(),
        };

        connector.test_connection(&config).await
    }

    pub async fn resolve_item(&self, item: &DiscoveredItem) -> Result<ResolvedDataSource> {
        let connector = self.get(&item.connector_type).await
            .ok_or_else(|| LabError::Custom(format!("Connector not found: {}", item.connector_type)))?;

        connector.resolve_item(item).await
    }

    pub async fn scan_with_connector(&self, connector_id: &str, config: &ConnectConfig, options: &ScanOptions) -> Result<Vec<DiscoveredItem>> {
        let connector = self.get(connector_id).await
            .ok_or_else(|| LabError::Custom(format!("Connector not found: {}", connector_id)))?;

        connector.scan(config, options).await
    }
}

impl Default for DataConnectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
