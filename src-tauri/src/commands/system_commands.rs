use tauri::State;
use crate::monitor::system_stats::{LiveSystemMetrics, SystemStatsMonitor};
use crate::wsl::bridge::{RocmStatusInfo, WslBridge, WslStatusInfo};
use crate::commands::config_commands::ConfigState;

pub struct MonitorState(pub SystemStatsMonitor);

#[tauri::command]
pub fn get_live_system_metrics(monitor_state: State<'_, MonitorState>) -> LiveSystemMetrics {
    monitor_state.0.get_metrics()
}

#[tauri::command]
pub async fn check_wsl_status(config_state: State<'_, ConfigState>) -> Result<WslStatusInfo, String> {
    let distro = {
        let guard = config_state.0.lock().map_err(|e| e.to_string())?;
        guard.wsl_distro.clone()
    };
    WslBridge::detect_wsl_status(&distro).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_rocm_status(config_state: State<'_, ConfigState>) -> Result<RocmStatusInfo, String> {
    let (distro, venv) = {
        let guard = config_state.0.lock().map_err(|e| e.to_string())?;
        (guard.wsl_distro.clone(), guard.venv_path.clone())
    };
    WslBridge::check_rocm_status(&distro, &venv).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_path_in_explorer(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| e.to_string())
}
