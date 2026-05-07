use crate::types::color::ColorAnalysisResult;
use crate::types::gradient::GradientResult;
use crate::types::region::RegionResult;

pub trait ColorAnalyzer {
    fn analyze_color(&self, rgba: &[u8], width: u32, height: u32) -> ColorAnalysisResult;
}

pub trait GradientAnalyzer {
    fn calculate_gradient(&self, gray: &[u8], width: u32, height: u32) -> GradientResult;
}

pub trait RegionAnalyzer {
    fn segment(&self, gray: &[u8], width: u32, height: u32, threshold: u32) -> RegionResult;
}

pub trait ImageAnalyzer: ColorAnalyzer + GradientAnalyzer + RegionAnalyzer {
    fn analyze_image(&self, rgba: &[u8], width: u32, height: u32) -> ImageAnalysisOutput;
}

#[derive(Debug, Clone)]
pub struct ImageAnalysisOutput {
    pub color: ColorAnalysisResult,
    pub gradient: GradientResult,
    pub region: RegionResult,
}