use crate::commands::config_commands::ConfigState;
use crate::error::{AppError, AppResult};
use crate::pipeline::orchestrator::{PipelineExecutionState, PipelineOrchestrator};
use crate::pipeline::vram_estimator::{FullPipelineResourceBudget, VramEstimator};
use crate::wsl::executor::ProcessLogLine;
use std::future::Future;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;

pub struct OrchestratorState(pub Arc<PipelineOrchestrator>);

/// Spawns a task that forwards every log line received on `rx` to the
/// frontend as a `pipeline_log_event`. Every command below that kicks off
/// pipeline work used to copy-paste this exact channel-draining loop; now
/// it's one place to change if the event name or forwarding logic ever
/// needs to.
fn spawn_log_forwarder(app_handle: AppHandle, mut rx: mpsc::UnboundedReceiver<ProcessLogLine>) {
    tokio::spawn(async move {
        while let Some(log) = rx.recv().await {
            let _ = app_handle.emit("pipeline_log_event", log);
        }
    });
}

/// Awaits `work` (the actual orchestrator call), then emits the resulting
/// pipeline state as `pipeline_state_updated` and, on failure, the error as
/// `pipeline_error_event`. This is the tail end that was previously
/// duplicated inside every `tokio::spawn` block in this file.
async fn run_and_report<F>(app_handle: AppHandle, orchestrator: Arc<PipelineOrchestrator>, work: F)
where
    F: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let res = work.await;
    let st = orchestrator.state.lock().await.clone();
    let _ = app_handle.emit("pipeline_state_updated", st);
    if let Err(e) = res {
        // Full anyhow context chain, not just the outermost frame -- see
        // AppError::Internal's rationale in error.rs for why `{:#}` matters.
        let _ = app_handle.emit("pipeline_error_event", format!("{:#}", e));
    }
}

#[tauri::command]
pub async fn set_pipeline_video(
    video_path: String,
    orchestrator_state: State<'_, OrchestratorState>,
    config_state: State<'_, ConfigState>,
) -> AppResult<PipelineExecutionState> {
    let distro = {
        let guard = config_state.0.lock().map_err(|_| AppError::LockPoisoned)?;
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
) -> AppResult<PipelineExecutionState> {
    let st = orchestrator_state.0.state.lock().await;
    Ok(st.clone())
}

#[tauri::command]
pub async fn get_resource_budget(
    config_state: State<'_, ConfigState>,
) -> AppResult<FullPipelineResourceBudget> {
    let cfg = {
        let guard = config_state.0.lock().map_err(|_| AppError::LockPoisoned)?;
        guard.clone()
    };
    Ok(VramEstimator::calculate_budget(&cfg))
}

#[tauri::command]
pub async fn start_pipeline_execution(
    app_handle: AppHandle,
    orchestrator_state: State<'_, OrchestratorState>,
    config_state: State<'_, ConfigState>,
) -> AppResult<()> {
    let cfg = {
        let guard = config_state.0.lock().map_err(|_| AppError::LockPoisoned)?;
        guard.clone()
    };

    let orchestrator = orchestrator_state.0.clone();
    let (tx, rx) = mpsc::unbounded_channel::<ProcessLogLine>();
    spawn_log_forwarder(app_handle.clone(), rx);

    tokio::spawn(run_and_report(app_handle, orchestrator.clone(), async move {
        orchestrator.start_pipeline(cfg, Some(tx)).await
    }));

    Ok(())
}

#[tauri::command]
pub async fn continue_pipeline_after_review(
    app_handle: AppHandle,
    orchestrator_state: State<'_, OrchestratorState>,
    config_state: State<'_, ConfigState>,
) -> AppResult<()> {
    let cfg = {
        let guard = config_state.0.lock().map_err(|_| AppError::LockPoisoned)?;
        guard.clone()
    };

    let orchestrator = orchestrator_state.0.clone();
    let (tx, rx) = mpsc::unbounded_channel::<ProcessLogLine>();
    spawn_log_forwarder(app_handle.clone(), rx);

    tokio::spawn(run_and_report(app_handle, orchestrator.clone(), async move {
        orchestrator.continue_after_review(cfg, Some(tx)).await
    }));

    Ok(())
}

#[tauri::command]
pub async fn run_single_stage(
    stage_index: usize,
    app_handle: AppHandle,
    orchestrator_state: State<'_, OrchestratorState>,
    config_state: State<'_, ConfigState>,
) -> AppResult<()> {
    let cfg = {
        let guard = config_state.0.lock().map_err(|_| AppError::LockPoisoned)?;
        guard.clone()
    };

    let orchestrator = orchestrator_state.0.clone();
    let (tx, rx) = mpsc::unbounded_channel::<ProcessLogLine>();
    spawn_log_forwarder(app_handle.clone(), rx);

    tokio::spawn(run_and_report(app_handle, orchestrator.clone(), async move {
        orchestrator
            .run_single_stage(stage_index, &cfg, Some(tx))
            .await
    }));

    Ok(())
}

#[tauri::command]
pub fn cancel_pipeline_execution(orchestrator_state: State<'_, OrchestratorState>) {
    orchestrator_state.0.cancel();
}
