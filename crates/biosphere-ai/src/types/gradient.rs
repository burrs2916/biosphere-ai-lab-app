use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientResult {
    pub magnitude: Vec<f32>,
    pub direction: Vec<f32>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeResult {
    pub edges: Vec<u8>,
    pub edge_count: usize,
    pub width: u32,
    pub height: u32,
}