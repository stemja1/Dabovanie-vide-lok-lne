use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WordItem {
    pub word: String,
    pub start: f64,
    pub end: f64,
    pub score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
        let content = fs::read_to_string(path.as_ref()).with_context(|| {
            format!(
                "Nepodarilo sa načítať utterance_metadata z {:?}",
                path.as_ref()
            )
        })?;
        let mut doc: UtteranceMetadataDocument = serde_json::from_str(&content)
            .with_context(|| "Chyba pri parsovaní utterance_metadata.json")?;
        doc.recalculate_timings();
        Ok(doc)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.validate_integrity()?;
        let content = serde_json::to_string_pretty(self)
            .with_context(|| "Nepodarilo sa serializovať utterance_metadata do JSON")?;
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path.as_ref(), content).with_context(|| {
            format!(
                "Nepodarilo sa zapísať utterance_metadata do {:?}",
                path.as_ref()
            )
        })?;
        Ok(())
    }

    /// Recalculates segment durations, total document duration, and sorts by start time
    pub fn recalculate_timings(&mut self) {
        self.utterances.sort_by(|a, b| {
            a.start_time
                .partial_cmp(&b.start_time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut max_end: f64 = 0.0;
        for utt in &mut self.utterances {
            utt.duration = (utt.end_time - utt.start_time).max(0.05);
            if utt.end_time > max_end {
                max_end = utt.end_time;
            }
        }
        self.total_duration = (max_end * 100.0).round() / 100.0;
    }

    /// Validates internal consistency of the metadata
    pub fn validate_integrity(&self) -> Result<()> {
        for (idx, utt) in self.utterances.iter().enumerate() {
            if utt.id.trim().is_empty() {
                bail!("Utterance #{idx} má prázdny identifikátor");
            }
            if utt.start_time < 0.0 {
                bail!(
                    "Utterance {} má záporný začiatočný čas: {}",
                    utt.id,
                    utt.start_time
                );
            }
            if utt.end_time <= utt.start_time {
                bail!(
                    "Utterance {} má neplatné časovanie: start={}s, end={}s",
                    utt.id,
                    utt.start_time,
                    utt.end_time
                );
            }
            if utt.speed_factor <= 0.0 || utt.speed_factor > 4.0 {
                bail!(
                    "Utterance {} má neplatný speed factor: {}",
                    utt.id,
                    utt.speed_factor
                );
            }
        }
        Ok(())
    }

    /// Splits an utterance into two at a specified split timestamp
    pub fn split_utterance(
        &mut self,
        utterance_id: &str,
        split_time: f64,
        sk_part1: String,
        sk_part2: String,
        zh_part1: String,
        zh_part2: String,
    ) -> Result<()> {
        let pos = self
            .utterances
            .iter()
            .position(|u| u.id == utterance_id)
            .ok_or_else(|| anyhow::anyhow!("Utterance ID '{}' nebol nájdený", utterance_id))?;

        let orig = self.utterances[pos].clone();
        if split_time <= orig.start_time || split_time >= orig.end_time {
            bail!(
                "Čas rozdelenia ({:.2}s) musí byť v intervale ({:.2}s, {:.2}s)",
                split_time,
                orig.start_time,
                orig.end_time
            );
        }

        let id1 = format!("{}_a", orig.id);
        let id2 = format!("{}_b", orig.id);

        let mut words1 = Vec::new();
        let mut words2 = Vec::new();

        for w in orig.words {
            if w.end <= split_time {
                words1.push(w);
            } else {
                words2.push(w);
            }
        }

        let utt1 = UtteranceItem {
            id: id1.clone(),
            start_time: orig.start_time,
            end_time: split_time,
            duration: (split_time - orig.start_time).max(0.1),
            speaker_id: orig.speaker_id.clone(),
            slovak_text: sk_part1,
            chinese_text: zh_part1,
            target_audio_file: Some(format!("audio_segments/{}.wav", id1)),
            speed_factor: orig.speed_factor,
            is_edited: true,
            confidence: orig.confidence,
            words: words1,
        };

        let utt2 = UtteranceItem {
            id: id2.clone(),
            start_time: split_time,
            end_time: orig.end_time,
            duration: (orig.end_time - split_time).max(0.1),
            speaker_id: orig.speaker_id,
            slovak_text: sk_part2,
            chinese_text: zh_part2,
            target_audio_file: Some(format!("audio_segments/{}.wav", id2)),
            speed_factor: orig.speed_factor,
            is_edited: true,
            confidence: orig.confidence,
            words: words2,
        };

        self.utterances.remove(pos);
        self.utterances.insert(pos, utt2);
        self.utterances.insert(pos, utt1);
        self.recalculate_timings();

        Ok(())
    }

    /// Merges two adjacent utterances into one
    pub fn merge_utterances(&mut self, id1: &str, id2: &str) -> Result<()> {
        let pos1 = self
            .utterances
            .iter()
            .position(|u| u.id == id1)
            .ok_or_else(|| anyhow::anyhow!("Utterance 1 '{}' nebol nájdený", id1))?;
        let pos2 = self
            .utterances
            .iter()
            .position(|u| u.id == id2)
            .ok_or_else(|| anyhow::anyhow!("Utterance 2 '{}' nebol nájdený", id2))?;

        if (pos1 as isize - pos2 as isize).abs() != 1 {
            bail!(
                "Zlúčiť je možné iba susediace segmenty (nájdené pozície: {}, {})",
                pos1,
                pos2
            );
        }

        let (first_idx, second_idx) = if pos1 < pos2 {
            (pos1, pos2)
        } else {
            (pos2, pos1)
        };
        let u1 = self.utterances[first_idx].clone();
        let u2 = self.utterances[second_idx].clone();

        let mut combined_words = u1.words;
        combined_words.extend(u2.words);

        let merged = UtteranceItem {
            id: format!("{}_merged", u1.id),
            start_time: u1.start_time,
            end_time: u2.end_time,
            duration: u2.end_time - u1.start_time,
            speaker_id: u1.speaker_id,
            slovak_text: format!("{} {}", u1.slovak_text.trim(), u2.slovak_text.trim()),
            chinese_text: format!("{}{}", u1.chinese_text.trim(), u2.chinese_text.trim()),
            target_audio_file: Some(format!("audio_segments/{}_merged.wav", u1.id)),
            speed_factor: ((u1.speed_factor + u2.speed_factor) / 2.0 * 100.0).round() / 100.0,
            is_edited: true,
            confidence: match (u1.confidence, u2.confidence) {
                (Some(c1), Some(c2)) => Some((c1 + c2) / 2.0),
                (Some(c1), None) => Some(c1),
                (None, Some(c2)) => Some(c2),
                _ => None,
            },
            words: combined_words,
        };

        self.utterances.remove(second_idx);
        self.utterances.remove(first_idx);
        self.utterances.insert(first_idx, merged);
        self.recalculate_timings();

        Ok(())
    }

    /// Generates sample mock metadata for testing & demonstration
    pub fn create_demo_data(video_source: &str) -> Self {
        let mut doc = Self {
            video_source: video_source.to_string(),
            total_duration: 14.8,
            sample_rate: 24000,
            source_language: "slk_Latn".to_string(),
            target_language: "zho_Hans".to_string(),
            generated_at_iso: "2026-08-30T14:30:00Z".to_string(),
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
        };
        doc.recalculate_timings();
        doc
    }
}
