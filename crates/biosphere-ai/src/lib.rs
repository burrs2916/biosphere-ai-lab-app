#![recursion_limit = "256"]

pub mod prelude;
pub mod types;
pub mod image_analysis;

pub use prelude::*;
pub use image_analysis::calculate_gradient_fast;
pub use image_analysis::analyze_colors_fast;
pub use image_analysis::segment_regions;
pub use image_analysis::GradientResult;
pub use image_analysis::gradient::BurnGradientCalculator;
pub use image_analysis::color::BurnColorAnalyzer;