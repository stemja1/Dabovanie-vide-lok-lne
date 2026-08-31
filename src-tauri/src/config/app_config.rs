use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub wsl_distro: String,
    pub venv_path: String,
    pub workspace_dir: String,
    pub models_cache_dir: String,

    // ASR settings
    pub asr_engine: AsrEngine,
    pub asr_device: AsrDevice,
    pub whisper_sk_model_id: String,

    // MT settings
    pub mt_model_id: String,
    pub source_lang: String,
    pub target_lang: String,

    // TTS settings
    pub tts_engine: TtsEngine,
    pub tts_voice: String,
    pub tts_speed_factor: f32,

    // Lip-sync settings
    pub lipsync_engine: LipsyncEngine,
    pub lipsync_batch_size: u32,
    pub lipsync_fallback_on_oom: bool,
    pub rocm_sdpa_fallback: bool,

    // Audio & Video processing
    pub target_resolution: String,
    pub ducking_level_db: f32,
    pub auto_pause_for_review: bool,

    // Development / Simulation
    pub simulate_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AsrEngine {
    WhisperSk,
    FasterWhisper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AsrDevice {
    GpuRocm,
    Cpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtsEngine {
    Piper,     // MIT - Commercial Safe
    Kokoro,    // Apache 2.0 - Commercial Safe
    CoquiXtts, // CPML - Non-commercial / Evaluation Only
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LipsyncEngine {
    LatentSync15, // High quality, 6.5-8 GB VRAM, UNet SDPA
    MuseTalk,     // Fast, lightweight, 4-5 GB VRAM
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            wsl_distro: "Ubuntu-24.04".to_string(),
            venv_path: "~/.dubbing_env".to_string(),
            workspace_dir: "~/ai_dubbing_workspace".to_string(),
            models_cache_dir: "~/ai_dubbing_workspace/models".to_string(),

            asr_engine: AsrEngine::WhisperSk,
            asr_device: AsrDevice::GpuRocm,
            whisper_sk_model_id: "NaiveNeuron/whisper-large-v3-sk".to_string(),

            mt_model_id: "facebook/nllb-200-distilled-600M".to_string(),
            source_lang: "slk_Latn".to_string(),
            target_lang: "zho_Hans".to_string(),

            tts_engine: TtsEngine::Piper,
            tts_voice: "zh_CN-huayan-medium".to_string(),
            tts_speed_factor: 1.0,

            lipsync_engine: LipsyncEngine::LatentSync15,
            lipsync_batch_size: 8,
            lipsync_fallback_on_oom: true,
            rocm_sdpa_fallback: true,

            target_resolution: "original".to_string(),
            ducking_level_db: -14.0,
            auto_pause_for_review: true,

            simulate_mode: false,
        }
    }
}

impl AppConfig {
    pub fn get_default_config_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("sk_zh_ai_dubbing").join("config.toml")
        } else {
            PathBuf::from("config.toml")
        }
    }

    pub fn load_or_default() -> Self {
        let path = Self::get_default_config_path();
        if path.exists() {
            match Self::load_from_file(&path) {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(
                        "Failed to load config from {:?}, using default: {}",
                        path,
                        e
                    );
                    Self::default()
                }
            }
        } else {
            let default_cfg = Self::default();
            let _ = default_cfg.save_to_file(&path);
            default_cfg
        }
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file at {:?}", path.as_ref()))?;
        let config: AppConfig =
            toml::from_str(&content).with_context(|| "Failed to parse TOML config")?;
        Ok(config)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {:?}", parent))?;
        }
        let content =
            toml::to_string_pretty(self).with_context(|| "Failed to serialize config to TOML")?;
        fs::write(path_ref, content)
            .with_context(|| format!("Failed to write config file to {:?}", path_ref))?;
        Ok(())
    }

    pub fn to_toml_string(&self) -> Result<String> {
        toml::to_string_pretty(self).context("Failed to serialize config to TOML string")
    }

    pub fn from_toml_string(toml_str: &str) -> Result<Self> {
        toml::from_str(toml_str).context("Failed to parse config from TOML string")
    }
}
