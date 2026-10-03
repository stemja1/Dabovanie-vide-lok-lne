use ai_dubbing_lib::config::app_config::{AppConfig, LipsyncEngine};
use ai_dubbing_lib::pipeline::vram_estimator::VramEstimator;

#[test]
fn test_latentsync15_fits_in_12gb_vram() {
    let cfg = AppConfig {
        lipsync_engine: LipsyncEngine::LatentSync15,
        ..Default::default()
    };

    let budget = VramEstimator::calculate_budget(&cfg);
    assert!(
        budget.is_overall_safe,
        "LatentSync 1.5 sequential execution must fit in 12GB VRAM with headroom"
    );
    // Strictly less than: an estimate that exactly equals the card capacity
    // cannot run, and `peak <= total` made that case look fine.
    assert!(
        budget.peak_vram_mb < VramEstimator::TARGET_GPU_VRAM_MB,
        "peak VRAM {} must leave headroom under {}",
        budget.peak_vram_mb,
        VramEstimator::TARGET_GPU_VRAM_MB
    );
    assert!(
        budget.peak_ram_mb < VramEstimator::TARGET_SYSTEM_RAM_MB,
        "peak RAM {} must leave headroom under {}",
        budget.peak_ram_mb,
        VramEstimator::TARGET_SYSTEM_RAM_MB
    );
}

#[test]
fn test_musetalk_uses_far_less_vram_than_latentsync() {
    let latentsync = VramEstimator::calculate_budget(&AppConfig {
        lipsync_engine: LipsyncEngine::LatentSync15,
        ..Default::default()
    });
    let musetalk = VramEstimator::calculate_budget(&AppConfig {
        lipsync_engine: LipsyncEngine::MuseTalk,
        ..Default::default()
    });

    assert!(
        musetalk.peak_vram_mb < latentsync.peak_vram_mb,
        "MuseTalk ({}) must need strictly less VRAM than LatentSync ({}) - this is \
         the premise of the automatic OOM fallback",
        musetalk.peak_vram_mb,
        latentsync.peak_vram_mb
    );
}

#[test]
fn test_musetalk_fallback_resource_profile() {
    let cfg = AppConfig {
        lipsync_engine: LipsyncEngine::MuseTalk,
        ..Default::default()
    };

    let budget = VramEstimator::calculate_budget(&cfg);
    assert!(
        budget.peak_vram_mb <= 6000,
        "MuseTalk peak VRAM must be well below 6GB"
    );
}
