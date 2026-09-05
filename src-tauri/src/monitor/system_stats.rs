use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use sysinfo::System;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveSystemMetrics {
    pub host_ram_used_mb: u64,
    pub host_ram_total_mb: u64,
    pub host_ram_percent: f32,
    pub cpu_usage_percent: f32,
    pub gpu_vram_used_mb: u64,
    pub gpu_vram_total_mb: u64,
    pub gpu_vram_percent: f32,
    pub gpu_name: String,
    pub is_rocm_ready: bool,
    pub timestamp_ms: i64,
}

pub struct SystemStatsMonitor {
    sys: Mutex<System>,
}

impl Default for SystemStatsMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemStatsMonitor {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self {
            sys: Mutex::new(sys),
        }
    }

    pub fn get_metrics(&self) -> LiveSystemMetrics {
        let mut sys = match self.sys.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        sys.refresh_memory();
        sys.refresh_cpu_all();

        let total_ram_kb = sys.total_memory();
        let used_ram_kb = sys.used_memory();
        let total_ram_mb = total_ram_kb / 1024;
        let used_ram_mb = used_ram_kb / 1024;
        let ram_pct = if total_ram_mb > 0 {
            (used_ram_mb as f32 / total_ram_mb as f32) * 100.0
        } else {
            0.0
        };

        let cpu_pct = sys.global_cpu_usage();

        // Honest placeholder: this fast host-side poll (called every 3s from the
        // frontend) has no cheap way to query live AMD/ROCm VRAM usage — that
        // requires a round-trip into WSL (see `WslBridge::check_rocm_status`,
        // which queries `torch.cuda.mem_get_info()` for real numbers). This used
        // to hardcode a specific GPU model and a plausible-looking VRAM baseline
        // (12288 / 1840 MB) regardless of the user's actual hardware, which
        // silently misrepresented every machine that isn't the developer's own.
        // Reporting 0/"Neznáme" here is honestly "not measured", not "measured
        // zero" — `is_rocm_ready` stays `false` until a real ROCm check confirms
        // it. Prefer wiring the frontend to `check_rocm_status` (on a slower
        // interval, since it shells out to WSL) over inventing numbers here.
        let gpu_vram_total_mb = 0u64;
        let gpu_vram_used_mb = 0u64;
        let gpu_vram_percent = 0.0f32;

        LiveSystemMetrics {
            host_ram_used_mb: used_ram_mb,
            host_ram_total_mb: total_ram_mb,
            host_ram_percent: ram_pct,
            cpu_usage_percent: cpu_pct,
            gpu_vram_used_mb,
            gpu_vram_total_mb,
            gpu_vram_percent,
            gpu_name: "Neznáme (spustite kontrolu ROCm)".to_string(),
            is_rocm_ready: false,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        }
    }
}
