use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sysinfo::System;
use tokio::sync::RwLock;

use crate::core::EventBus;
use crate::core::event::LabEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    pub cpu_usage_percent: f32,
    pub memory_total_mb: u64,
    pub memory_available_mb: u64,
    pub memory_usage_percent: f32,
    pub disk_total_gb: u64,
    pub disk_available_gb: u64,
    pub disk_usage_percent: f32,
    pub gpu_usage_percent: Option<f32>,
    pub gpu_memory_used_mb: Option<u64>,
    pub gpu_memory_total_mb: Option<u64>,
    pub timestamp: u64,
}

struct SharedState {
    sys: System,
    tick: u64,
}

pub struct ResourceMonitor {
    event_bus: Arc<EventBus>,
    interval_secs: u64,
    running: Arc<RwLock<bool>>,
    handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    session_id: Arc<RwLock<Option<crate::types::SessionId>>>,
    shared: Arc<RwLock<SharedState>>,
}

impl ResourceMonitor {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        let sys = System::new_all();
        Self {
            event_bus,
            interval_secs: 5,
            running: Arc::new(RwLock::new(false)),
            handle: Arc::new(RwLock::new(None)),
            session_id: Arc::new(RwLock::new(None)),
            shared: Arc::new(RwLock::new(SharedState { sys, tick: 0 })),
        }
    }

    pub fn with_interval(mut self, secs: u64) -> Self {
        self.interval_secs = secs;
        self
    }

    pub async fn set_session_id(&self, id: Option<crate::types::SessionId>) {
        let mut sid = self.session_id.write().await;
        *sid = id;
    }

    pub async fn start(&self) {
        let mut running = self.running.write().await;
        if *running {
            return;
        }
        *running = true;
        drop(running);

        let event_bus = self.event_bus.clone();
        let interval = self.interval_secs;
        let running_flag = self.running.clone();
        let session_id_flag = self.session_id.clone();
        let shared = self.shared.clone();

        let handle = tokio::spawn(async move {
            loop {
                {
                    let r = running_flag.read().await;
                    if !*r {
                        break;
                    }
                }

                let snapshot = {
                    let mut state = shared.write().await;
                    state.sys.refresh_all();
                    state.tick += 1;
                    Self::build_snapshot(&state.sys, state.tick)
                };

                let used_mem = snapshot.memory_total_mb - snapshot.memory_available_mb;
                event_bus.emit(LabEvent::HardwareAlert {
                    session_id: session_id_flag.read().await.clone(),
                    cpu_usage: snapshot.cpu_usage_percent,
                    memory_usage: snapshot.memory_usage_percent,
                    memory_total_mb: snapshot.memory_total_mb,
                    memory_available_mb: snapshot.memory_available_mb,
                    disk_total_gb: snapshot.disk_total_gb,
                    disk_available_gb: snapshot.disk_available_gb,
                    disk_usage_percent: snapshot.disk_usage_percent,
                    gpu_usage: snapshot.gpu_usage_percent,
                    gpu_memory_used_mb: snapshot.gpu_memory_used_mb,
                    gpu_memory_total_mb: snapshot.gpu_memory_total_mb,
                    message: format!(
                        "CPU: {:.0}% | MEM: {:.0}% ({}/{}MB) | DISK: {:.0}% ({}/{}GB) | GPU: {}",
                        snapshot.cpu_usage_percent,
                        snapshot.memory_usage_percent,
                        used_mem,
                        snapshot.memory_total_mb,
                        snapshot.disk_usage_percent,
                        snapshot.disk_total_gb - snapshot.disk_available_gb,
                        snapshot.disk_total_gb,
                        snapshot.gpu_usage_percent.map(|v| format!("{:.0}%", v)).unwrap_or_else(|| "N/A".to_string())
                    ),
                });

                tokio::time::sleep(Duration::from_secs(interval)).await;
            }
        });

        let mut h = self.handle.write().await;
        *h = Some(handle);
    }

    pub async fn stop(&self) {
        {
            let mut running = self.running.write().await;
            *running = false;
        }
        let mut h = self.handle.write().await;
        if let Some(handle) = h.take() {
            handle.abort();
        }
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    pub async fn snapshot(&self) -> ResourceSnapshot {
        let mut state = self.shared.write().await;
        state.sys.refresh_all();
        Self::build_snapshot(&state.sys, state.tick)
    }

    fn build_snapshot(sys: &System, tick: u64) -> ResourceSnapshot {
        let cpu_usage: f32 = sys.global_cpu_usage();
        let total_mem = sys.total_memory() / 1024;
        let used_mem = sys.used_memory() / 1024;
        let available_mem = total_mem.saturating_sub(used_mem);
        let mem_usage: f32 = if total_mem > 0 {
            (used_mem as f32 / total_mem as f32) * 100.0
        } else {
            0.0
        };

        let (disk_total_gb, disk_available_gb, disk_usage_percent) = Self::read_disk_stats();
        let (gpu_usage, gpu_mem_used, gpu_mem_total) = Self::read_gpu_stats();

        ResourceSnapshot {
            cpu_usage_percent: cpu_usage,
            memory_total_mb: total_mem,
            memory_available_mb: available_mem,
            memory_usage_percent: mem_usage,
            disk_total_gb,
            disk_available_gb,
            disk_usage_percent,
            gpu_usage_percent: gpu_usage,
            gpu_memory_used_mb: gpu_mem_used,
            gpu_memory_total_mb: gpu_mem_total,
            timestamp: tick,
        }
    }

    fn read_disk_stats() -> (u64, u64, f32) {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let mut total: u64 = 0;
        let mut available: u64 = 0;
        for disk in &disks {
            total += disk.total_space() / (1024 * 1024 * 1024);
            available += disk.available_space() / (1024 * 1024 * 1024);
        }
        let used = total.saturating_sub(available);
        let usage_percent = if total > 0 {
            (used as f32 / total as f32) * 100.0
        } else {
            0.0
        };
        (total, available, usage_percent)
    }

    fn read_gpu_stats() -> (Option<f32>, Option<u64>, Option<u64>) {
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("ioreg")
                .args(["-l", "-w0", "-r", "-c", "IOGPUDevice"])
                .output()
            {
                if output.status.success() {
                    if let Ok(text) = String::from_utf8(output.stdout) {
                        if let Some((util, mem_used, mem_total)) = Self::parse_ioreg_gpu(&text) {
                            return (Some(util), mem_used, mem_total);
                        }
                    }
                }
            }

            if let Ok(output) = std::process::Command::new("system_profiler")
                .args(["SPDisplaysDataType"])
                .output()
            {
                if output.status.success() {
                    if let Ok(text) = String::from_utf8(output.stdout) {
                        if let Some(vram) = Self::parse_spdisplays_vram(&text) {
                            return (None, None, Some(vram));
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = std::process::Command::new("nvidia-smi")
                .args(["--query-gpu=utilization.gpu,memory.used,memory.total", "--format=csv,noheader,nounits"])
                .output()
            {
                if output.status.success() {
                    if let Ok(text) = String::from_utf8(output.stdout) {
                        let parts: Vec<&str> = text.trim().split(',').collect();
                        if parts.len() >= 3 {
                            let gpu_util = parts[0].trim().parse::<f32>().ok();
                            let mem_used = parts[1].trim().parse::<u64>().ok();
                            let mem_total = parts[2].trim().parse::<u64>().ok();
                            return (gpu_util, mem_used, mem_total);
                        }
                    }
                }
            }
        }

        (None, None, None)
    }

    #[cfg(target_os = "macos")]
    fn parse_ioreg_gpu(text: &str) -> Option<(f32, Option<u64>, Option<u64>)> {
        let mut utilization: Option<f32> = None;
        let mut mem_used: Option<u64> = None;
        let mut mem_total: Option<u64> = None;

        for line in text.lines() {
            let trimmed = line.trim();

            if trimmed.contains("\"PerformanceStatistics\"") {
                continue;
            }

            if trimmed.contains("\"utilization\"") {
                if let Some(val) = trimmed.split('=').last() {
                    let num: String = val.trim()
                        .trim_end_matches(',')
                        .trim_end_matches(';')
                        .chars()
                        .filter(|c| c.is_ascii_digit() || *c == '.')
                        .collect();
                    if let Ok(v) = num.parse::<f32>() {
                        utilization = Some(v);
                    }
                }
            }

            if trimmed.contains("\"In use\"") || trimmed.contains("\"InUse\"") {
                if let Some(val) = trimmed.split('=').last() {
                    let num: String = val.trim()
                        .trim_end_matches(',')
                        .trim_end_matches(';')
                        .chars()
                        .filter(|c| c.is_ascii_digit())
                        .collect();
                    if let Ok(v) = num.parse::<u64>() {
                        mem_used = Some(v / 1024);
                    }
                }
            }

            if trimmed.contains("\"Total\"") || trimmed.contains("\"VRAM,totalMB\"") {
                if let Some(val) = trimmed.split('=').last() {
                    let num: String = val.trim()
                        .trim_end_matches(',')
                        .trim_end_matches(';')
                        .chars()
                        .filter(|c| c.is_ascii_digit())
                        .collect();
                    if let Ok(v) = num.parse::<u64>() {
                        mem_total = Some(v);
                    }
                }
            }
        }

        if utilization.is_some() || mem_used.is_some() || mem_total.is_some() {
            Some((utilization.unwrap_or(0.0), mem_used, mem_total))
        } else {
            None
        }
    }

    #[cfg(target_os = "macos")]
    fn parse_spdisplays_vram(text: &str) -> Option<u64> {
        for line in text.lines() {
            if line.contains("VRAM") {
                let num: String = line.chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(v) = num.parse::<u64>() {
                    return Some(v);
                }
            }
        }
        None
    }
}

impl std::fmt::Debug for ResourceMonitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceMonitor")
            .field("interval_secs", &self.interval_secs)
            .finish()
    }
}
