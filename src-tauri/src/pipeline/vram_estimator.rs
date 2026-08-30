use serde::{Deserialize, Serialize};
use crate::config::app_config::{AppConfig, AsrDevice, LipsyncEngine, TtsEngine};
use crate::pipeline::stages::PipelineStageId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResourceEstimate {
    pub stage_id: PipelineStageId,
    pub stage_name: String,
    pub estimated_ram_mb: u64,
    pub estimated_vram_mb: u64,
    pub is_gpu_active: bool,
    pub max_supported_vram_mb: u64,
    pub max_supported_ram_mb: u64,
    pub is_safe: bool,
    pub warning_message: Option<String>,
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullPipelineResourceBudget {
    pub stages: Vec<StageResourceEstimate>,
    pub peak_vram_mb: u64,
    pub peak_ram_mb: u64,
    pub total_system_ram_mb: u64,
    pub total_gpu_vram_mb: u64,
    pub is_overall_safe: bool,
    pub hardware_profile: String,
}

pub struct VramEstimator;

impl VramEstimator {
    pub const TARGET_GPU_VRAM_MB: u64 = 12288; // AMD RX 7700 XT (12 GB)
    pub const TARGET_SYSTEM_RAM_MB: u64 = 16384; // 16 GB Host RAM

    pub fn calculate_budget(config: &AppConfig) -> FullPipelineResourceBudget {
        let mut stages = Vec::new();

        // 1. Demux
        stages.push(StageResourceEstimate {
            stage_id: PipelineStageId::Demux,
            stage_name: "Extrakcia & Demuxing".to_string(),
            estimated_ram_mb: 512,
            estimated_vram_mb: 0,
            is_gpu_active: false,
            max_supported_vram_mb: Self::TARGET_GPU_VRAM_MB,
            max_supported_ram_mb: Self::TARGET_SYSTEM_RAM_MB,
            is_safe: true,
            warning_message: None,
            recommendation: None,
        });

        // 2. ASR
        let (asr_vram, asr_ram, asr_gpu, asr_warn) = match (config.asr_engine, config.asr_device) {
            (_, AsrDevice::Cpu) => (0, 3500, false, Some("ASR beží na CPU — spracovanie bude pomalšie, ale ušetrí VRAM.".to_string())),
            (crate::config::app_config::AsrEngine::WhisperSk, AsrDevice::GpuRocm) => (5600, 4200, true, None),
            (crate::config::app_config::AsrEngine::FasterWhisper, AsrDevice::GpuRocm) => (4200, 3200, true, Some("faster-whisper vyžaduje overenie ROCm podpory v CTranslate2.".to_string())),
        };
        stages.push(StageResourceEstimate {
            stage_id: PipelineStageId::Asr,
            stage_name: "Slovenský ASR (Whisper)".to_string(),
            estimated_ram_mb: asr_ram,
            estimated_vram_mb: asr_vram,
            is_gpu_active: asr_gpu,
            max_supported_vram_mb: Self::TARGET_GPU_VRAM_MB,
            max_supported_ram_mb: Self::TARGET_SYSTEM_RAM_MB,
            is_safe: asr_vram <= Self::TARGET_GPU_VRAM_MB,
            warning_message: asr_warn,
            recommendation: None,
        });

        // 3. Translation
        stages.push(StageResourceEstimate {
            stage_id: PipelineStageId::Translate,
            stage_name: "Preklad SK → ZH (NLLB-200)".to_string(),
            estimated_ram_mb: 2048,
            estimated_vram_mb: 2560,
            is_gpu_active: true,
            max_supported_vram_mb: Self::TARGET_GPU_VRAM_MB,
            max_supported_ram_mb: Self::TARGET_SYSTEM_RAM_MB,
            is_safe: true,
            warning_message: None,
            recommendation: None,
        });

        // 4. Review
        stages.push(StageResourceEstimate {
            stage_id: PipelineStageId::Review,
            stage_name: "Interaktívna Kontrola Metadát".to_string(),
            estimated_ram_mb: 256,
            estimated_vram_mb: 0,
            is_gpu_active: false,
            max_supported_vram_mb: Self::TARGET_GPU_VRAM_MB,
            max_supported_ram_mb: Self::TARGET_SYSTEM_RAM_MB,
            is_safe: true,
            warning_message: None,
            recommendation: None,
        });

        // 5. TTS
        let (tts_vram, tts_ram, tts_gpu, tts_warn, tts_rec) = match config.tts_engine {
            TtsEngine::Piper => (256, 800, false, None, Some("Piper TTS je komerčne bezpečný (MIT) a ultra-ľahký na RAM/VRAM.".to_string())),
            TtsEngine::Kokoro => (1800, 1500, true, None, Some("Kokoro TTS poskytuje vysokú kvalitu s Apache 2.0 licenciou.".to_string())),
            TtsEngine::CoquiXtts => (3600, 3200, true, Some("UPOZORNENIE: Coqui XTTS-v2 je pod CPML nekomerčnou licenciou.".to_string()), None),
        };
        stages.push(StageResourceEstimate {
            stage_id: PipelineStageId::Tts,
            stage_name: "Syntéza Reči (TTS)".to_string(),
            estimated_ram_mb: tts_ram,
            estimated_vram_mb: tts_vram,
            is_gpu_active: tts_gpu,
            max_supported_vram_mb: Self::TARGET_GPU_VRAM_MB,
            max_supported_ram_mb: Self::TARGET_SYSTEM_RAM_MB,
            is_safe: true,
            warning_message: tts_warn,
            recommendation: tts_rec,
        });

        // 6. Lip-sync
        let (ls_vram, ls_ram, ls_warn, ls_rec) = match config.lipsync_engine {
            LipsyncEngine::LatentSync15 => (
                7680,
                6144,
                if config.rocm_sdpa_fallback {
                    None
                } else {
                    Some("Pozor: Bez 'ROCm SDPA Fallback' môže xFormers zlyhať na AMD architektúre.".to_string())
                },
                Some("LatentSync 1.5 bezpečne vojde do 12 GB VRAM (~7.5 GB spotreba). Pri OOM zlyhaní sa automaticky aktivuje MuseTalk fallback.".to_string())
            ),
            LipsyncEngine::MuseTalk => (
                4600,
                4096,
                None,
                Some("MuseTalk vyžaduje len ~4.5 GB VRAM a beží 2-3x rýchlejšie.".to_string())
            ),
        };
        stages.push(StageResourceEstimate {
            stage_id: PipelineStageId::Lipsync,
            stage_name: "Lip-Sync Animácia".to_string(),
            estimated_ram_mb: ls_ram,
            estimated_vram_mb: ls_vram,
            is_gpu_active: true,
            max_supported_vram_mb: Self::TARGET_GPU_VRAM_MB,
            max_supported_ram_mb: Self::TARGET_SYSTEM_RAM_MB,
            is_safe: ls_vram <= Self::TARGET_GPU_VRAM_MB,
            warning_message: ls_warn,
            recommendation: ls_rec,
        });

        // 7. Mux
        stages.push(StageResourceEstimate {
            stage_id: PipelineStageId::Mux,
            stage_name: "Muxing & Audio Ducking".to_string(),
            estimated_ram_mb: 1024,
            estimated_vram_mb: 0,
            is_gpu_active: false,
            max_supported_vram_mb: Self::TARGET_GPU_VRAM_MB,
            max_supported_ram_mb: Self::TARGET_SYSTEM_RAM_MB,
            is_safe: true,
            warning_message: None,
            recommendation: None,
        });

        let peak_vram = stages.iter().map(|s| s.estimated_vram_mb).max().unwrap_or(0);
        let peak_ram = stages.iter().map(|s| s.estimated_ram_mb).max().unwrap_or(0);

        let overall_safe = peak_vram <= Self::TARGET_GPU_VRAM_MB && peak_ram <= Self::TARGET_SYSTEM_RAM_MB;

        FullPipelineResourceBudget {
            stages,
            peak_vram_mb: peak_vram,
            peak_ram_mb: peak_ram,
            total_system_ram_mb: Self::TARGET_SYSTEM_RAM_MB,
            total_gpu_vram_mb: Self::TARGET_GPU_VRAM_MB,
            is_overall_safe: overall_safe,
            hardware_profile: "AMD Ryzen 5 5600 (16GB RAM) + Radeon RX 7700 XT (12GB VRAM)".to_string(),
        }
    }
}
