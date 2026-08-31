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
