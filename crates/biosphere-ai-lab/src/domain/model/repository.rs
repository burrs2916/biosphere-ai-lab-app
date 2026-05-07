use async_trait::async_trait;

use crate::core::Result;

use super::aggregate::{ModelRegistration, ModelId, ModelStatus};

#[async_trait]
pub trait ModelRepository: Send + Sync {
    async fn save(&self, model: &ModelRegistration) -> Result<()>;

    async fn load(&self, id: &ModelId) -> Result<Option<ModelRegistration>>;

    async fn list(&self, status: Option<ModelStatus>) -> Result<Vec<ModelRegistration>>;

    async fn list_by_name(&self, name: &str) -> Result<Vec<ModelRegistration>>;

    async fn delete(&self, id: &ModelId) -> Result<()>;

    async fn exists(&self, id: &ModelId) -> Result<bool>;
}
