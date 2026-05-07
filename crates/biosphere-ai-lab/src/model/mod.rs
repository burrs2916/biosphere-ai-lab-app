pub mod model_trait;
pub mod registry;
pub mod layers;
pub mod presets;

pub use model_trait::{ModelArch, ModelArchDef, LayerDescription};
pub use registry::ModelRegistry;
pub use layers::*;
pub use presets::*;
