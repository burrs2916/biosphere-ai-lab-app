use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::DataFormat;
use super::accessor::DataAccessor;
use super::csv_accessor::CsvDataAccessor;

pub struct DataAccessorRegistry {
    accessors: Arc<RwLock<HashMap<DataFormat, Arc<dyn DataAccessor>>>>,
}

impl DataAccessorRegistry {
    pub fn new() -> Self {
        let mut accessors: HashMap<DataFormat, Arc<dyn DataAccessor>> = HashMap::new();
        accessors.insert(DataFormat::Csv, Arc::new(CsvDataAccessor::new()));
        Self {
            accessors: Arc::new(RwLock::new(accessors)),
        }
    }

    pub async fn get(&self, format: &DataFormat) -> Option<Arc<dyn DataAccessor>> {
        self.accessors.read().await.get(format).cloned()
    }

    pub async fn register(&self, format: DataFormat, accessor: impl DataAccessor + 'static) {
        self.accessors.write().await.insert(format, Arc::new(accessor));
    }
}

impl Default for DataAccessorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
