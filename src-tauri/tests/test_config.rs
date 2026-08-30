use ai_dubbing_lib::config::app_config::{AppConfig, LipsyncEngine, TtsEngine};

#[test]
fn test_default_config() {
    let cfg = AppConfig::default();
    assert_eq!(cfg.wsl_distro, "Ubuntu-24.04");
    assert_eq!(cfg.tts_engine, TtsEngine::Piper);
    assert_eq!(cfg.lipsync_engine, LipsyncEngine::LatentSync15);
    assert!(cfg.rocm_sdpa_fallback);
    assert!(cfg.lipsync_fallback_on_oom);
}

#[test]
fn test_toml_roundtrip() {
    let mut cfg = AppConfig::default();
    cfg.tts_engine = TtsEngine::Kokoro;
    cfg.lipsync_batch_size = 12;

    let toml_str = cfg.to_toml_string().expect("Serialization should succeed");
    let parsed: AppConfig = AppConfig::from_toml_string(&toml_str).expect("Parsing should succeed");

    assert_eq!(parsed.tts_engine, TtsEngine::Kokoro);
    assert_eq!(parsed.lipsync_batch_size, 12);
    assert_eq!(parsed.wsl_distro, "Ubuntu-24.04");
}
