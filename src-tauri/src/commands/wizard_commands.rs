use tauri::{AppHandle, Emitter, State};
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::wizard::checker::{DependencyChecker, SystemDiagnosticsReport};
use crate::wizard::installer::WizardInstaller;
use crate::wizard::models_manifest::{ModelsManifest, ModelManifestItem};
use crate::commands::config_commands::ConfigState;
use crate::wsl::executor::ProcessLogLine;

pub struct WizardState(pub Arc<WizardInstaller>);

#[tauri::command]
pub async fn run_system_diagnostics(
    config_state: State<'_, ConfigState>,
) -> Result<SystemDiagnosticsReport, String> {
    let cfg = {
        let guard = config_state.0.lock().map_err(|e| e.to_string())?;
        guard.clone()
    };

    DependencyChecker::run_full_check(&cfg.wsl_distro, &cfg.venv_path, &cfg.workspace_dir)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_models_manifest() -> Vec<ModelManifestItem> {
    ModelsManifest::get_all_models()
}

#[tauri::command]
pub async fn run_wizard_step(
    step_id: String,
    app_handle: AppHandle,
    config_state: State<'_, ConfigState>,
    wizard_state: State<'_, WizardState>,
) -> Result<bool, String> {
    let cfg = {
        let guard = config_state.0.lock().map_err(|e| e.to_string())?;
        guard.clone()
    };

    let installer = wizard_state.0.clone();
    installer.reset_cancel();

    let (tx, mut rx) = mpsc::unbounded_channel::<ProcessLogLine>();

    // Spawn task to forward logs to frontend via Tauri event
    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        while let Some(log) = rx.recv().await {
            let _ = app_handle_clone.emit("wizard_log_event", log);
        }
    });

    match step_id.as_str() {
        "wsl_install" => {
            installer.install_wsl2_ubuntu(&cfg.wsl_distro, Some(tx))
                .await
                .map_err(|e| e.to_string())
        }
        "system_packages" => {
            installer.install_system_packages(&cfg.wsl_distro, Some(tx))
                .await
                .map_err(|e| e.to_string())
        }
        "python_rocm" => {
            installer.setup_python_venv_and_rocm(&cfg.wsl_distro, &cfg.venv_path, &cfg.workspace_dir, Some(tx))
                .await
                .map_err(|e| e.to_string())
        }
        "lipsync_repos" => {
            installer.setup_lipsync_repos(&cfg.wsl_distro, &cfg.venv_path, &cfg.workspace_dir, Some(tx))
                .await
                .map_err(|e| e.to_string())
        }
        _ if step_id.starts_with("model_") => {
            let model_id = step_id.trim_start_matches("model_");
            installer.download_model_checkpoint(&cfg.wsl_distro, &cfg.venv_path, &cfg.workspace_dir, model_id, Some(tx))
                .await
                .map_err(|e| e.to_string())
        }
        _ => Err(format!("Neznámy inštalačný krok: {}", step_id)),
    }
}

#[tauri::command]
pub fn cancel_wizard_install(wizard_state: State<'_, WizardState>) {
    wizard_state.0.cancel();
}
