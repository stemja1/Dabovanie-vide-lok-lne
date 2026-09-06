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
pub async fn check_rocm_status(config_state: State<'_, ConfigState>) -> AppResult<RocmStatusInfo> {
    let (distro, venv) = {
        let guard = config_state.0.lock().map_err(|_| AppError::LockPoisoned)?;
        (guard.wsl_distro.clone(), guard.venv_path.clone())
    };
    Ok(WslBridge::check_rocm_status(&distro, &venv).await?)
}

#[tauri::command]
pub async fn pick_video_dialog() -> AppResult<Option<String>> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter(
            "Video súbory (*.mp4, *.mkv, *.mov, *.webm, *.avi)",
            &["mp4", "mkv", "mov", "webm", "avi", "flv", "wmv", "m4v"],
        )
        .set_title("Vyberte slovenské video na dabovanie")
        .pick_file()
        .await;

    Ok(file.map(|f| f.path().to_string_lossy().to_string()))
}

#[tauri::command]
pub fn open_path_in_explorer(path: String) -> AppResult<()> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "Cesta k priečinku alebo súboru nesmie byť prázdna.".to_string(),
        ));
    }

    if trimmed.contains('\0') {
        return Err(AppError::Validation(
            "Zadaná cesta obsahuje neplatné nulové bajty.".to_string(),
        ));
    }

    let p = std::path::Path::new(trimmed);
    if !p.exists() {
        return Err(AppError::Validation(format!(
            "Zadaná cesta neexistuje na disku: '{}'",
            trimmed
        )));
    }

    let canonical = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    open::that(&canonical).map_err(AppError::Io)
}
