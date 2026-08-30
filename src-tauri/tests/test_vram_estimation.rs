use ai_dubbing_lib::config::app_config::{AppConfig, LipsyncEngine};
use ai_dubbing_lib::pipeline::vram_estimator::VramEstimator;

#[test]
fn test_latentsync15_fits_in_12gb_vram() {
    let mut cfg = AppConfig::default();
    cfg.lipsync_engine = LipsyncEngine::LatentSync15;

    let budget = VramEstimator::calculate_budget(&cfg);
    assert!(budget.is_overall_safe, "LatentSync 1.5 sequential execution must fit in 12GB VRAM");
    assert!(budget.peak_vram_mb <= 12288, "Peak VRAM cannot exceed 12288 MB");
    assert!(budget.peak_ram_mb <= 16384, "Peak RAM cannot exceed 16384 MB");
}

#[test]
fn test_musetalk_fallback_resource_profile() {
    let mut cfg = AppConfig::default();
    cfg.lipsync_engine = LipsyncEngine::MuseTalk;

    let budget = VramEstimator::calculate_budget(&cfg);
    assert!(budget.peak_vram_mb <= 6000, "MuseTalk peak VRAM must be well below 6GB");
}
