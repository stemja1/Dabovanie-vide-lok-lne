use crate::commands::config_commands::ConfigState;
use crate::error::{AppError, AppResult};
use crate::monitor::system_stats::{LiveSystemMetrics, SystemStatsMonitor};
use crate::wsl::bridge::{RocmStatusInfo, WslBridge, WslStatusInfo};
use tauri::State;

pub struct MonitorState(pub SystemStatsMonitor);

#[tauri::command]
pub fn get_live_system_metrics(monitor_state: State<'_, MonitorState>) -> LiveSystemMetrics {
    monitor_state.0.get_metrics()
}

#[tauri::command]
pub async fn check_wsl_status(config_state: State<'_, ConfigState>) -> AppResult<WslStatusInfo> {
    let distro = {
        let guard = config_state.0.lock().map_err(|_| AppError::LockPoisoned)?;
        guard.wsl_distro.clone()
    };
    Ok(WslBridge::detect_wsl_status(&distro).await?)
}

#[tauri::command]
pub async fn check_rocm_status(
    config_state: State<'_, ConfigState>,
) -> AppResult<RocmStatusInfo> {
    let (distro, venv) = {
        let guard = config_state.0.lock().map_err(|_| AppError::LockPoisoned)?;
        (guard.wsl_distro.clone(), guard.venv_path.clone())
    };
    Ok(WslBridge::check_rocm_status(&distro, &venv).await?)
}

#[tauri::command]
pub fn open_path_in_explorer(path: String) -> AppResult<()> {
    open::that(&path).map_err(AppError::Io)
}
