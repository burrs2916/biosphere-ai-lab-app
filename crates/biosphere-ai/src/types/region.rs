use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionInfo {
    pub id: u32,
    pub avg_r: u8,
    pub avg_g: u8,
    pub avg_b: u8,
    pub pixel_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionResult {
    pub regions: Vec<RegionInfo>,
    pub region_count: usize,
    pub region_map: Vec<u32>,
    pub width: u32,
    pub height: u32,
}