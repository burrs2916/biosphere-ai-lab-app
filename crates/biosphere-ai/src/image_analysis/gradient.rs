use rayon::prelude::*;

#[derive(Debug, Clone)]
pub struct GradientResult {
    pub magnitude: Vec<f32>,
    pub direction: Vec<f32>,
    pub width: u32,
    pub height: u32,
}

pub struct BurnGradientCalculator;

impl BurnGradientCalculator {
    pub fn new() -> Self {
        Self
    }
    
    pub fn calculate(gray: &[u8], width: u32, height: u32) -> GradientResult {
        if width < 3 || height < 3 {
            let size = (width * height) as usize;
            return GradientResult {
                magnitude: vec![0.0f32; size],
                direction: vec![0.0f32; size],
                width,
                height,
            };
        }
        
        let w = width as usize;
        let h = height as usize;
        let total = w * h;
        
        let (magnitude, direction): (Vec<f32>, Vec<f32>) = (0..total)
            .into_par_iter()
            .map(|idx| {
                let y = idx / w;
                let x = idx % w;
                
                if x == 0 || x == w - 1 || y == 0 || y == h - 1 {
                    return (0.0f32, 0.0f32);
                }
                
                let get = |px: usize, py: usize| -> f32 {
                    gray[py * w + px] as f32
                };
                
                let gx = 
                    -get(x - 1, y - 1) + get(x + 1, y - 1) +
                    -2.0 * get(x - 1, y) + 2.0 * get(x + 1, y) +
                    -get(x - 1, y + 1) + get(x + 1, y + 1);
                
                let gy = 
                    -get(x - 1, y - 1) - 2.0 * get(x, y - 1) - get(x + 1, y - 1) +
                    get(x - 1, y + 1) + 2.0 * get(x, y + 1) + get(x + 1, y + 1);
                
                let mag = (gx * gx + gy * gy).sqrt();
                let dir = gy.atan2(gx).to_degrees();
                let dir = if dir < 0.0 { dir + 360.0 } else { dir };
                
                (mag, dir)
            })
            .unzip();
        
        GradientResult {
            magnitude,
            direction,
            width,
            height,
        }
    }
}

impl Default for BurnGradientCalculator {
    fn default() -> Self {
        Self::new()
    }
}

pub fn calculate_gradient_fast(gray: &[u8], width: u32, height: u32) -> GradientResult {
    BurnGradientCalculator::calculate(gray, width, height)
}
