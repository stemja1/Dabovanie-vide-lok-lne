use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordItem {
    pub word: String,
    pub start: f64,
    pub end: f64,
    pub score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtteranceItem {
    pub id: String,
    pub start_time: f64,
    pub end_time: f64,
    pub duration: f64,
    pub speaker_id: String,
    pub slovak_text: String,
    pub chinese_text: String,
    pub target_audio_file: Option<String>,
    pub speed_factor: f32,
    pub is_edited: bool,
    pub confidence: Option<f32>,
    pub words: Vec<WordItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtteranceMetadataDocument {
    pub video_source: String,
    pub total_duration: f64,
    pub sample_rate: u32,
    pub source_language: String,
    pub target_language: String,
    pub utterances: Vec<UtteranceItem>,
    pub generated_at_iso: String,
    pub is_verified_by_user: bool,
}

impl UtteranceMetadataDocument {
    pub fn new_empty(video_source: &str) -> Self {
        Self {
            video_source: video_source.to_string(),
            total_duration: 0.0,
            sample_rate: 24000,
            source_language: "slk_Latn".to_string(),
            target_language: "zho_Hans".to_string(),
            utterances: Vec::new(),
            generated_at_iso: chrono::Utc::now().to_rfc3339(),
            is_verified_by_user: false,
        }
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Nepodarilo sa načítať utterance_metadata z {:?}", path.as_ref()))?;
        let doc: UtteranceMetadataDocument = serde_json::from_str(&content)
            .with_context(|| "Chyba pri parsovaní utterance_metadata.json")?;
        Ok(doc)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .with_context(|| "Nepodarilo sa serializovať utterance_metadata do JSON")?;
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path.as_ref(), content)
            .with_context(|| format!("Nepodarilo sa zapísať utterance_metadata do {:?}", path.as_ref()))?;
        Ok(())
    }

    /// Generates sample mock metadata for testing & demonstration
    pub fn create_demo_data(video_source: &str) -> Self {
        Self {
            video_source: video_source.to_string(),
            total_duration: 28.5,
            sample_rate: 24000,
            source_language: "slk_Latn".to_string(),
            target_language: "zho_Hans".to_string(),
            generated_at_iso: chrono::Utc::now().to_rfc3339(),
            is_verified_by_user: false,
            utterances: vec![
                UtteranceItem {
                    id: "utt_001".to_string(),
                    start_time: 0.5,
                    end_time: 3.8,
                    duration: 3.3,
                    speaker_id: "SPEAKER_00".to_string(),
                    slovak_text: "Dobrý deň, vítam vás pri prezentácii nášho nového produktu.".to_string(),
                    chinese_text: "您好，欢迎来到我们新产品的展示会。".to_string(),
                    target_audio_file: Some("audio_segments/utt_001.wav".to_string()),
                    speed_factor: 1.0,
                    is_edited: false,
                    confidence: Some(0.98),
                    words: vec![
                        WordItem { word: "Dobrý".to_string(), start: 0.5, end: 0.8, score: Some(0.99) },
                        WordItem { word: "deň,".to_string(), start: 0.8, end: 1.1, score: Some(0.98) },
                        WordItem { word: "vítam".to_string(), start: 1.2, end: 1.5, score: Some(0.97) },
                        WordItem { word: "vás".to_string(), start: 1.5, end: 1.7, score: Some(0.99) },
                        WordItem { word: "pri".to_string(), start: 1.8, end: 2.0, score: Some(0.96) },
                        WordItem { word: "prezentácii".to_string(), start: 2.0, end: 2.7, score: Some(0.98) },
                        WordItem { word: "nového".to_string(), start: 2.8, end: 3.2, score: Some(0.99) },
                        WordItem { word: "produktu.".to_string(), start: 3.2, end: 3.8, score: Some(0.98) },
                    ],
                },
                UtteranceItem {
                    id: "utt_002".to_string(),
                    start_time: 4.2,
                    end_time: 9.0,
                    duration: 4.8,
                    speaker_id: "SPEAKER_00".to_string(),
                    slovak_text: "Tento systém využíva pokročilú umelú inteligenciu a beží kompletne lokálne na vašom hardvéri.".to_string(),
                    chinese_text: "该系统利用先进的人工智能，并完全在您的本地硬件上运行。".to_string(),
                    target_audio_file: Some("audio_segments/utt_002.wav".to_string()),
                    speed_factor: 1.05,
                    is_edited: false,
                    confidence: Some(0.96),
                    words: vec![
                        WordItem { word: "Tento".to_string(), start: 4.2, end: 4.5, score: Some(0.99) },
                        WordItem { word: "systém".to_string(), start: 4.6, end: 5.1, score: Some(0.98) },
                        WordItem { word: "využíva".to_string(), start: 5.2, end: 5.8, score: Some(0.97) },
                        WordItem { word: "umelú".to_string(), start: 6.0, end: 6.5, score: Some(0.96) },
                        WordItem { word: "inteligenciu".to_string(), start: 6.5, end: 7.3, score: Some(0.98) },
                        WordItem { word: "lokálne.".to_string(), start: 7.5, end: 9.0, score: Some(0.95) },
                    ],
                },
                UtteranceItem {
                    id: "utt_003".to_string(),
                    start_time: 9.6,
                    end_time: 14.8,
                    duration: 5.2,
                    speaker_id: "SPEAKER_00".to_string(),
                    slovak_text: "Vďaka optimalizácii pre grafické karty AMD Radeon dosahuje vysoký výkon bez odosielania dát na cloud.".to_string(),
                    chinese_text: "由于针对AMD Radeon显卡进行了优化，无需将数据发送到云端即可实现高性能。".to_string(),
                    target_audio_file: Some("audio_segments/utt_003.wav".to_string()),
                    speed_factor: 0.98,
                    is_edited: false,
                    confidence: Some(0.97),
                    words: vec![],
                },
            ],
        }
    }
}
