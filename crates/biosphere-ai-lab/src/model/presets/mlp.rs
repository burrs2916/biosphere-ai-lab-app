use crate::core::{LabError, Result};
use crate::types::{ArchType, PluginId, TensorShape};
use crate::model::model_trait::{LayerDescription, ModelArchDef, ModelArch};
use crate::model::layers::{ActivationType, DenseLayerConfig};

pub struct MlpModel {
    id: PluginId,
    input_size: usize,
    hidden_sizes: Vec<usize>,
    output_size: usize,
    activation: ActivationType,
}

impl MlpModel {
    pub fn new(
        input_size: usize,
        hidden_sizes: Vec<usize>,
        output_size: usize,
        activation: ActivationType,
    ) -> Self {
        Self {
            id: PluginId::new("mlp"),
            input_size,
            hidden_sizes,
            output_size,
            activation,
        }
    }

    pub fn default_classifier(num_features: usize, num_classes: usize) -> Self {
        Self::new(
            num_features,
            vec![128, 64],
            num_classes,
            ActivationType::ReLU,
        )
    }
}

impl ModelArch for MlpModel {
    fn id(&self) -> &PluginId {
        &self.id
    }

    fn name(&self) -> &str {
        "MLP (Multi-Layer Perceptron)"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Fully connected feed-forward neural network"
    }

    fn architecture_type(&self) -> ArchType {
        ArchType::Mlp
    }

    fn input_shape(&self) -> TensorShape {
        TensorShape::vector(self.input_size)
    }

    fn output_shape(&self) -> TensorShape {
        TensorShape::vector(self.output_size)
    }

    fn parameter_count(&self) -> usize {
        let mut total = 0usize;
        let mut prev = self.input_size;
        for &hidden in &self.hidden_sizes {
            total += prev * hidden + hidden;
            prev = hidden;
        }
        total += prev * self.output_size + self.output_size;
        total
    }

    fn layer_descriptions(&self) -> Vec<LayerDescription> {
        let mut layers = Vec::new();
        let mut prev = self.input_size;

        for (i, &hidden) in self.hidden_sizes.iter().enumerate() {
            layers.push(LayerDescription {
                layer_type: "Dense".to_string(),
                name: format!("hidden_{}", i),
                input_shape: TensorShape::vector(prev),
                output_shape: TensorShape::vector(hidden),
                params: prev * hidden + hidden,
                config: serde_json::to_value(DenseLayerConfig {
                    input_size: prev,
                    output_size: hidden,
                    activation: self.activation.clone(),
                    use_bias: true,
                }).unwrap_or(serde_json::Value::Null),
            });
            prev = hidden;
        }

        layers.push(LayerDescription {
            layer_type: "Dense".to_string(),
            name: "output".to_string(),
            input_shape: TensorShape::vector(prev),
            output_shape: TensorShape::vector(self.output_size),
            params: prev * self.output_size + self.output_size,
            config: serde_json::to_value(DenseLayerConfig {
                input_size: prev,
                output_size: self.output_size,
                activation: ActivationType::Softmax,
                use_bias: true,
            }).unwrap_or(serde_json::Value::Null),
        });

        layers
    }

    fn validate(&self) -> Result<()> {
        if self.input_size == 0 {
            return Err(LabError::InvalidConfig("Input size must be > 0".to_string()));
        }
        if self.output_size == 0 {
            return Err(LabError::InvalidConfig("Output size must be > 0".to_string()));
        }
        if self.hidden_sizes.is_empty() {
            return Err(LabError::InvalidConfig("Must have at least one hidden layer".to_string()));
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
