use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use crate::types::color::{ColorAnalysisResult, ColorCluster, ColorHistogram, ColorInfo};

pub struct BurnColorAnalyzer;

pub struct ImageFeatures {
    pub width: u32,
    pub height: u32,
    pub pixel_count: u64,
    pub aspect_ratio: f64,
    pub unique_color_count: usize,
    pub color_density: f64,
    pub fast_contrast: f64,
    pub fast_brightness: f64,
}

struct AnalysisParams {
    target_dominant_colors: usize,
    initial_color_distance: f64,
    cluster_count: usize,
    max_iterations: usize,
    sample_size: usize,
}

impl BurnColorAnalyzer {
    pub fn new() -> Self {
        Self
    }
    
    pub fn quick_evaluate_features(rgba: &[u8], width: u32, height: u32) -> ImageFeatures {
        let pixel_count = (width * height) as u64;
        let aspect_ratio = width as f64 / height as f64;
        
        let sample_rate = 0.01;
        let sample_step = (1.0 / sample_rate) as usize;
        let mut sampled_colors: Vec<(u8, u8, u8)> = Vec::new();
        let mut brightness_sum = 0u64;
        let mut brightness_count = 0u64;
        
        for i in (0..rgba.len()).step_by(4 * sample_step) {
            if i + 2 < rgba.len() {
                let r = rgba[i];
                let g = rgba[i + 1];
                let b = rgba[i + 2];
                sampled_colors.push((r, g, b));
                brightness_sum += (r as u64 + g as u64 + b as u64) / 3;
                brightness_count += 1;
            }
        }
        
        let mut color_set: HashSet<u32> = HashSet::new();
        for (r, g, b) in &sampled_colors {
            let key = ((*r as u32) << 16) | ((*g as u32) << 8) | (*b as u32);
            color_set.insert(key);
        }
        
        let estimated_unique = (color_set.len() as f64 / sample_rate) as usize;
        let estimated_unique = estimated_unique.min(pixel_count as usize);
        
        let color_density = estimated_unique as f64 / pixel_count as f64;
        
        let avg_brightness = if brightness_count > 0 {
            brightness_sum as f64 / brightness_count as f64
        } else {
            128.0
        };
        
        let mut contrast_sum = 0.0;
        let mut contrast_count = 0;
        for chunk in sampled_colors.chunks(2) {
            if chunk.len() == 2 {
                let (r1, g1, b1) = chunk[0];
                let (r2, g2, b2) = chunk[1];
                let diff = ((r1 as i32 - r2 as i32).abs() + 
                           (g1 as i32 - g2 as i32).abs() + 
                           (b1 as i32 - b2 as i32).abs()) as f64 / 3.0;
                contrast_sum += diff;
                contrast_count += 1;
            }
        }
        let fast_contrast = if contrast_count > 0 {
            contrast_sum / contrast_count as f64
        } else {
            30.0
        };
        
        ImageFeatures {
            width,
            height,
            pixel_count,
            aspect_ratio,
            unique_color_count: estimated_unique,
            color_density,
            fast_contrast,
            fast_brightness: avg_brightness,
        }
    }
    
    fn calculate_analysis_params(features: &ImageFeatures) -> AnalysisParams {
        let target_dominant_colors = 10;
        
        let base_distance = 35.0;
        let density_factor = if features.color_density > 0.5 {
            1.2
        } else if features.color_density > 0.3 {
            1.0
        } else if features.color_density > 0.1 {
            0.9
        } else {
            0.8
        };
        
        let contrast_factor = if features.fast_contrast > 60.0 {
            1.1
        } else if features.fast_contrast > 40.0 {
            1.0
        } else if features.fast_contrast > 20.0 {
            0.95
        } else {
            0.85
        };
        
        let resolution_factor = if features.pixel_count > 1_000_000 {
            1.1
        } else if features.pixel_count > 500_000 {
            1.0
        } else if features.pixel_count > 200_000 {
            0.95
        } else {
            0.9
        };
        
        let initial_color_distance = base_distance * density_factor * contrast_factor * resolution_factor;
        
        let cluster_count = if features.unique_color_count > 100000 {
            10
        } else if features.unique_color_count > 50000 {
            8
        } else if features.unique_color_count > 10000 {
            6
        } else {
            5
        };
        
        let max_iterations = if features.pixel_count > 1_000_000 {
            25
        } else if features.pixel_count > 500_000 {
            40
        } else {
            60
        };
        
        let sample_size = if features.pixel_count > 1_000_000 {
            400
        } else if features.pixel_count > 500_000 {
            600
        } else {
            800
        };
        
        eprintln!("[Burn-AI] ========== 动态参数计算 ==========");
        eprintln!("[Burn-AI] 图像特征:");
        eprintln!("[Burn-AI]   - 分辨率: {}x{} ({}万像素)", features.width, features.height, features.pixel_count / 10000);
        eprintln!("[Burn-AI]   - 宽高比: {:.2}", features.aspect_ratio);
        eprintln!("[Burn-AI]   - 估计唯一颜色: {} 种", features.unique_color_count);
        eprintln!("[Burn-AI]   - 颜色密度: {:.4}", features.color_density);
        eprintln!("[Burn-AI]   - 快速对比度: {:.1}", features.fast_contrast);
        eprintln!("[Burn-AI]   - 快速亮度: {:.1}", features.fast_brightness);
        eprintln!("[Burn-AI] 分析参数:");
        eprintln!("[Burn-AI]   - 目标主色调: {} 种", target_dominant_colors);
        eprintln!("[Burn-AI]   - 初始颜色距离: {:.2}", initial_color_distance);
        eprintln!("[Burn-AI]   - 聚类数量: {}", cluster_count);
        eprintln!("[Burn-AI]   - 最大迭代: {}", max_iterations);
        eprintln!("[Burn-AI]   - 采样大小: {}", sample_size);
        
        AnalysisParams {
            target_dominant_colors,
            initial_color_distance,
            cluster_count,
            max_iterations,
            sample_size,
        }
    }
    
    fn analyze_color_distribution(colors: &[ColorInfo]) -> (f64, f64, f64) {
        if colors.is_empty() {
            return (0.0, 0.0, 0.0);
        }
        
        let sample_size = colors.len().min(1000);
        let mut distances = Vec::new();
        
        for i in 0..sample_size {
            for j in (i + 1)..sample_size.min(i + 50) {
                if j < colors.len() {
                    let dist = Self::color_distance(
                        colors[i].r, colors[i].g, colors[i].b,
                        colors[j].r, colors[j].g, colors[j].b,
                    );
                    distances.push(dist);
                }
            }
        }
        
        if distances.is_empty() {
            return (0.0, 0.0, 0.0);
        }
        
        distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let min_dist = distances[0];
        let max_dist = distances[distances.len() - 1];
        let median_dist = distances[distances.len() / 2];
        
        (min_dist, median_dist, max_dist)
    }
    
    fn calculate_adaptive_threshold(colors: &[ColorInfo], target_count: usize) -> f64 {
        let (min_dist, median_dist, max_dist) = Self::analyze_color_distribution(colors);
        
        eprintln!("[Burn-AI]   颜色分布分析: 最小距离={:.2}, 中位数={:.2}, 最大距离={:.2}", 
            min_dist, median_dist, max_dist);
        
        let coverage_ratio = target_count as f64 / colors.len().min(2000) as f64;
        
        let base_threshold = if coverage_ratio > 0.1 {
            median_dist * 0.3
        } else if coverage_ratio > 0.05 {
            median_dist * 0.5
        } else {
            median_dist * 0.7
        };
        
        let adjusted_threshold = base_threshold.max(20.0).min(max_dist * 0.8);
        
        eprintln!("[Burn-AI]   自适应阈值: {:.2} (覆盖率: {:.2}%)", 
            adjusted_threshold, coverage_ratio * 100.0);
        
        adjusted_threshold
    }
    
    fn select_dominant_colors_smart(colors: &[ColorInfo], target_count: usize) -> Vec<ColorInfo> {
        if colors.is_empty() {
            return Vec::new();
        }
        
        eprintln!("[Burn-AI]   [预聚类] 开始合并相似颜色...");
        let merged_colors = Self::merge_similar_colors(colors, 50.0);
        eprintln!("[Burn-AI]   [预聚类] 完成，从 {} 种颜色合并为 {} 种代表颜色", 
            colors.len().min(3000), merged_colors.len());
        
        let adaptive_threshold = Self::calculate_adaptive_threshold(&merged_colors, target_count);
        
        let mut result: Vec<ColorInfo> = Vec::new();
        let mut used_indices: Vec<bool> = vec![false; merged_colors.len()];
        
        eprintln!("[Burn-AI]   [阶段1] 使用自适应阈值 {:.2} 选择颜色", adaptive_threshold);
        
        while result.len() < target_count {
            let mut best_idx: Option<usize> = None;
            let mut best_score: f64 = 0.0;
            
            for (idx, color) in merged_colors.iter().enumerate() {
                if used_indices[idx] {
                    continue;
                }
                
                let mut min_distance = f64::MAX;
                for selected in &result {
                    let distance = Self::color_distance_lab(
                        color.r, color.g, color.b,
                        selected.r, selected.g, selected.b,
                    );
                    min_distance = min_distance.min(distance);
                }
                
                if min_distance < adaptive_threshold && !result.is_empty() {
                    continue;
                }
                
                let frequency_score = (color.count as f64).ln();
                let diversity_score = if result.is_empty() {
                    1.0
                } else {
                    min_distance / adaptive_threshold
                };
                
                let score = frequency_score * 0.6 + diversity_score * 0.4;
                
                if score > best_score {
                    best_score = score;
                    best_idx = Some(idx);
                }
            }
            
            if let Some(idx) = best_idx {
                result.push(merged_colors[idx].clone());
                used_indices[idx] = true;
            } else {
                break;
            }
        }
        
        eprintln!("[Burn-AI]   [阶段1] 完成，已选择 {} 种颜色", result.len());
        
        if result.len() < target_count {
            let relaxed_threshold = adaptive_threshold * 0.5;
            eprintln!("[Burn-AI]   [阶段2] 降低阈值至 {:.2} 继续选择", relaxed_threshold);
            
            while result.len() < target_count {
                let mut best_idx: Option<usize> = None;
                let mut best_score: f64 = 0.0;
                
                for (idx, color) in merged_colors.iter().enumerate() {
                    if used_indices[idx] {
                        continue;
                    }
                    
                    let mut min_distance = f64::MAX;
                    for selected in &result {
                        let distance = Self::color_distance_lab(
                            color.r, color.g, color.b,
                            selected.r, selected.g, selected.b,
                        );
                        min_distance = min_distance.min(distance);
                    }
                    
                    if min_distance < relaxed_threshold {
                        continue;
                    }
                    
                    let frequency_score = (color.count as f64).ln();
                    let diversity_score = min_distance / relaxed_threshold;
                    
                    let score = frequency_score * 0.7 + diversity_score * 0.3;
                    
                    if score > best_score {
                        best_score = score;
                        best_idx = Some(idx);
                    }
                }
                
                if let Some(idx) = best_idx {
                    result.push(merged_colors[idx].clone());
                    used_indices[idx] = true;
                } else {
                    break;
                }
            }
            
            eprintln!("[Burn-AI]   [阶段2] 完成，已选择 {} 种颜色", result.len());
        }
        
        if result.len() < target_count {
            eprintln!("[Burn-AI]   [阶段3] 按频率补充剩余颜色（保持颜色差异）");
            
            let mut remaining: Vec<(usize, &ColorInfo)> = merged_colors
                .iter()
                .enumerate()
                .filter(|(idx, _)| !used_indices[*idx])
                .collect();
            
            remaining.sort_by(|a, b| b.1.count.cmp(&a.1.count));
            
            for (_, color) in remaining {
                let mut min_distance = f64::MAX;
                for selected in &result {
                    let distance = Self::color_distance_lab(
                        color.r, color.g, color.b,
                        selected.r, selected.g, selected.b,
                    );
                    min_distance = min_distance.min(distance);
                }
                
                if min_distance >= 15.0 {
                    result.push(color.clone());
                    eprintln!("[Burn-AI]     补充颜色: RGB({}, {}, {}) 距离={:.2}", 
                        color.r, color.g, color.b, min_distance);
                }
                
                if result.len() >= target_count {
                    break;
                }
            }
            
            eprintln!("[Burn-AI]   [阶段3] 完成，已选择 {} 种颜色", result.len());
        }
        
        result
    }
    
    fn merge_similar_colors(colors: &[ColorInfo], threshold: f64) -> Vec<ColorInfo> {
        if colors.is_empty() {
            return Vec::new();
        }
        
        let candidate_size = colors.len().min(3000);
        let candidates: &[ColorInfo] = &colors[..candidate_size];
        
        let mut clusters: Vec<Vec<usize>> = Vec::new();
        let mut assigned: Vec<bool> = vec![false; candidates.len()];
        
        for (idx, color) in candidates.iter().enumerate() {
            if assigned[idx] {
                continue;
            }
            
            let mut cluster = vec![idx];
            assigned[idx] = true;
            
            for (other_idx, other_color) in candidates.iter().enumerate() {
                if assigned[other_idx] {
                    continue;
                }
                
                let distance = Self::color_distance_lab(
                    color.r, color.g, color.b,
                    other_color.r, other_color.g, other_color.b,
                );
                
                if distance < threshold {
                    cluster.push(other_idx);
                    assigned[other_idx] = true;
                }
            }
            
            clusters.push(cluster);
        }
        
        let mut merged: Vec<ColorInfo> = clusters
            .into_iter()
            .map(|cluster| {
                let mut total_count = 0u64;
                let mut best_color = &candidates[cluster[0]];
                
                for &idx in &cluster {
                    total_count += candidates[idx].count;
                    if candidates[idx].count > best_color.count {
                        best_color = &candidates[idx];
                    }
                }
                
                ColorInfo {
                    r: best_color.r,
                    g: best_color.g,
                    b: best_color.b,
                    count: total_count,
                    percentage: total_count as f64 / colors.iter().map(|c| c.count).sum::<u64>() as f64,
                }
            })
            .collect();
        
        merged.sort_by(|a, b| b.count.cmp(&a.count));
        
        merged
    }
    
    fn select_colors_two_stage(colors: &[ColorInfo], target_count: usize) -> (Vec<ColorInfo>, Vec<ColorInfo>) {
        if colors.is_empty() {
            return (Vec::new(), Vec::new());
        }
        
        eprintln!("[Burn-AI]   [预聚类] 开始合并相似颜色...");
        let merged_colors = Self::merge_similar_colors(colors, 50.0);
        eprintln!("[Burn-AI]   [预聚类] 完成，从 {} 种颜色合并为 {} 种代表颜色", 
            colors.len().min(3000), merged_colors.len());
        
        let mut dominant_colors = Vec::new();
        let mut used_color_keys: std::collections::HashSet<u32> = std::collections::HashSet::new();
        
        eprintln!("[Burn-AI]   [阶段1] 选择前{}个高频颜色", target_count);
        for color in merged_colors.iter().take(target_count) {
            dominant_colors.push(color.clone());
            used_color_keys.insert(((color.r as u32) << 16) | ((color.g as u32) << 8) | (color.b as u32));
        }
        
        if dominant_colors.len() < target_count {
            eprintln!("[Burn-AI]   [阶段1补充] 预聚类颜色不足，从原始列表补充");
            for color in colors.iter() {
                let key = ((color.r as u32) << 16) | ((color.g as u32) << 8) | (color.b as u32);
                if !used_color_keys.contains(&key) {
                    let mut is_different = true;
                    for selected in &dominant_colors {
                        let distance = Self::color_distance(
                            color.r, color.g, color.b,
                            selected.r, selected.g, selected.b,
                        );
                        if distance < 30.0 {
                            is_different = false;
                            break;
                        }
                    }
                    if is_different {
                        dominant_colors.push(color.clone());
                        used_color_keys.insert(key);
                        if dominant_colors.len() >= target_count {
                            break;
                        }
                    }
                }
            }
        }
        eprintln!("[Burn-AI]   [阶段1] 完成，已选择 {} 种高频颜色", dominant_colors.len());
        
        eprintln!("[Burn-AI]   [阶段2] 选择独特颜色（显著性评分算法）");
        
        let mut color_salience: Vec<(ColorInfo, f64)> = Vec::new();
        
        for color in colors.iter().take(50000) {
            let key = ((color.r as u32) << 16) | ((color.g as u32) << 8) | (color.b as u32);
            if used_color_keys.contains(&key) {
                continue;
            }
            
            let (h, s, v) = Self::rgb_to_hsv(color.r, color.g, color.b);
            
            if s < 0.3 {
                continue;
            }
            
            let salience = Self::calculate_color_salience(color, &dominant_colors);
            
            if salience > 50.0 {
                color_salience.push((color.clone(), salience));
            }
        }
        
        color_salience.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        let mut accent_colors: Vec<ColorInfo> = Vec::new();
        
        for (color, salience) in color_salience {
            let mut min_distance_to_accent = f64::MAX;
            for selected in &accent_colors {
                let distance = Self::color_distance_lab(
                    color.r, color.g, color.b,
                    selected.r, selected.g, selected.b,
                );
                min_distance_to_accent = min_distance_to_accent.min(distance);
            }
            
            if accent_colors.is_empty() || min_distance_to_accent > 20.0 {
                let (h, _, _) = Self::rgb_to_hsv(color.r, color.g, color.b);
                eprintln!("[Burn-AI]     发现独特颜色: RGB({}, {}, {}) 色相={:.0}° 显著性={:.1}", 
                    color.r, color.g, color.b, h, salience);
                accent_colors.push(color);
                
                if accent_colors.len() >= 10 {
                    break;
                }
            }
        }
        
        eprintln!("[Burn-AI]   [阶段2] 发现 {} 种独特颜色", accent_colors.len());
        
        (dominant_colors, accent_colors)
    }
    
    fn select_dominant_colors_iterative(
        colors: &[ColorInfo],
        target_count: usize,
        _initial_distance: f64,
    ) -> Vec<ColorInfo> {
        eprintln!("[Burn-AI] [智能选择] 开始智能主色调选择...");
        let result = Self::select_dominant_colors_smart(colors, target_count);
        eprintln!("[Burn-AI] [智能选择] 完成，提取 {} 种主色调", result.len());
        result
    }
    
    pub fn analyze(&self, rgba: &[u8], width: u32, height: u32) -> ColorAnalysisResult {
        let total_start = Instant::now();
        let pixel_count = (width * height) as u64;
        
        eprintln!("[Burn-AI] ========== 颜色分析开始 ==========");
        eprintln!("[Burn-AI] 输入: {}x{} = {} 像素 ({}万)", width, height, pixel_count, pixel_count / 10000);
        eprintln!("[Burn-AI] RGBA数据大小: {} bytes", rgba.len());
        
        let step_start = Instant::now();
        let features = Self::quick_evaluate_features(rgba, width, height);
        eprintln!("[Burn-AI] [子步骤1] 快速特征评估完成 | 耗时: {}ms", step_start.elapsed().as_millis());
        
        let params = Self::calculate_analysis_params(&features);
        
        let step_start = Instant::now();
        let histogram = Self::calculate_histogram(rgba);
        eprintln!("[Burn-AI] [子步骤2] 直方图计算完成 | 耗时: {}ms", step_start.elapsed().as_millis());
        
        let step_start = Instant::now();
        let color_counts = Self::count_unique_colors(rgba);
        let unique_count = color_counts.len();
        eprintln!("[Burn-AI] [子步骤3] 唯一颜色统计: {} 种 | 耗时: {}ms", 
            unique_count, step_start.elapsed().as_millis());
        
        let step_start = Instant::now();
        
        let mut unique_colors: Vec<ColorInfo> = color_counts
            .iter()
            .map(|(&key, &count)| {
                let (r, g, b) = Self::key_to_color(key);
                ColorInfo {
                    r,
                    g,
                    b,
                    count,
                    percentage: count as f64 / pixel_count as f64,
                }
            })
            .collect();
        
        unique_colors.sort_by(|a, b| b.count.cmp(&a.count));
        eprintln!("[Burn-AI] [子步骤4] 颜色排序完成 | 耗时: {}ms", step_start.elapsed().as_millis());
        
        let step_start = Instant::now();
        eprintln!("[Burn-AI] [子步骤5] 主色调选择 (两阶段算法):");
        let (dominant_colors, accent_colors) = Self::select_colors_two_stage(
            &unique_colors,
            params.target_dominant_colors,
        );
        eprintln!("[Burn-AI]   最终提取: {} 种主色调, {} 种独特颜色 | 耗时: {}ms", 
            dominant_colors.len(), accent_colors.len(), step_start.elapsed().as_millis());
        
        eprintln!("[Burn-AI]   主色调详情:");
        for (i, c) in dominant_colors.iter().enumerate().take(10) {
            eprintln!("[Burn-AI]     {}: RGB({}, {}, {}) - {:.2}%", 
                i + 1, c.r, c.g, c.b, c.percentage * 100.0);
        }
        
        if !accent_colors.is_empty() {
            eprintln!("[Burn-AI]   独特颜色详情:");
            for (i, c) in accent_colors.iter().enumerate() {
                eprintln!("[Burn-AI]     {}: RGB({}, {}, {}) - {:.2}%", 
                    i + 1, c.r, c.g, c.b, c.percentage * 100.0);
            }
        }
        
        let step_start = Instant::now();
        let color_clusters = Self::kmeans_clustering(&unique_colors, params.cluster_count, params.max_iterations);
        eprintln!("[Burn-AI] [子步骤6] K-means聚类: {} 簇 | 耗时: {}ms", 
            color_clusters.len(), step_start.elapsed().as_millis());
        
        let step_start = Instant::now();
        let color_features = Self::calculate_color_features(&unique_colors, &dominant_colors);
        eprintln!("[Burn-AI] [子步骤7] 颜色特征计算完成 | 耗时: {}ms", step_start.elapsed().as_millis());
        eprintln!("[Burn-AI]   色温: {:.0}K ({})", color_features.color_temperature, color_features.temperature_category);
        eprintln!("[Burn-AI]   主导色相: {}", color_features.dominant_hue);
        eprintln!("[Burn-AI]   和谐度: {:.1}分", color_features.color_harmony_score);
        eprintln!("[Burn-AI]   平均饱和度: {:.2}", color_features.saturation_avg);
        eprintln!("[Burn-AI]   平均亮度: {:.2}", color_features.brightness_avg);
        
        eprintln!("[Burn-AI] ========== 颜色分析完成 ==========");
        eprintln!("[Burn-AI] 总耗时: {}ms", total_start.elapsed().as_millis());
        
        ColorAnalysisResult {
            histogram,
            unique_colors,
            color_clusters,
            dominant_colors,
            accent_colors,
            color_count: unique_count,
            features: color_features,
        }
    }
    
    fn calculate_histogram(rgba: &[u8]) -> ColorHistogram {
        let pixel_count = rgba.len() / 4;
        if pixel_count == 0 {
            return ColorHistogram::default();
        }
        
        let red: [AtomicU64; 256] = std::array::from_fn(|_| AtomicU64::new(0));
        let green: [AtomicU64; 256] = std::array::from_fn(|_| AtomicU64::new(0));
        let blue: [AtomicU64; 256] = std::array::from_fn(|_| AtomicU64::new(0));
        let gray: [AtomicU64; 256] = std::array::from_fn(|_| AtomicU64::new(0));
        
        rgba.par_chunks(4).for_each(|chunk| {
            if chunk.len() >= 3 {
                let r = chunk[0] as usize;
                let g = chunk[1] as usize;
                let b = chunk[2] as usize;
                let gray_val = ((0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64) as usize).min(255);
                
                red[r].fetch_add(1, Ordering::Relaxed);
                green[g].fetch_add(1, Ordering::Relaxed);
                blue[b].fetch_add(1, Ordering::Relaxed);
                gray[gray_val].fetch_add(1, Ordering::Relaxed);
            }
        });
        
        ColorHistogram {
            red: red.map(|v| v.load(Ordering::Relaxed)).to_vec(),
            green: green.map(|v| v.load(Ordering::Relaxed)).to_vec(),
            blue: blue.map(|v| v.load(Ordering::Relaxed)).to_vec(),
            gray: gray.map(|v| v.load(Ordering::Relaxed)).to_vec(),
        }
    }
    
    fn count_unique_colors(rgba: &[u8]) -> HashMap<u32, u64> {
        let pixel_count = rgba.len() / 4;
        if pixel_count == 0 {
            return HashMap::new();
        }
        
        let chunk_size = (pixel_count / rayon::current_num_threads()).max(1000);
        
        let local_maps: Vec<HashMap<u32, u64>> = rgba
            .par_chunks(chunk_size * 4)
            .map(|chunk| {
                let mut local_counts: HashMap<u32, u64> = HashMap::new();
                for pixel in chunk.chunks(4) {
                    if pixel.len() >= 3 {
                        let key = Self::color_to_key(pixel[0], pixel[1], pixel[2]);
                        *local_counts.entry(key).or_insert(0) += 1;
                    }
                }
                local_counts
            })
            .collect();
        
        let mut result: HashMap<u32, u64> = HashMap::new();
        for local_map in local_maps {
            for (key, count) in local_map {
                *result.entry(key).or_insert(0) += count;
            }
        }
        result
    }
    
    fn color_to_key(r: u8, g: u8, b: u8) -> u32 {
        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
    
    fn key_to_color(key: u32) -> (u8, u8, u8) {
        let r = ((key >> 16) & 0xFF) as u8;
        let g = ((key >> 8) & 0xFF) as u8;
        let b = (key & 0xFF) as u8;
        (r, g, b)
    }
    
    fn color_distance(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) -> f64 {
        let dr = r1 as f64 - r2 as f64;
        let dg = g1 as f64 - g2 as f64;
        let db = b1 as f64 - b2 as f64;
        (dr * dr + dg * dg + db * db).sqrt()
    }
    
    fn rgb_to_xyz(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
        let r = r as f64 / 255.0;
        let g = g as f64 / 255.0;
        let b = b as f64 / 255.0;
        
        let r = if r > 0.04045 { ((r + 0.055) / 1.055).powf(2.4) } else { r / 12.92 };
        let g = if g > 0.04045 { ((g + 0.055) / 1.055).powf(2.4) } else { g / 12.92 };
        let b = if b > 0.04045 { ((b + 0.055) / 1.055).powf(2.4) } else { b / 12.92 };
        
        let x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375;
        let y = r * 0.2126729 + g * 0.7151522 + b * 0.0721750;
        let z = r * 0.0193339 + g * 0.1191920 + b * 0.9503041;
        
        (x * 100.0, y * 100.0, z * 100.0)
    }
    
    fn xyz_to_lab(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        let xn = 95.047;
        let yn = 100.000;
        let zn = 108.883;
        
        let fx = if x / xn > 0.008856 { (x / xn).powf(1.0 / 3.0) } else { 7.787 * (x / xn) + 16.0 / 116.0 };
        let fy = if y / yn > 0.008856 { (y / yn).powf(1.0 / 3.0) } else { 7.787 * (y / yn) + 16.0 / 116.0 };
        let fz = if z / zn > 0.008856 { (z / zn).powf(1.0 / 3.0) } else { 7.787 * (z / zn) + 16.0 / 116.0 };
        
        let l = 116.0 * fy - 16.0;
        let a = 500.0 * (fx - fy);
        let b = 200.0 * (fy - fz);
        
        (l, a, b)
    }
    
    fn rgb_to_lab(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
        let (x, y, z) = Self::rgb_to_xyz(r, g, b);
        Self::xyz_to_lab(x, y, z)
    }
    
    fn color_distance_lab(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) -> f64 {
        let (l1, a1, b1_lab) = Self::rgb_to_lab(r1, g1, b1);
        let (l2, a2, b2_lab) = Self::rgb_to_lab(r2, g2, b2);
        
        let dl = l1 - l2;
        let da = a1 - a2;
        let db = b1_lab - b2_lab;
        
        (dl * dl + da * da + db * db).sqrt()
    }
    
    fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
        let r = r as f64 / 255.0;
        let g = g as f64 / 255.0;
        let b = b as f64 / 255.0;
        
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        
        let h = if delta < 1e-6 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };
        let h = if h < 0.0 { h + 360.0 } else { h };
        
        let s = if max < 1e-6 { 0.0 } else { delta / max };
        let v = max;
        
        (h, s, v)
    }
    
    fn calculate_color_salience(color: &ColorInfo, dominant_colors: &[ColorInfo]) -> f64 {
        let (h, s, v) = Self::rgb_to_hsv(color.r, color.g, color.b);
        
        let saturation_score = s * 30.0;
        
        let hue_salience = if (h >= 0.0 && h < 30.0) || (h >= 330.0 && h <= 360.0) {
            25.0
        } else if h >= 30.0 && h < 60.0 {
            20.0
        } else if h >= 270.0 && h < 330.0 {
            18.0
        } else if h >= 60.0 && h < 90.0 {
            15.0
        } else if h >= 180.0 && h < 270.0 {
            12.0
        } else {
            10.0
        };
        
        let mut min_distance = f64::MAX;
        for dominant in dominant_colors {
            let dist = Self::color_distance_lab(
                color.r, color.g, color.b,
                dominant.r, dominant.g, dominant.b,
            );
            min_distance = min_distance.min(dist);
        }
        let contrast_score = (min_distance / 50.0).min(1.0) * 25.0;
        
        let brightness_score = v * 20.0;
        
        saturation_score + hue_salience + contrast_score + brightness_score
    }
    
    fn calculate_color_features(colors: &[ColorInfo], dominant_colors: &[ColorInfo]) -> crate::types::color::ColorFeatures {
        let mut total_saturation = 0.0;
        let mut total_brightness = 0.0;
        let mut total_temp = 0.0;
        let mut total_weight = 0.0;
        let mut hue_counts: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        
        for color in dominant_colors.iter().chain(colors.iter().take(1000)) {
            let weight = color.count as f64;
            let (h, s, v) = Self::rgb_to_hsv(color.r, color.g, color.b);
            
            total_saturation += s * weight;
            total_brightness += v * weight;
            total_temp += Self::calculate_color_temperature(color.r, color.g, color.b) * weight;
            total_weight += weight;
            
            let hue_name = Self::get_hue_name(h);
            *hue_counts.entry(hue_name).or_insert(0.0) += weight;
        }
        
        let avg_saturation = if total_weight > 0.0 { total_saturation / total_weight } else { 0.0 };
        let avg_brightness = if total_weight > 0.0 { total_brightness / total_weight } else { 0.0 };
        let color_temp = if total_weight > 0.0 { total_temp / total_weight } else { 6500.0 };
        
        let temperature_category = if color_temp < 4000.0 {
            "暖色调".to_string()
        } else if color_temp < 6000.0 {
            "中性色调".to_string()
        } else {
            "冷色调".to_string()
        };
        
        let dominant_hue = hue_counts
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| "未知".to_string());
        
        let is_grayscale = avg_saturation < 0.1;
        
        let total_pixels: u64 = colors.iter().map(|c| c.count).sum();
        let color_entropy = if total_pixels > 0 {
            let mut entropy = 0.0;
            for color in colors.iter().take(1000) {
                if color.count > 0 {
                    let p = color.count as f64 / total_pixels as f64;
                    entropy -= p * p.log2();
                }
            }
            entropy
        } else {
            0.0
        };
        
        let color_harmony_score = Self::calculate_harmony_score(dominant_colors);
        
        crate::types::color::ColorFeatures {
            color_temperature: color_temp,
            temperature_category,
            dominant_hue,
            color_harmony_score,
            is_grayscale,
            color_entropy,
            saturation_avg: avg_saturation,
            brightness_avg: avg_brightness,
        }
    }
    
    fn calculate_color_temperature(r: u8, g: u8, b: u8) -> f64 {
        let r = r as f64;
        let g = g as f64;
        let b = b as f64;
        
        let x = (r / 255.0).powf(2.2) * 0.4124 + (g / 255.0).powf(2.2) * 0.3576 + (b / 255.0).powf(2.2) * 0.1805;
        let y = (r / 255.0).powf(2.2) * 0.2126 + (g / 255.0).powf(2.2) * 0.7152 + (b / 255.0).powf(2.2) * 0.0722;
        let z = (r / 255.0).powf(2.2) * 0.0193 + (g / 255.0).powf(2.2) * 0.1192 + (b / 255.0).powf(2.2) * 0.9505;
        
        let n = (0.23881 * x + 0.25499 * y - 0.58291 * z) / (0.11109 * x - 0.85406 * y + 0.52259 * z);
        
        let temp = 449.0 * n.powf(3.0) + 3525.0 * n.powf(2.0) + 6823.3 * n + 5520.33;
        temp.max(1000.0).min(40000.0)
    }
    
    fn get_hue_name(h: f64) -> String {
        if h < 15.0 || h >= 345.0 {
            "红色".to_string()
        } else if h < 45.0 {
            "橙色".to_string()
        } else if h < 75.0 {
            "黄色".to_string()
        } else if h < 150.0 {
            "绿色".to_string()
        } else if h < 195.0 {
            "青色".to_string()
        } else if h < 255.0 {
            "蓝色".to_string()
        } else if h < 285.0 {
            "紫色".to_string()
        } else {
            "品红".to_string()
        }
    }
    
    fn calculate_harmony_score(colors: &[ColorInfo]) -> f64 {
        if colors.len() < 2 {
            return 100.0;
        }
        
        let hues: Vec<f64> = colors.iter().map(|c| {
            let (h, _, _) = Self::rgb_to_hsv(c.r, c.g, c.b);
            h
        }).collect();
        
        let mut harmony_score = 0.0;
        let mut count = 0;
        
        for i in 0..hues.len() {
            for j in (i + 1)..hues.len() {
                let diff = (hues[i] - hues[j]).abs();
                let diff = diff.min(360.0 - diff);
                
                let score = if diff < 30.0 || diff > 330.0 {
                    100.0
                } else if (diff - 180.0).abs() < 30.0 {
                    90.0
                } else if (diff - 120.0).abs() < 30.0 || (diff - 240.0).abs() < 30.0 {
                    85.0
                } else if (diff - 90.0).abs() < 30.0 || (diff - 270.0).abs() < 30.0 {
                    70.0
                } else if (diff - 60.0).abs() < 30.0 || (diff - 300.0).abs() < 30.0 {
                    60.0
                } else {
                    40.0
                };
                
                harmony_score += score;
                count += 1;
            }
        }
        
        if count > 0 { harmony_score / count as f64 } else { 50.0 }
    }
    
    fn kmeans_clustering(colors: &[ColorInfo], k: usize, max_iterations: usize) -> Vec<ColorCluster> {
        if colors.is_empty() || k == 0 {
            return Vec::new();
        }
        
        let pixel_count_estimate = colors.iter().map(|c| c.count).sum::<u64>() as f64;
        let color_density = colors.len() as f64 / pixel_count_estimate.max(1.0);
        
        let sample_size = if color_density > 0.5 {
            400.min(colors.len())
        } else if color_density > 0.3 {
            600.min(colors.len())
        } else {
            800.min(colors.len())
        };
        
        let step = colors.len() / sample_size;
        let sampled: Vec<&ColorInfo> = (0..colors.len()).step_by(step.max(1)).map(|i| &colors[i]).collect();
        
        let mut centers: Vec<(f64, f64, f64)> = sampled
            .iter()
            .take(k)
            .map(|c| (c.r as f64, c.g as f64, c.b as f64))
            .collect();
        
        if centers.len() < k {
            while centers.len() < k {
                let idx = centers.len();
                centers.push((sampled[idx % sampled.len()].r as f64, 
                            sampled[idx % sampled.len()].g as f64, 
                            sampled[idx % sampled.len()].b as f64));
            }
        }
        
        let mut assignments = vec![0usize; sampled.len()];
        
        for _ in 0..max_iterations {
            for (i, color) in sampled.iter().enumerate() {
                let mut min_dist = f64::MAX;
                let mut min_idx = 0;
                
                for (j, center) in centers.iter().enumerate() {
                    let dist = Self::color_distance(
                        color.r, color.g, color.b,
                        center.0 as u8, center.1 as u8, center.2 as u8,
                    );
                    if dist < min_dist {
                        min_dist = dist;
                        min_idx = j;
                    }
                }
                assignments[i] = min_idx;
            }
            
            let mut new_centers: Vec<(f64, f64, f64)> = vec![(0.0, 0.0, 0.0); k];
            let mut counts: Vec<usize> = vec![0; k];
            
            for (i, color) in sampled.iter().enumerate() {
                let cluster = assignments[i];
                new_centers[cluster].0 += color.r as f64 * color.count as f64;
                new_centers[cluster].1 += color.g as f64 * color.count as f64;
                new_centers[cluster].2 += color.b as f64 * color.count as f64;
                counts[cluster] += color.count as usize;
            }
            
            for j in 0..k {
                if counts[j] > 0 {
                    centers[j] = (
                        new_centers[j].0 / counts[j] as f64,
                        new_centers[j].1 / counts[j] as f64,
                        new_centers[j].2 / counts[j] as f64,
                    );
                }
            }
        }
        
        let mut clusters: Vec<ColorCluster> = centers
            .iter()
            .enumerate()
            .map(|(i, center)| {
                let cluster_pixels: Vec<&ColorInfo> = sampled
                    .iter()
                    .zip(assignments.iter())
                    .filter(|(_, &a)| a == i)
                    .map(|(c, _)| *c)
                    .collect();
                
                let pixel_count: u64 = cluster_pixels.iter().map(|c| c.count).sum();
                let percentage = pixel_count as f64 / pixel_count_estimate;
                
                ColorCluster {
                    center_r: center.0 as u8,
                    center_g: center.1 as u8,
                    center_b: center.2 as u8,
                    total_count: pixel_count,
                    total_percentage: percentage,
                    colors: cluster_pixels.iter().map(|c| ColorInfo {
                        r: c.r,
                        g: c.g,
                        b: c.b,
                        count: c.count,
                        percentage: c.percentage,
                    }).collect(),
                }
            })
            .collect();
        
        clusters.sort_by(|a, b| b.total_count.cmp(&a.total_count));
        clusters
    }
}

impl Default for BurnColorAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

pub fn analyze_colors_fast(rgba: &[u8], width: u32, height: u32) -> ColorAnalysisResult {
    let analyzer = BurnColorAnalyzer::new();
    analyzer.analyze(rgba, width, height)
}
