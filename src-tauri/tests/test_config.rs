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
    let cfg = AppConfig {
        tts_engine: TtsEngine::Kokoro,
        lipsync_batch_size: 12,
        ..Default::default()
    };

    let toml_str = cfg.to_toml_string().expect("Serialization should succeed");
    let parsed: AppConfig = AppConfig::from_toml_string(&toml_str).expect("Parsing should succeed");

    assert_eq!(parsed.tts_engine, TtsEngine::Kokoro);
    assert_eq!(parsed.lipsync_batch_size, 12);
    assert_eq!(parsed.wsl_distro, "Ubuntu-24.04");
}

#[test]
fn test_config_validation_boundaries() {
    let mut invalid_speed = AppConfig::default();
    invalid_speed.tts_speed_factor = f32::NAN;
    assert!(invalid_speed.validate().is_err());

    let mut invalid_batch = AppConfig::default();
    invalid_batch.lipsync_batch_size = 0;
    assert!(invalid_batch.validate().is_err());

    let mut invalid_distro = AppConfig::default();
    invalid_distro.wsl_distro = "   ".to_string();
    assert!(invalid_distro.validate().is_err());

    let mut invalid_ducking = AppConfig::default();
    invalid_ducking.ducking_level_db = 10.0;
    assert!(invalid_ducking.validate().is_err());

    assert!(AppConfig::default().validate().is_ok());
}
