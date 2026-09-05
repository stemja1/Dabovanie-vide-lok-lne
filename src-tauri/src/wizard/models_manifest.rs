use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManifestItem {
    pub id: String,
    pub name: String,
    pub category: ModelCategory,
    pub description: String,
    pub license: String,
    pub is_commercial_safe: bool,
    pub approximate_size_mb: u64,
    pub local_relative_path: String,
    pub download_urls: Vec<String>,
    pub expected_sha256: Option<String>,
    pub is_required_for_mvp: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCategory {
    Asr,
    Translation,
    Tts,
    Lipsync,
    Auxiliary,
}

pub struct ModelsManifest;

impl ModelsManifest {
    pub fn get_all_models() -> Vec<ModelManifestItem> {
        vec![
            ModelManifestItem {
                id: "whisper-large-v3-sk".to_string(),
                name: "Whisper Large-v3 SK Fine-tune".to_string(),
                category: ModelCategory::Asr,
                description: "Slovenský ASR model od NaiveNeuron s presným časovaním slov a interpunkciou.".to_string(),
                license: "Apache 2.0".to_string(),
                is_commercial_safe: true,
                approximate_size_mb: 3100,
                local_relative_path: "models/asr/whisper-large-v3-sk".to_string(),
                download_urls: vec![
                    "https://huggingface.co/NaiveNeuron/whisper-large-v3-sk".to_string(),
                ],
                expected_sha256: None,
                is_required_for_mvp: true,
            },
            ModelManifestItem {
                id: "nllb-200-distilled-600m".to_string(),
                name: "NLLB-200 Distilled 600M".to_string(),
                category: ModelCategory::Translation,
                description: "Vysokorýchlostný neurónový prekladač slk_Latn -> zho_Hans s nízkou spotrebou VRAM (~2 GB).".to_string(),
                license: "CC-BY-NC-4.0 / Research".to_string(),
                is_commercial_safe: false,
                approximate_size_mb: 1200,
                local_relative_path: "models/mt/nllb-200-distilled-600M".to_string(),
                download_urls: vec![
                    "https://huggingface.co/facebook/nllb-200-distilled-600M".to_string(),
                ],
                expected_sha256: None,
                is_required_for_mvp: true,
            },
            ModelManifestItem {
                id: "piper-zh-huayan".to_string(),
                name: "Piper TTS — Chinese (Huayan Medium)".to_string(),
                category: ModelCategory::Tts,
                description: "Ultra-rýchly syntetizátor čínskej reči pre CPU aj ROCm. Vhodný pre komerčné nasadenie.".to_string(),
                license: "MIT (Komerčne bezpečné)".to_string(),
                is_commercial_safe: true,
                approximate_size_mb: 65,
                local_relative_path: "models/tts/piper/zh_CN-huayan-medium.onnx".to_string(),
                download_urls: vec![
                    "https://huggingface.co/rhasspy/piper-voices/resolve/main/zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx".to_string(),
                ],
                expected_sha256: None,
                is_required_for_mvp: true,
            },
            ModelManifestItem {
                id: "kokoro-v019".to_string(),
                name: "Kokoro TTS (v0.19 Multilingual)".to_string(),
                category: ModelCategory::Tts,
                description: "Moderný 82M neurónový TTS model s vysokou kvalitou intonácie. Komerčná Apache 2.0 licencia.".to_string(),
                license: "Apache 2.0 (Komerčne bezpečné)".to_string(),
                is_commercial_safe: true,
                approximate_size_mb: 340,
                local_relative_path: "models/tts/kokoro/kokoro-v0_19.onnx".to_string(),
                download_urls: vec![
                    "https://huggingface.co/hexgrad/Kokoro-82M/resolve/main/kokoro-v0_19.onnx".to_string(),
                ],
                expected_sha256: None,
                is_required_for_mvp: false,
            },
            ModelManifestItem {
                id: "coqui-xtts-v2".to_string(),
                name: "Coqui XTTS-v2 (Klonovanie hlasu)".to_string(),
                category: ModelCategory::Tts,
                description: "Viacjazyčný TTS model s klonovaním hlasu z referenčného audia. UPOZORNENIE: CPML nekomerčná licencia!".to_string(),
                license: "CPML (Len nekomerčné / testovacie)".to_string(),
                is_commercial_safe: false,
                approximate_size_mb: 3200,
                local_relative_path: "models/tts/coqui-xtts-v2".to_string(),
                download_urls: vec![
                    "https://huggingface.co/coqui/XTTS-v2".to_string(),
                ],
                expected_sha256: None,
                is_required_for_mvp: false,
            },
            ModelManifestItem {
                id: "latentsync-1-5".to_string(),
                name: "LatentSync 1.5 Checkpoint".to_string(),
                category: ModelCategory::Lipsync,
                description: "UNet lip-sync checkpoint pre LatentSync v1.5 (~7 GB VRAM). Optimalizované pre ROCm SDPA fallback.".to_string(),
                license: "Apache 2.0".to_string(),
                is_commercial_safe: true,
                approximate_size_mb: 3800,
                local_relative_path: "models/lipsync/latentsync/latentsync_unet.pt".to_string(),
                download_urls: vec![
                    "https://huggingface.co/ByteDance/LatentSync/resolve/main/latentsync_unet.pt".to_string(),
                ],
                expected_sha256: None,
                is_required_for_mvp: true,
            },
            ModelManifestItem {
                id: "musetalk-weights".to_string(),
                name: "MuseTalk Checkpoints & DWPose".to_string(),
                category: ModelCategory::Lipsync,
                description: "Odľahčený lip-sync model s nízkou spotrebou (~4.5 GB VRAM) — ideálny fallback pri OOM alebo rýchlom režime.".to_string(),
                license: "MIT".to_string(),
                is_commercial_safe: true,
                approximate_size_mb: 3400,
                local_relative_path: "models/lipsync/musetalk/musetalk.json".to_string(),
                download_urls: vec![
                    "https://huggingface.co/TMElyralab/MuseTalk/resolve/main/musetalk/musetalk.json".to_string(),
                    "https://huggingface.co/TMElyralab/MuseTalk/resolve/main/musetalk/pytorch_model.bin".to_string(),
                ],
                expected_sha256: None,
                is_required_for_mvp: true,
            },
        ]
    }
}
