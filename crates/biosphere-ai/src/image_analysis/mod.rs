pub mod traits;
pub mod gradient;
pub mod color;
pub mod region;

pub mod prelude {
    pub use crate::types::color::{ColorAnalysisResult, ColorCluster, ColorHistogram, ColorInfo};
    pub use crate::types::gradient::{GradientResult, EdgeResult};
    pub use crate::types::region::{RegionResult, RegionInfo};
    pub use crate::image_analysis::traits::*;
}

pub use gradient::GradientResult;
pub use color::BurnColorAnalyzer;
pub use color::analyze_colors_fast;
pub use gradient::calculate_gradient_fast;
pub use region::segment_regions;