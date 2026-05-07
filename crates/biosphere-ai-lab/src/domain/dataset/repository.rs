use async_trait::async_trait;

use crate::core::Result;

use super::aggregate::{Dataset, DatasetId, DatasetSplit, DatasetSummary, DatasetFilter};

#[async_trait]
pub trait DatasetRepository: Send + Sync {
    async fn save(&self, dataset: &Dataset) -> Result<()>;

    async fn load(&self, id: &DatasetId) -> Result<Option<Dataset>>;

    async fn list(&self, filter: &DatasetFilter) -> Result<Vec<DatasetSummary>>;

    async fn delete(&self, id: &DatasetId) -> Result<()>;

    async fn exists(&self, id: &DatasetId) -> Result<bool>;

    async fn find_by_digest(&self, digest: &str) -> Result<Option<Dataset>>;

    async fn find_by_name(&self, name: &str) -> Result<Vec<DatasetSummary>>;

    async fn save_split(&self, dataset_id: &DatasetId, split: &DatasetSplit) -> Result<()>;

    async fn load_splits(&self, dataset_id: &DatasetId) -> Result<Vec<DatasetSplit>>;

    async fn load_split(&self, dataset_id: &DatasetId, name: &str) -> Result<Option<DatasetSplit>>;

    async fn delete_split(&self, dataset_id: &DatasetId, name: &str) -> Result<()>;
}
