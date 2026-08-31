use crate::commands::config_commands::ConfigState;
use crate::pipeline::orchestrator::{PipelineExecutionState, PipelineOrchestrator};
use crate::pipeline::vram_estimator::{FullPipelineResourceBudget, VramEstimator};
use crate::wsl::executor::ProcessLogLine;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;

pub struct OrchestratorState(pub Arc<PipelineOrchestrator>);

#[tauri::command]
pub async fn set_pipeline_video(
    video_path: String,
    orchestrator_state: State<'_, OrchestratorState>,
    config_state: State<'_, ConfigState>,
) -> Result<PipelineExecutionState, String> {
    let distro = {
        let guard = config_state.0.lock().map_err(|e| e.to_string())?;
        guard.wsl_distro.clone()
    };

    orchestrator_state
        .0
        .set_input_video(&video_path, &distro)
        .await;
    let st = orchestrator_state.0.state.lock().await;
    Ok(st.clone())
}

#[tauri::command]
pub async fn get_pipeline_state(
    orchestrator_state: State<'_, OrchestratorState>,
) -> Result<PipelineExecutionState, String> {
    let st = orchestrator_state.0.state.lock().await;
    Ok(st.clone())
}

#[tauri::command]
pub async fn get_resource_budget(
    config_state: State<'_, ConfigState>,
) -> Result<FullPipelineResourceBudget, String> {
    let cfg = {
        let guard = config_state.0.lock().map_err(|e| e.to_string())?;
        guard.clone()
    };
    Ok(VramEstimator::calculate_budget(&cfg))
}

#[tauri::command]
pub async fn start_pipeline_execution(
    app_handle: AppHandle,
    orchestrator_state: State<'_, OrchestratorState>,
    config_state: State<'_, ConfigState>,
) -> Result<(), String> {
    let cfg = {
        let guard = config_state.0.lock().map_err(|e| e.to_string())?;
        guard.clone()
    };

    let orchestrator = orchestrator_state.0.clone();
    let (tx, mut rx) = mpsc::unbounded_channel::<ProcessLogLine>();

    // Forward logs to frontend
    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        while let Some(log) = rx.recv().await {
            let _ = app_handle_clone.emit("pipeline_log_event", log);
        }
    });

    // Run pipeline
    tokio::spawn(async move {
        let res = orchestrator.start_pipeline(cfg, Some(tx)).await;
        let st = orchestrator.state.lock().await.clone();
        let _ = app_handle.emit("pipeline_state_updated", st);
        if let Err(e) = res {
            let _ = app_handle.emit("pipeline_error_event", e.to_string());
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn continue_pipeline_after_review(
    app_handle: AppHandle,
    orchestrator_state: State<'_, OrchestratorState>,
    config_state: State<'_, ConfigState>,
) -> Result<(), String> {
    let cfg = {
        let guard = config_state.0.lock().map_err(|e| e.to_string())?;
        guard.clone()
    };

    let orchestrator = orchestrator_state.0.clone();
    let (tx, mut rx) = mpsc::unbounded_channel::<ProcessLogLine>();

    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        while let Some(log) = rx.recv().await {
            let _ = app_handle_clone.emit("pipeline_log_event", log);
        }
    });

    tokio::spawn(async move {
        let res = orchestrator.continue_after_review(cfg, Some(tx)).await;
        let st = orchestrator.state.lock().await.clone();
        let _ = app_handle.emit("pipeline_state_updated", st);
        if let Err(e) = res {
            let _ = app_handle.emit("pipeline_error_event", e.to_string());
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn run_single_stage(
    stage_index: usize,
    app_handle: AppHandle,
    orchestrator_state: State<'_, OrchestratorState>,
    config_state: State<'_, ConfigState>,
) -> Result<(), String> {
    let cfg = {
        let guard = config_state.0.lock().map_err(|e| e.to_string())?;
        guard.clone()
    };

    let orchestrator = orchestrator_state.0.clone();
    let (tx, mut rx) = mpsc::unbounded_channel::<ProcessLogLine>();

    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        while let Some(log) = rx.recv().await {
            let _ = app_handle_clone.emit("pipeline_log_event", log);
        }
    });

    tokio::spawn(async move {
        let res = orchestrator
            .run_single_stage(stage_index, &cfg, Some(tx))
            .await;
        let st = orchestrator.state.lock().await.clone();
        let _ = app_handle.emit("pipeline_state_updated", st);
        if let Err(e) = res {
            let _ = app_handle.emit("pipeline_error_event", e.to_string());
        }
    });

    Ok(())
}

#[tauri::command]
pub fn cancel_pipeline_execution(orchestrator_state: State<'_, OrchestratorState>) {
    orchestrator_state.0.cancel();
}
