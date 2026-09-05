pub mod commands;
pub mod config;
pub mod error;
pub mod monitor;
pub mod pipeline;
pub mod wizard;
pub mod wsl;

use commands::*;
use config::app_config::AppConfig;
use monitor::system_stats::SystemStatsMonitor;
use pipeline::orchestrator::PipelineOrchestrator;
use std::sync::{Arc, Mutex};
use wizard::installer::WizardInstaller;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    let initial_config = AppConfig::load_or_default();
    let config_state = ConfigState(Mutex::new(initial_config));
    let wizard_state = WizardState(Arc::new(WizardInstaller::new()));
    let orchestrator_state = OrchestratorState(Arc::new(PipelineOrchestrator::new()));
    let monitor_state = MonitorState(SystemStatsMonitor::new());

    tauri::Builder::default()
        .manage(config_state)
        .manage(wizard_state)
        .manage(orchestrator_state)
        .manage(monitor_state)
        .invoke_handler(tauri::generate_handler![
            // Config
            get_config,
            save_config,
            reset_config_to_default,
            export_config_toml,
            import_config_toml,
            // Wizard & Diagnostics
            run_system_diagnostics,
            get_models_manifest,
            run_wizard_step,
            cancel_wizard_install,
            // Pipeline & VRAM
            set_pipeline_video,
            get_pipeline_state,
            get_resource_budget,
            start_pipeline_execution,
            continue_pipeline_after_review,
            run_single_stage,
            cancel_pipeline_execution,
            // Metadata Editor
            load_utterance_metadata,
            save_utterance_metadata,
            get_demo_utterance_metadata,
            update_utterance_item,
            split_utterance_item,
            merge_utterance_items,
            // System & Stats
            get_live_system_metrics,
            check_wsl_status,
            check_rocm_status,
            open_path_in_explorer,
        ])
        .run(tauri::generate_context!())
        .expect("Chyba pri spúšťaní AI Dabing Štúdia");
}
