use serde::{Deserialize, Serialize};

use crate::core::Result;
use crate::types::{ComputeBackend, TaskType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cpu_cores: usize,
    pub cpu_model: String,
    pub total_memory_mb: u64,
    pub available_memory_mb: u64,
    pub cpu_usage_percent: f32,
    pub gpu_devices: Vec<GpuInfo>,
    pub os_name: String,
    pub os_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub vram_mb: u64,
    pub compute_backend: ComputeBackend,
    pub driver_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRecommendation {
    pub recommended_batch_size: usize,
    pub recommended_epochs: usize,
    pub recommended_backend: ComputeBackend,
    pub recommended_learning_rate: f64,
    pub estimated_training_time_minutes: f64,
    pub can_train_locally: bool,
    pub should_use_remote: bool,
    pub warnings: Vec<String>,
}

pub struct HardwareDetector;

impl HardwareDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect(&self) -> Result<HardwareInfo> {
        use sysinfo::System;

        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_cores = sys.physical_core_count().unwrap_or(1);
        let cpu_model = sys.cpus().first()
            .map(|c| c.brand().to_string())
            .unwrap_or_default();
        let total_memory_mb = sys.total_memory() / 1024;
        let available_memory_mb = sys.available_memory() / 1024;
        let cpu_usage_percent = sys.global_cpu_usage();

        let os_name = System::name().unwrap_or_default();
        let os_version = System::os_version().unwrap_or_default();

        let gpu_devices = self.detect_gpus();

        crate::infrastructure::log(
            "HARDWARE",
            &format!("检测到硬件: CPU {}核 {} | 内存 {}MB可用/{}MB总计 | GPU {}个",
                cpu_cores, cpu_model, available_memory_mb, total_memory_mb, gpu_devices.len()),
            None,
        );

        Ok(HardwareInfo {
            cpu_cores,
            cpu_model,
            total_memory_mb,
            available_memory_mb,
            cpu_usage_percent,
            gpu_devices,
            os_name,
            os_version,
        })
    }

    fn detect_gpus(&self) -> Vec<GpuInfo> {
        let mut gpus = Vec::new();

        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("system_profiler")
                .args(["SPDisplaysDataType", "-json"])
                .output()
            {
                if output.status.success() {
                    if let Ok(json_str) = String::from_utf8(output.stdout) {
                        gpus = Self::parse_macos_gpu_json(&json_str);
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = std::process::Command::new("lspci")
                .arg("-mm")
                .output()
            {
                if output.status.success() {
                    if let Ok(text) = String::from_utf8(output.stdout) {
                        for line in text.lines() {
                            if line.to_lowercase().contains("vga") || line.to_lowercase().contains("3d") || line.to_lowercase().contains("display") {
                                let parts: Vec<&str> = line.split('"').filter(|s| !s.trim().is_empty()).collect();
                                if parts.len() >= 4 {
                                    let name = parts[parts.len() - 1].trim().to_string();
                                    gpus.push(GpuInfo {
                                        name: name.clone(),
                                        vram_mb: 0,
                                        compute_backend: if name.to_lowercase().contains("nvidia") {
                                            ComputeBackend::Cuda
                                        } else {
                                            ComputeBackend::Wgpu
                                        },
                                        driver_version: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = std::process::Command::new("wmic")
                .args(["path", "win32_VideoController", "get", "Name,AdapterRAM,DriverVersion", "/format:csv"])
                .output()
            {
                if output.status.success() {
                    if let Ok(text) = String::from_utf8(output.stdout) {
                        for line in text.lines().skip(2) {
                            let fields: Vec<&str> = line.split(',').filter(|s| !s.is_empty()).collect();
                            if fields.len() >= 2 {
                                let name = fields.get(1).unwrap_or(&"").trim().to_string();
                                if name.is_empty() { continue; }
                                let vram: u64 = fields.get(2)
                                    .and_then(|s| s.trim().parse().ok())
                                    .unwrap_or(0) / (1024 * 1024);
                                let driver = fields.get(3).map(|s| s.trim().to_string());
                                gpus.push(GpuInfo {
                                    name: name.clone(),
                                    vram_mb: vram,
                                    compute_backend: if name.to_lowercase().contains("nvidia") {
                                        ComputeBackend::Cuda
                                    } else {
                                        ComputeBackend::Wgpu
                                    },
                                    driver_version: driver,
                                });
                            }
                        }
                    }
                }
            }
        }

        gpus
    }

    #[cfg(target_os = "macos")]
    fn parse_macos_gpu_json(json_str: &str) -> Vec<GpuInfo> {
        let mut gpus = Vec::new();

        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(arrays) = value.get("SPDisplaysDataType").and_then(|v| v.as_array()) {
                for display in arrays {
                    let name = display.get("_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown GPU")
                        .to_string();

                    let vram_mb = display.get("spdisplays_vram")
                        .and_then(|v| v.as_str())
                        .and_then(|s| {
                            let num_part: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
                            num_part.parse::<u64>().ok()
                        })
                        .or_else(|| {
                            display.get("spdisplays_vram_shared")
                                .and_then(|v| v.as_str())
                                .and_then(|s| {
                                    let num_part: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
                                    num_part.parse::<u64>().ok()
                                })
                        })
                        .unwrap_or(0);

                    let metal_supported = display.get("spdisplays_mtlgpufamilysupport")
                        .and_then(|v| v.as_str())
                        .is_some();

                    let is_apple_silicon = name.contains("Apple") || display.get("spdisplays_chipset-model")
                        .and_then(|v| v.as_str())
                        .map(|s| s.contains("Apple"))
                        .unwrap_or(false);

                    let compute_backend = if is_apple_silicon || metal_supported {
                        ComputeBackend::Metal
                    } else {
                        ComputeBackend::Wgpu
                    };

                    let driver = display.get("spdisplays_mtlgpufamilysupport")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    gpus.push(GpuInfo {
                        name,
                        vram_mb,
                        compute_backend,
                        driver_version: driver,
                    });
                }
            }
        }

        gpus
    }
}

impl Default for HardwareDetector {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ConfigRecommender;

impl ConfigRecommender {
    pub fn new() -> Self {
        Self
    }

    pub fn recommend(
        &self,
        hardware: &HardwareInfo,
        task_type: TaskType,
        data_size: usize,
    ) -> TrainingRecommendation {
        let has_gpu = !hardware.gpu_devices.is_empty();
        let available_mem = hardware.available_memory_mb;

        let recommended_backend = if has_gpu {
            hardware.gpu_devices.first()
                .map(|g| g.compute_backend)
                .unwrap_or(ComputeBackend::Wgpu)
        } else {
            ComputeBackend::Wgpu
        };

        let recommended_batch_size = if available_mem > 8000 {
            64
        } else if available_mem > 4000 {
            32
        } else {
            16
        };

        let recommended_epochs = match task_type {
            TaskType::Classification => 50,
            TaskType::Regression => 100,
            TaskType::Clustering => 200,
            _ => 50,
        };

        let estimated_time = (data_size as f64 / recommended_batch_size as f64
            * recommended_epochs as f64 * 0.001)
            .min(1440.0);

        let can_train_locally = available_mem > 1000;
        let should_use_remote = !can_train_locally || data_size > 1_000_000;

        let mut warnings = Vec::new();
        if available_mem < 2000 {
            warnings.push("Low available memory. Consider closing other applications.".to_string());
        }
        if !has_gpu && data_size > 100_000 {
            warnings.push("No GPU detected. Large dataset training will be slow on CPU.".to_string());
        }
        if should_use_remote {
            warnings.push("Dataset is large. Consider using remote training.".to_string());
        }

        TrainingRecommendation {
            recommended_batch_size,
            recommended_epochs,
            recommended_backend,
            recommended_learning_rate: 0.001,
            estimated_training_time_minutes: estimated_time,
            can_train_locally,
            should_use_remote,
            warnings,
        }
    }
}

impl Default for ConfigRecommender {
    fn default() -> Self {
        Self::new()
    }
}
