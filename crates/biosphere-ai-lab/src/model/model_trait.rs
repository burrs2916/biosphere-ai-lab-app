use serde::{Deserialize, Serialize};

use crate::core::Result;
use crate::types::{ArchType, PluginId, TensorShape};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerDescription {
    pub layer_type: String,
    pub name: String,
    pub input_shape: TensorShape,
    pub output_shape: TensorShape,
    pub params: usize,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelArchDef {
    pub id: PluginId,
    pub arch_type: ArchType,
    pub layers: Vec<LayerDescription>,
    pub total_params: usize,
    pub input_shape: TensorShape,
    pub output_shape: TensorShape,
}

pub trait ModelArch: Send + Sync {
    fn id(&self) -> &PluginId;
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    fn architecture_type(&self) -> ArchType;
    fn input_shape(&self) -> TensorShape;
    fn output_shape(&self) -> TensorShape;
    fn parameter_count(&self) -> usize;
    fn layer_descriptions(&self) -> Vec<LayerDescription>;
    fn validate(&self) -> Result<()>;
    fn serialize(&self) -> Result<ModelArchDef>;
}
