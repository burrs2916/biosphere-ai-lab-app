use super::super::types::RegionResult;

pub fn segment_regions(gray: &[u8], width: u32, height: u32, tolerance: u8) -> RegionResult {
    let w = width as usize;
    let h = height as usize;
    let size = w * h;
    
    if size == 0 {
        return RegionResult {
            region_count: 0,
            regions: Vec::new(),
            region_map: Vec::new(),
            width,
            height,
        };
    }
    
    let mut region_map = vec![0u32; size];
    let mut visited = vec![false; size];
    let mut regions: Vec<crate::types::RegionInfo> = Vec::new();
    let mut next_region_id = 1u32;
    
    let estimated_regions = (size / 100).max(10);
    regions.reserve(estimated_regions);

    for i in 0..size {
        if visited[i] {
            continue;
        }
        
        let base_gray = gray[i];
        let (sum_gray, pixel_count) = flood_fill_fast(
            gray, w, h, i, base_gray, tolerance,
            &mut visited, &mut region_map, next_region_id,
        );

        if pixel_count > 0 {
            regions.push(crate::types::RegionInfo {
                id: next_region_id,
                avg_r: (sum_gray / pixel_count) as u8,
                avg_g: (sum_gray / pixel_count) as u8,
                avg_b: (sum_gray / pixel_count) as u8,
                pixel_count,
            });
            next_region_id += 1;
        }
    }

    RegionResult {
        region_count: regions.len(),
        regions,
        region_map,
        width,
        height,
    }
}

#[inline]
fn flood_fill_fast(
    gray: &[u8],
    width: usize,
    height: usize,
    start_idx: usize,
    base_gray: u8,
    tolerance: u8,
    visited: &mut [bool],
    region_map: &mut [u32],
    region_id: u32,
) -> (u64, u64) {
    let mut sum_gray = 0u64;
    let mut pixel_count = 0u64;
    
    let size = width * height;
    let mut stack: Vec<usize> = Vec::with_capacity(256);
    stack.push(start_idx);

    while let Some(idx) = stack.pop() {
        if idx >= size || visited[idx] {
            continue;
        }

        let current_gray = gray[idx];
        let diff = current_gray.abs_diff(base_gray);

        if diff > tolerance {
            continue;
        }

        visited[idx] = true;
        region_map[idx] = region_id;
        sum_gray += current_gray as u64;
        pixel_count += 1;

        let x = idx % width;
        let y = idx / width;

        if x > 0 {
            stack.push(idx - 1);
        }
        if x + 1 < width {
            stack.push(idx + 1);
        }
        if y > 0 {
            stack.push(idx - width);
        }
        if y + 1 < height {
            stack.push(idx + width);
        }
    }
    
    (sum_gray, pixel_count)
}
