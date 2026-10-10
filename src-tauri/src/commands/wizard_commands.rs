use crate::commands::config_commands::ConfigState;
use crate::error::{AppError, AppResult};
use crate::wizard::checker::{DependencyChecker, SystemDiagnosticsReport};
use crate::wizard::installer::WizardInstaller;
use crate::wizard::models_manifest::{ModelManifestItem, ModelsManifest};
use crate::wsl::executor::ProcessLogLine;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;

pub struct WizardState(pub Arc<WizardInstaller>);

#[tauri::command]
pub async fn run_system_diagnostics(
    config_state: State<'_, ConfigState>,
) -> AppResult<SystemDiagnosticsReport> {
    let cfg = {
        let guard = config_state.0.lock().map_err(|_| AppError::LockPoisoned)?;
        guard.clone()
    };

    Ok(
        DependencyChecker::run_full_check(&cfg.wsl_distro, &cfg.venv_path, &cfg.workspace_dir)
            .await?,
    )
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
) -> AppResult<bool> {
    let cfg = {
        let guard = config_state.0.lock().map_err(|_| AppError::LockPoisoned)?;
        guard.clone()
    };

    let installer = wizard_state.0.clone();
    // NEVYMAŽEME tu cancel flag. `handleRunAllSteps` v UI volá `run_wizard_step`
    // deväťkrát za sebou; predtým sa `reset_cancel()` volal na začiatku KAŽDÉHO
    // kroku, takže používateľovo "Zrušiť" platilo iba do konca práve
    // prebiehajúceho kroku a nasledujúci krok si myslel, že nič nezrušené.
    // Reset teraz robí výlučne nový používateľský beh - vid nižšie.
    //
    // Hranicu "nového behu" tu nepoznáme (príkaz sa volá 9x), preto flag
    // nekreslíme vôbec: `handleRunAllSteps` ho vyčistí pred svojim štartom cez
    // `run_wizard_step` s prázdnym `step_id`? Nie - jednoduchšie: `cancel()`
    // nastavuje flag a nič ho automaticky nezmaže; `reset_cancel()` je
    // explicitný a zýva jediným miestom, kde sa volá.
    let starting_new_run = step_id.is_empty() || step_id == "reset_cancel";
    if starting_new_run {
        installer.reset_cancel();
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<ProcessLogLine>();

    // Spawn task to forward logs to frontend via Tauri event
    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        while let Some(log) = rx.recv().await {
            let _ = app_handle_clone.emit("wizard_log_event", log);
        }
    });

    let success = match step_id.as_str() {
        "wsl_install" => {
            installer
                .install_wsl2_ubuntu(&cfg.wsl_distro, Some(tx))
                .await?
        }
        "system_packages" => {
            installer
                .install_system_packages(&cfg.wsl_distro, Some(tx))
                .await?
        }
        "python_rocm" => {
            installer
                .setup_python_venv_and_rocm(
                    &cfg.wsl_distro,
                    &cfg.venv_path,
                    &cfg.workspace_dir,
                    Some(tx),
                )
                .await?
        }
        "lipsync_repos" => {
            installer
                .setup_lipsync_repos(
                    &cfg.wsl_distro,
                    &cfg.venv_path,
                    &cfg.workspace_dir,
                    Some(tx),
                )
                .await?
        }
        _ if step_id.starts_with("model_") => {
            let model_id = step_id.trim_start_matches("model_");
            installer
                .download_model_checkpoint(
                    &cfg.wsl_distro,
                    &cfg.venv_path,
                    &cfg.workspace_dir,
                    model_id,
                    Some(tx),
                )
                .await?
        }
        _ => {
            return Err(AppError::Validation(format!(
                "Neznámy inštalačný krok: {}",
                step_id
            )))
        }
    };

    // Rozlíšené zrušenie od zlyhania. Python downloader pri zrušení končí
    // `sys.exit(130)`; bez tohto rozdelenia UI hlásilo "CHYBA v kroku" po tom,
    // čo používateľ sám zrušil inštaláciu, a hlásilo to aj vtedy, keď je
    // `success == false` preto, že predchádzajúci krok bol zrušený.
    if installer.is_cancelled() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "INŠTALÁCIA ZRUŠENÁ POUŽÍVATEĽOM"
        )));
    }

    if !success {
        return Err(AppError::Internal(anyhow::anyhow!(
            "Inštalačný krok '{}' zlyhal. Skontrolujte chybové hlásenie v logu.",
            step_id
        )));
    }

    Ok(true)
}

#[tauri::command]
pub fn cancel_wizard_install(wizard_state: State<'_, WizardState>) {
    wizard_state.0.cancel();
}

/// Explicitne zmaže príznak zrušenia pred novým inštalačným behom.
#[tauri::command]
pub fn reset_wizard_cancel(wizard_state: State<'_, WizardState>) {
    wizard_state.0.reset_cancel();
}
