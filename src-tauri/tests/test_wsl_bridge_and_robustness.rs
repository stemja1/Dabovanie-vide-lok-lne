use ai_dubbing_lib::config::app_config::AppConfig;
use ai_dubbing_lib::monitor::system_stats::SystemStatsMonitor;
use ai_dubbing_lib::pipeline::orchestrator::PipelineOrchestrator;
use ai_dubbing_lib::wizard::installer::WizardInstaller;
use ai_dubbing_lib::wsl::bridge::WslBridge;

#[test]
fn test_default_implementations() {
    let _monitor = SystemStatsMonitor::default();
    let _installer = WizardInstaller::default();
    let orchestrator = PipelineOrchestrator::default();
    assert!(!orchestrator
        .is_cancelled
        .load(std::sync::atomic::Ordering::SeqCst));
}

#[tokio::test]
async fn test_orchestrator_stage_out_of_bounds_rejection() {
    let orchestrator = PipelineOrchestrator::new();
    let cfg = AppConfig::default();
    let res = orchestrator.run_single_stage(999, &cfg, None).await;
    assert!(res.is_err(), "Out of bounds stage index must return Err");
    assert!(res.unwrap_err().to_string().contains("mimo rozsahu"));

    // A rejected index must not leave the orchestrator permanently busy: the
    // bounds check used to run *after* `is_running` was claimed, so the whole
    // app refused every further run until restart.
    let state = orchestrator.state.lock().await;
    assert!(
        !state.is_running,
        "a rejected stage index must not leave is_running stuck at true"
    );
    assert_eq!(
        state.current_stage_index, 0,
        "a rejected stage index must not corrupt current_stage_index"
    );
}

#[tokio::test]
async fn test_single_stage_refuses_to_start_while_pipeline_runs() {
    let orchestrator = PipelineOrchestrator::new();
    {
        let mut state = orchestrator.state.lock().await;
        state.is_running = true;
    }
    let cfg = AppConfig::default();
    let res = orchestrator.run_single_stage(0, &cfg, None).await;
    assert!(
        res.is_err(),
        "running a single stage during an active run must be refused"
    );
    assert!(res.unwrap_err().to_string().contains("už beží"));
}

#[tokio::test]
async fn test_cancel_flag_does_not_block_next_single_stage_run() {
    let orchestrator = PipelineOrchestrator::new();
    // Simulate a user cancelling an earlier run. The flag is sticky by design
    // (only the pipeline loop observes it), so a later standalone stage must
    // clear it - otherwise every retry was killed before it did anything.
    orchestrator.cancel();
    assert!(orchestrator
        .is_cancelled
        .load(std::sync::atomic::Ordering::SeqCst));

    let cfg = AppConfig::default();
    // An out-of-range index returns Err after `reset_cancel()` runs, which is
    // enough to observe the flag was cleared.
    let _ = orchestrator.run_single_stage(999, &cfg, None).await;
    assert!(
        !orchestrator
            .is_cancelled
            .load(std::sync::atomic::Ordering::SeqCst),
        "a stale cancel flag must not silently no-op the next stage"
    );
}

#[tokio::test]
async fn test_orchestrator_continue_requires_paused_state() {
    let orchestrator = PipelineOrchestrator::new();
    let cfg = AppConfig::default();
    let res = orchestrator.continue_after_review(cfg, None).await;
    assert!(
        res.is_err(),
        "continue_after_review must fail when not paused"
    );
    assert!(res
        .unwrap_err()
        .to_string()
        .contains("nie je v stave pozastavenia"));
}

#[test]
fn test_wsl_utf16_decoding() {
    // Plain UTF-8
    let ascii_bytes = b"NAME    STATE    VERSION\n* Ubuntu-24.04    Running    2\n";
    let decoded = WslBridge::decode_wsl_output(ascii_bytes);
    assert!(decoded.contains("Ubuntu-24.04"));

    // UTF-16LE with BOM (0xFF, 0xFE)
    let mut utf16_bom: Vec<u8> = vec![0xFF, 0xFE];
    for ch in "Ubuntu-24.04 Running 2".encode_utf16() {
        utf16_bom.extend_from_slice(&ch.to_le_bytes());
    }
    let decoded_bom = WslBridge::decode_wsl_output(&utf16_bom);
    assert!(decoded_bom.contains("Ubuntu-24.04"));

    // UTF-16LE without BOM (null bytes interleaved)
    let mut utf16_nobom: Vec<u8> = Vec::new();
    for ch in "Ubuntu-24.04 Running 2".encode_utf16() {
        utf16_nobom.extend_from_slice(&ch.to_le_bytes());
    }
    let decoded_nobom = WslBridge::decode_wsl_output(&utf16_nobom);
    assert!(decoded_nobom.contains("Ubuntu-24.04"));
}

#[test]
fn test_wsl_list_parser() {
    let sample_output = r"
  NAME            STATE           VERSION
* Ubuntu-24.04    Running         2
  docker-desktop  Stopped         2
  Ubuntu-22.04    Stopped         1
";
    let distros = WslBridge::parse_wsl_list_output(sample_output);
    assert_eq!(distros.len(), 3);
    assert_eq!(distros[0].name, "Ubuntu-24.04");
    assert!(distros[0].is_default);
    assert_eq!(distros[0].state, "Running");
    assert_eq!(distros[0].version, 2);

    assert_eq!(distros[1].name, "docker-desktop");
    assert!(!distros[1].is_default);

    assert_eq!(distros[2].name, "Ubuntu-22.04");
    assert_eq!(distros[2].version, 1);
}
