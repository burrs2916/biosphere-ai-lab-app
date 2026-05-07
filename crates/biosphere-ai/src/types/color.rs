use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorInfo {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub count: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorHistogram {
    pub red: Vec<u64>,
    pub green: Vec<u64>,
    pub blue: Vec<u64>,
    pub gray: Vec<u64>,
}

impl Default for ColorHistogram {
    fn default() -> Self {
        Self {
            red: vec![0; 256],
            green: vec![0; 256],
            blue: vec![0; 256],
            gray: vec![0; 256],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorCluster {
    pub center_r: u8,
    pub center_g: u8,
    pub center_b: u8,
    pub colors: Vec<ColorInfo>,
    pub total_count: u64,
    pub total_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorFeatures {
    pub color_temperature: f64,
    pub temperature_category: String,
    pub dominant_hue: String,
    pub color_harmony_score: f64,
    pub is_grayscale: bool,
    pub color_entropy: f64,
    pub saturation_avg: f64,
    pub brightness_avg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorAnalysisResult {
    pub histogram: ColorHistogram,
    pub unique_colors: Vec<ColorInfo>,
    pub color_clusters: Vec<ColorCluster>,
    pub dominant_colors: Vec<ColorInfo>,
    pub accent_colors: Vec<ColorInfo>,
    pub color_count: usize,
    pub features: ColorFeatures,
}