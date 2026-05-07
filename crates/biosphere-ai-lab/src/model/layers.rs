use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerDef {
    pub layer_type: LayerType,
    pub name: String,
    pub input_size: Option<usize>,
    pub output_size: Option<usize>,
    pub activation: Option<ActivationType>,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerType {
    Dense,
    Conv2D,
    Conv1D,
    MaxPool2D,
    AvgPool2D,
    Dropout,
    BatchNorm,
    LayerNorm,
    Flatten,
    Reshape,
    Embedding,
    Lstm,
    Gru,
    Attention,
    ResidualBlock,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivationType {
    ReLU,
    Sigmoid,
    Tanh,
    Softmax,
    LeakyReLU { alpha: f64 },
    ELU { alpha: f64 },
    GELU,
    Swish,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenseLayerConfig {
    pub input_size: usize,
    pub output_size: usize,
    pub activation: ActivationType,
    pub use_bias: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conv2DLayerConfig {
    pub in_channels: usize,
    pub out_channels: usize,
    pub kernel_size: usize,
    pub stride: usize,
    pub padding: usize,
    pub activation: ActivationType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropoutLayerConfig {
    pub rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LstmLayerConfig {
    pub input_size: usize,
    pub hidden_size: usize,
    pub num_layers: usize,
    pub bidirectional: bool,
}
