pub mod color;
pub mod gradient;
pub mod region;

pub use color::{ColorAnalysisResult, ColorCluster, ColorHistogram, ColorInfo};
pub use gradient::{GradientResult, EdgeResult};
pub use region::{RegionResult, RegionInfo};