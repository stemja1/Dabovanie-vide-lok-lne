use ai_dubbing_lib::wsl::executor::{ProcessErrorKind, WslExecutor};

#[test]
fn test_oom_diagnosis() {
    let mock_error_log = "torch.cuda.OutOfMemoryError: HIP out of memory. Tried to allocate 2.40 GiB (GPU 0; 12.00 GiB total capacity; 10.80 GiB already allocated)";
    let (kind, remedy) = WslExecutor::diagnose_error(mock_error_log);
    assert_eq!(kind, Some(ProcessErrorKind::OutOfMemoryGpu));
    assert!(remedy.unwrap().contains("MuseTalk"));
}

#[test]
fn test_xformers_diagnosis() {
    let mock_error_log =
        "ImportError: cannot import name 'memory_efficient_attention' from 'xformers.ops'";
    let (kind, remedy) = WslExecutor::diagnose_error(mock_error_log);
    assert_eq!(kind, Some(ProcessErrorKind::XformersIncompatible));
    assert!(remedy.unwrap().contains("ROCm Native SDPA Fallback"));
}
