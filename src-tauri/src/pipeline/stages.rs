use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStageId {
    Demux,
    Asr,
    Translate,
    Review,
    Tts,
    Lipsync,
    Mux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    Idle,
    Running,
    ReviewPaused,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStageInfo {
    pub id: PipelineStageId,
    pub name: String,
    pub description: String,
    pub status: StageStatus,
    pub progress_percent: f32,
    pub started_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
    pub estimated_vram_gb: f32,
    pub estimated_ram_gb: f32,
    pub engine_badge: String,
    pub is_gpu_accelerated: bool,
    pub error_message: Option<String>,
    pub user_suggestion: Option<String>,
}

pub struct StageFactory;

impl StageFactory {
    pub fn build_default_stages() -> Vec<PipelineStageInfo> {
        vec![
            PipelineStageInfo {
                id: PipelineStageId::Demux,
                name: "1. Extrakcia & Demuxing".to_string(),
                description: "Izolácia zvukovej stopy, detekcia hlasových segmentov a normalizácia formátu (FFmpeg)".to_string(),
                status: StageStatus::Idle,
                progress_percent: 0.0,
                started_at_ms: None,
                completed_at_ms: None,
                estimated_vram_gb: 0.0,
                estimated_ram_gb: 0.5,
                engine_badge: "FFmpeg CLI".to_string(),
                is_gpu_accelerated: false,
                error_message: None,
                user_suggestion: None,
            },
            PipelineStageInfo {
                id: PipelineStageId::Asr,
                name: "2. Slovenský ASR (Prepis)".to_string(),
                description: "Automatické rozpoznávanie slovenskej reči s presnými časovými značkami slov (Whisper-SK / faster-whisper)".to_string(),
                status: StageStatus::Idle,
                progress_percent: 0.0,
                started_at_ms: None,
                completed_at_ms: None,
                estimated_vram_gb: 5.5,
                estimated_ram_gb: 4.5,
                engine_badge: "Whisper-SK (ROCm)".to_string(),
                is_gpu_accelerated: true,
                error_message: None,
                user_suggestion: None,
            },
            PipelineStageInfo {
                id: PipelineStageId::Translate,
                name: "3. Preklad SK → ZH".to_string(),
                description: "Neurónový preklad slovenských viet do zjednodušenej čínštiny (NLLB-200)".to_string(),
                status: StageStatus::Idle,
                progress_percent: 0.0,
                started_at_ms: None,
                completed_at_ms: None,
                estimated_vram_gb: 2.5,
                estimated_ram_gb: 2.0,
                engine_badge: "NLLB-200 (PyTorch)".to_string(),
                is_gpu_accelerated: true,
                error_message: None,
                user_suggestion: None,
            },
            PipelineStageInfo {
                id: PipelineStageId::Review,
                name: "4. Kontrola Metadát & Prekladu".to_string(),
                description: "Interaktívna kontrola a editácia utterance_metadata JSON priamo v GUI pred syntézou reči".to_string(),
                status: StageStatus::Idle,
                progress_percent: 0.0,
                started_at_ms: None,
                completed_at_ms: None,
                estimated_vram_gb: 0.0,
                estimated_ram_gb: 0.2,
                engine_badge: "Interaktívny Editor".to_string(),
                is_gpu_accelerated: false,
                error_message: None,
                user_suggestion: None,
            },
            PipelineStageInfo {
                id: PipelineStageId::Tts,
                name: "5. Čínska Syntéza Reči (TTS)".to_string(),
                description: "Generovanie čínskeho audia so zarovnaním dĺžky segmentov (Piper MIT / Kokoro Apache 2.0)".to_string(),
                status: StageStatus::Idle,
                progress_percent: 0.0,
                started_at_ms: None,
                completed_at_ms: None,
                estimated_vram_gb: 0.5,
                estimated_ram_gb: 0.8,
                engine_badge: "Piper (MIT / Kom.)".to_string(),
                is_gpu_accelerated: false,
                error_message: None,
                user_suggestion: None,
            },
            PipelineStageInfo {
                id: PipelineStageId::Lipsync,
                name: "6. Lip-Sync Synchronizácia".to_string(),
                description: "Rozanimovanie pier tváre podľa vygenerovanej čínštiny (LatentSync 1.5 / MuseTalk fallback)".to_string(),
                status: StageStatus::Idle,
                progress_percent: 0.0,
                started_at_ms: None,
                completed_at_ms: None,
                estimated_vram_gb: 7.5,
                estimated_ram_gb: 6.0,
                engine_badge: "LatentSync 1.5 (ROCm SDPA)".to_string(),
                is_gpu_accelerated: true,
                error_message: None,
                user_suggestion: None,
            },
            PipelineStageInfo {
                id: PipelineStageId::Mux,
                name: "7. Záverečný Muxing & Post-processing".to_string(),
                description: "Zmiešanie hudby na pozadí s dabingom, generovanie a vpečenie titulkov (FFmpeg)".to_string(),
                status: StageStatus::Idle,
                progress_percent: 0.0,
                started_at_ms: None,
                completed_at_ms: None,
                estimated_vram_gb: 0.0,
                estimated_ram_gb: 1.0,
                engine_badge: "FFmpeg Mux".to_string(),
                is_gpu_accelerated: false,
                error_message: None,
                user_suggestion: None,
            },
        ]
    }
}
