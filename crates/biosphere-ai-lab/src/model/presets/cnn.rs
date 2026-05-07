use crate::core::{LabError, Result};
use crate::types::{ArchType, PluginId, TensorShape};
use crate::model::model_trait::{LayerDescription, ModelArchDef, ModelArch};
use crate::model::layers::{ActivationType, Conv2DLayerConfig, DenseLayerConfig};

pub struct CnnModel {
    id: PluginId,
    channels: usize,
    height: usize,
    width: usize,
    num_classes: usize,
}

impl CnnModel {
    pub fn new(channels: usize, height: usize, width: usize, num_classes: usize) -> Self {
        Self {
            id: PluginId::new("cnn"),
            channels,
            height,
            width,
            num_classes,
        }
    }

    pub fn default_classifier(channels: usize, height: usize, width: usize, num_classes: usize) -> Self {
        Self::new(channels, height, width, num_classes)
    }
}

impl ModelArch for CnnModel {
    fn id(&self) -> &PluginId {
        &self.id
    }

    fn name(&self) -> &str {
        "CNN (Convolutional Neural Network)"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Convolutional neural network with BatchNorm and MaxPool for image classification"
    }

    fn architecture_type(&self) -> ArchType {
        ArchType::Cnn
    }

    fn input_shape(&self) -> TensorShape {
        TensorShape::new(vec![self.channels, self.height, self.width])
    }

    fn output_shape(&self) -> TensorShape {
        TensorShape::vector(self.num_classes)
    }

    fn parameter_count(&self) -> usize {
        let conv1 = self.channels * 32 * 9 + 32;
        let bn1 = 32 * 2;
        let conv2 = 32 * 64 * 9 + 64;
        let bn2 = 64 * 2;
        let fc1 = 64 * 128 + 128;
        let fc2 = 128 * self.num_classes + self.num_classes;
        conv1 + bn1 + conv2 + bn2 + fc1 + fc2
    }

    fn layer_descriptions(&self) -> Vec<LayerDescription> {
        vec![
            LayerDescription {
                layer_type: "Conv2D".to_string(),
                name: "conv1".to_string(),
                input_shape: TensorShape::new(vec![self.channels, self.height, self.width]),
                output_shape: TensorShape::new(vec![32, self.height, self.width]),
                params: self.channels * 32 * 9 + 32,
                config: serde_json::to_value(Conv2DLayerConfig {
                    in_channels: self.channels,
                    out_channels: 32,
                    kernel_size: 3,
                    stride: 1,
                    padding: 1,
                    activation: ActivationType::ReLU,
                }).unwrap_or(serde_json::Value::Null),
            },
            LayerDescription {
                layer_type: "BatchNorm".to_string(),
                name: "bn1".to_string(),
                input_shape: TensorShape::new(vec![32, self.height, self.width]),
                output_shape: TensorShape::new(vec![32, self.height, self.width]),
                params: 64,
                config: serde_json::json!({"num_features": 32}),
            },
            LayerDescription {
                layer_type: "MaxPool2D".to_string(),
                name: "pool1".to_string(),
                input_shape: TensorShape::new(vec![32, self.height, self.width]),
                output_shape: TensorShape::new(vec![32, self.height / 2, self.width / 2]),
                params: 0,
                config: serde_json::json!({"kernel_size": 2, "stride": 2}),
            },
            LayerDescription {
                layer_type: "Conv2D".to_string(),
                name: "conv2".to_string(),
                input_shape: TensorShape::new(vec![32, self.height / 2, self.width / 2]),
                output_shape: TensorShape::new(vec![64, self.height / 2, self.width / 2]),
                params: 32 * 64 * 9 + 64,
                config: serde_json::to_value(Conv2DLayerConfig {
                    in_channels: 32,
                    out_channels: 64,
                    kernel_size: 3,
                    stride: 1,
                    padding: 1,
                    activation: ActivationType::ReLU,
                }).unwrap_or(serde_json::Value::Null),
            },
            LayerDescription {
                layer_type: "BatchNorm".to_string(),
                name: "bn2".to_string(),
                input_shape: TensorShape::new(vec![64, self.height / 2, self.width / 2]),
                output_shape: TensorShape::new(vec![64, self.height / 2, self.width / 2]),
                params: 128,
                config: serde_json::json!({"num_features": 64}),
            },
            LayerDescription {
                layer_type: "MaxPool2D".to_string(),
                name: "pool2".to_string(),
                input_shape: TensorShape::new(vec![64, self.height / 2, self.width / 2]),
                output_shape: TensorShape::new(vec![64, self.height / 4, self.width / 4]),
                params: 0,
                config: serde_json::json!({"kernel_size": 2, "stride": 2}),
            },
            LayerDescription {
                layer_type: "AdaptiveAvgPool2D".to_string(),
                name: "adaptive_pool".to_string(),
                input_shape: TensorShape::new(vec![64, self.height / 4, self.width / 4]),
                output_shape: TensorShape::new(vec![64, 1, 1]),
                params: 0,
                config: serde_json::json!({"output_size": [1, 1]}),
            },
            LayerDescription {
                layer_type: "Dense".to_string(),
                name: "fc1".to_string(),
                input_shape: TensorShape::vector(64),
                output_shape: TensorShape::vector(128),
                params: 64 * 128 + 128,
                config: serde_json::to_value(DenseLayerConfig {
                    input_size: 64,
                    output_size: 128,
                    activation: ActivationType::ReLU,
                    use_bias: true,
                }).unwrap_or(serde_json::Value::Null),
            },
            LayerDescription {
                layer_type: "Dense".to_string(),
                name: "fc2".to_string(),
                input_shape: TensorShape::vector(128),
                output_shape: TensorShape::vector(self.num_classes),
                params: 128 * self.num_classes + self.num_classes,
                config: serde_json::to_value(DenseLayerConfig {
                    input_size: 128,
                    output_size: self.num_classes,
                    activation: ActivationType::Softmax,
                    use_bias: true,
                }).unwrap_or(serde_json::Value::Null),
            },
        ]
    }

    fn validate(&self) -> Result<()> {
        if self.channels == 0 {
            return Err(LabError::InvalidConfig("Channels must be > 0".to_string()));
        }
        if self.height == 0 || self.width == 0 {
            return Err(LabError::InvalidConfig("Image dimensions must be > 0".to_string()));
        }
        if self.num_classes == 0 {
            return Err(LabError::InvalidConfig("Number of classes must be > 0".to_string()));
        }
        Ok(())
    }

    fn serialize(&self) -> Result<ModelArchDef> {
        Ok(ModelArchDef {
            id: self.id.clone(),
            arch_type: self.architecture_type(),
            layers: self.layer_descriptions(),
            total_params: self.parameter_count(),
            input_shape: self.input_shape(),
            output_shape: self.output_shape(),
        })
    }
}
