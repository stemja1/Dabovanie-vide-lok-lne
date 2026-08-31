use ai_dubbing_lib::config::app_config::{
    AppConfig, AsrDevice, AsrEngine, LipsyncEngine, TtsEngine,
};
use ai_dubbing_lib::pipeline::vram_estimator::VramEstimator;

#[test]
fn test_all_model_combinations_memory_headroom() {
    let asr_engines = [AsrEngine::WhisperSk, AsrEngine::FasterWhisper];
    let asr_devices = [AsrDevice::GpuRocm, AsrDevice::Cpu];
    let tts_engines = [TtsEngine::Piper, TtsEngine::Kokoro, TtsEngine::CoquiXtts];
    let lipsync_engines = [LipsyncEngine::LatentSync15, LipsyncEngine::MuseTalk];

    for asr in asr_engines {
        for dev in asr_devices {
            for tts in tts_engines {
                for lipsync in lipsync_engines {
                    let cfg = AppConfig {
                        asr_engine: asr,
                        asr_device: dev,
                        tts_engine: tts,
                        lipsync_engine: lipsync,
                        ..Default::default()
                    };

                    let budget = VramEstimator::calculate_budget(&cfg);
                    assert!(
                        budget.is_overall_safe,
                        "Budget must be safe for combination {:?} {:?} {:?} {:?}",
                        asr, dev, tts, lipsync
                    );
                    assert!(
                        budget.peak_vram_mb <= 12288,
                        "Peak VRAM cannot exceed 12288 MB"
                    );
                    assert!(
                        budget.peak_ram_mb <= 16384,
                        "Peak RAM cannot exceed 16384 MB"
                    );
                }
            }
        }
    }
}
