use ai_dubbing_lib::config::app_config::{
    AppConfig, AsrDevice, AsrEngine, LipsyncEngine, TtsEngine,
};
use ai_dubbing_lib::pipeline::vram_estimator::VramEstimator;

use ai_dubbing_lib::pipeline::stages::PipelineStageId;

/// Every engine combination must stay inside the reference envelope *with
/// headroom*, not merely below the limit. `is_overall_safe` used to compare the
/// peak against the same constant the total came from, so it was always true and
/// this test could not fail - it asserted nothing about the estimator.
#[test]
fn test_all_model_combinations_memory_headroom() {
    let asr_engines = [AsrEngine::WhisperSk, AsrEngine::FasterWhisper];
    let asr_devices = [AsrDevice::GpuRocm, AsrDevice::Cpu];
    let tts_engines = [TtsEngine::Piper, TtsEngine::Kokoro, TtsEngine::CoquiXtts];
    let lipsync_engines = [LipsyncEngine::LatentSync15, LipsyncEngine::MuseTalk];

    let mut checked = 0usize;

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

                    // The aggregate flag must agree with the per-stage numbers,
                    // otherwise the UI shows "safe" while a stage says otherwise.
                    assert_eq!(
                        budget.is_overall_safe,
                        budget.stages.iter().all(|s| s.is_safe),
                        "aggregate flag disagrees with per-stage safety for {:?} {:?} {:?} {:?}",
                        asr,
                        dev,
                        tts,
                        lipsync
                    );

                    assert!(
                        budget.is_overall_safe,
                        "Budget must be safe for combination {:?} {:?} {:?} {:?}",
                        asr, dev, tts, lipsync
                    );

                    // Headroom must actually be left over, not just "not over".
                    assert!(
                        budget.peak_vram_mb < VramEstimator::TARGET_GPU_VRAM_MB,
                        "peak VRAM {} leaves no headroom under {}",
                        budget.peak_vram_mb,
                        VramEstimator::TARGET_GPU_VRAM_MB
                    );
                    assert!(
                        budget.peak_ram_mb < VramEstimator::TARGET_SYSTEM_RAM_MB,
                        "peak RAM {} leaves no headroom under {}",
                        budget.peak_ram_mb,
                        VramEstimator::TARGET_SYSTEM_RAM_MB
                    );

                    // And the reported peak must really be the maximum of the
                    // per-stage estimates, not an unrelated aggregate.
                    let stage_max = budget
                        .stages
                        .iter()
                        .map(|s| s.estimated_vram_mb)
                        .max()
                        .unwrap_or(0);
                    assert_eq!(
                        budget.peak_vram_mb, stage_max,
                        "peak_vram_mb must equal the largest per-stage estimate"
                    );

                    checked += 1;
                }
            }
        }
    }

    assert_eq!(checked, 24, "all 24 engine combinations must be covered");
}

/// A GPU-bound configuration must be recognisable as GPU-bound, otherwise the
/// VRAM meter silently degrades into a RAM-only view.
#[test]
fn test_gpu_active_stages_are_reported() {
    let cfg = AppConfig {
        asr_device: AsrDevice::GpuRocm,
        lipsync_engine: LipsyncEngine::LatentSync15,
        ..Default::default()
    };
    let budget = VramEstimator::calculate_budget(&cfg);

    assert!(
        budget.stages.iter().filter(|s| s.is_gpu_active).count() > 0,
        "a ROCm GPU configuration must mark at least one stage as GPU-active"
    );

    // ffmpeg-only stages must not be billed GPU memory.
    let demux = budget
        .stages
        .iter()
        .find(|s| s.stage_id == PipelineStageId::Demux)
        .expect("demux stage must exist");
    assert_eq!(demux.estimated_vram_mb, 0);
    assert!(!demux.is_gpu_active);
}
