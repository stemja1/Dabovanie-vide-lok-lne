use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};

use crate::config::app_config::{AppConfig, LipsyncEngine};
use crate::pipeline::stages::{PipelineStageId, PipelineStageInfo, StageFactory, StageStatus};
use crate::wsl::executor::{ProcessErrorKind, ProcessLogLine, WslExecutor};
use crate::wsl::path_mapper::PathMapper;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineExecutionState {
    pub is_running: bool,
    pub is_paused_for_review: bool,
    pub current_stage_index: usize,
    pub stages: Vec<PipelineStageInfo>,
    pub input_video_path_win: String,
    pub input_video_path_wsl: String,
    pub output_video_path_win: Option<String>,
    pub output_video_path_wsl: Option<String>,
    pub metadata_json_path_win: Option<String>,
    pub metadata_json_path_wsl: Option<String>,
    pub error_summary: Option<String>,
    pub active_lipsync_engine: String,
}

pub struct PipelineOrchestrator {
    pub state: Arc<Mutex<PipelineExecutionState>>,
    pub is_cancelled: Arc<AtomicBool>,
}

impl Default for PipelineOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineOrchestrator {
    pub fn new() -> Self {
        let stages = StageFactory::build_default_stages();
        Self {
            state: Arc::new(Mutex::new(PipelineExecutionState {
                is_running: false,
                is_paused_for_review: false,
                current_stage_index: 0,
                stages,
                input_video_path_win: String::new(),
                input_video_path_wsl: String::new(),
                output_video_path_win: None,
                output_video_path_wsl: None,
                metadata_json_path_win: None,
                metadata_json_path_wsl: None,
                error_summary: None,
                active_lipsync_engine: "LatentSync 1.5".to_string(),
            })),
            is_cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
    }

    pub fn reset_cancel(&self) {
        self.is_cancelled.store(false, Ordering::SeqCst);
    }

    /// Ensures all Python pipeline scripts are present in the WSL workspace directory
    pub async fn ensure_scripts_synced(distro: &str, workspace_dir: &str) {
        let ws = workspace_dir.trim_end_matches('/');
        let cmd = format!(
            r#"
mkdir -p "{0}/scripts"
if [ ! -f "{0}/scripts/stage_1_demux.py" ]; then
    for cand in /mnt/c/Dabovanie-vide-lok-lne-main/scripts /mnt/c/*/Dabovanie-vide-lok-lne*/scripts /mnt/c/*/*/scripts; do
        if [ -d "$cand" ] && [ -f "$cand/stage_1_demux.py" ]; then
            cp -ru "$cand"/*.py "{0}/scripts/" 2>/dev/null || true
            break
        fi
    done
fi
"#,
            ws
        );
        let _ = WslExecutor::run_command_output(distro, &cmd).await;
    }

    /// Prepares pipeline for a given input video
    pub async fn set_input_video(&self, win_video_path: &str, distro: &str) {
        let input_wsl = PathMapper::win_to_wsl(win_video_path);
        let input_path = std::path::Path::new(&input_wsl);
        let parent = input_path
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("/home/ubuntu/ai_dubbing_workspace")
            .to_string();
        let stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("video")
            .to_string();

        let out_wsl = format!("{}/{}_dubbed_zh.mp4", parent, stem);
        let out_win = PathMapper::wsl_to_win(&out_wsl, distro);

        let meta_wsl = format!("{}/{}_utterance_metadata.json", parent, stem);
        let meta_win = PathMapper::wsl_to_win(&meta_wsl, distro);

        let mut st = self.state.lock().await;
        st.input_video_path_win = win_video_path.to_string();
        st.input_video_path_wsl = input_wsl;
        st.output_video_path_wsl = Some(out_wsl);
        st.output_video_path_win = Some(out_win);
        st.metadata_json_path_wsl = Some(meta_wsl);
        st.metadata_json_path_win = Some(meta_win);

        st.stages = StageFactory::build_default_stages();
        st.error_summary = None;
        st.is_running = false;
        st.is_paused_for_review = false;
        st.current_stage_index = 0;
    }

    /// Executes the pipeline sequentially
    pub async fn start_pipeline(
        &self,
        config: AppConfig,
        log_tx: Option<mpsc::UnboundedSender<ProcessLogLine>>,
    ) -> Result<()> {
        self.reset_cancel();

        // Ensure scripts are synced to workspace directory in WSL
        Self::ensure_scripts_synced(&config.wsl_distro, &config.workspace_dir).await;

        {
            let mut st = self.state.lock().await;
            st.is_running = true;
            st.is_paused_for_review = false;
            st.error_summary = None;
            st.active_lipsync_engine = match config.lipsync_engine {
                LipsyncEngine::LatentSync15 => "LatentSync 1.5".to_string(),
                LipsyncEngine::MuseTalk => "MuseTalk".to_string(),
            };
        }

        let stages_count = {
            let st = self.state.lock().await;
            st.stages.len()
        };

        for idx in 0..stages_count {
            if self.is_cancelled.load(Ordering::SeqCst) {
                let mut st = self.state.lock().await;
                st.is_running = false;
                if let Some(s) = st.stages.get_mut(idx) {
                    s.status = StageStatus::Skipped;
                }
                return Ok(());
            }

            let stage_id = {
                let st = self.state.lock().await;
                st.stages[idx].id
            };

            // If it's Review stage and auto_pause is enabled
            if stage_id == PipelineStageId::Review {
                let should_pause = config.auto_pause_for_review;
                if should_pause {
                    {
                        let mut st = self.state.lock().await;
                        st.current_stage_index = idx;
                        st.is_paused_for_review = true;
                        if let Some(s) = st.stages.get_mut(idx) {
                            s.status = StageStatus::ReviewPaused;
                            s.progress_percent = 50.0;
                        }
                    }

                    if let Some(ref tx) = log_tx {
                        let _ = tx.send(ProcessLogLine {
                            stream: "system".to_string(),
                            message: "=== PIPELINE POZASTAVENÝ: Prebieha kontrola vygenerovaných metadát a prekladu používateľom. ===".to_string(),
                            timestamp_ms: chrono::Utc::now().timestamp_millis(),
                            is_progress: false,
                            progress_percent: None,
                            step_tag: Some("review".to_string()),
                        });
                    }

                    // Orchestrator halts here until user reviews and calls `continue_after_review`
                    return Ok(());
                }

                let mut st = self.state.lock().await;
                if let Some(s) = st.stages.get_mut(idx) {
                    s.status = StageStatus::Completed;
                    s.progress_percent = 100.0;
                }
                continue;
            }

            // Run normal stage
            let res = self.run_single_stage(idx, &config, log_tx.clone()).await;
            if let Err(e) = res {
                let mut st = self.state.lock().await;
                st.is_running = false;
                st.error_summary = Some(format!("Fáza {} zlyhala: {}", idx + 1, e));
                return Err(e);
            }
        }

        {
            let mut st = self.state.lock().await;
            st.is_running = false;
        }

        if let Some(ref tx) = log_tx {
            let _ = tx.send(ProcessLogLine {
                stream: "system".to_string(),
                message: "=== AI DABINGOVÝ PIPELINE ÚSPEŠNE DOKONČENÝ! ===".to_string(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                is_progress: false,
                progress_percent: Some(100.0),
                step_tag: Some("complete".to_string()),
            });
        }

        Ok(())
    }

    /// Resumes the pipeline after user has confirmed the utterance metadata
    pub async fn continue_after_review(
        &self,
        config: AppConfig,
        log_tx: Option<mpsc::UnboundedSender<ProcessLogLine>>,
    ) -> Result<()> {
        {
            let mut st = self.state.lock().await;
            st.is_paused_for_review = false;
            st.is_running = true;
            for s in &mut st.stages {
                if s.id == PipelineStageId::Review {
                    s.status = StageStatus::Completed;
                    s.progress_percent = 100.0;
                    s.completed_at_ms = Some(chrono::Utc::now().timestamp_millis());
                }
            }
        }

        if let Some(ref tx) = log_tx {
            let _ = tx.send(ProcessLogLine {
                stream: "system".to_string(),
                message: "Pokračujem v pipeline: Generovanie TTS reči a Lip-sync...".to_string(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                is_progress: false,
                progress_percent: None,
                step_tag: Some("resume".to_string()),
            });
        }

        let stages_count = {
            let st = self.state.lock().await;
            st.stages.len()
        };

        // Resume from Stage 4 (TTS) to the end
        for idx in 4..stages_count {
            if self.is_cancelled.load(Ordering::SeqCst) {
                let mut st = self.state.lock().await;
                st.is_running = false;
                return Ok(());
            }

            let res = self.run_single_stage(idx, &config, log_tx.clone()).await;
            if let Err(e) = res {
                let mut st = self.state.lock().await;
                st.is_running = false;
                st.error_summary = Some(format!("Fáza {} zlyhala: {}", idx + 1, e));
                return Err(e);
            }
        }

        {
            let mut st = self.state.lock().await;
            st.is_running = false;
        }

        if let Some(ref tx) = log_tx {
            let _ = tx.send(ProcessLogLine {
                stream: "system".to_string(),
                message: "=== VŠETKY FÁZY BOLI ÚSPEŠNE DOKONČENÉ ===".to_string(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                is_progress: false,
                progress_percent: Some(100.0),
                step_tag: Some("complete".to_string()),
            });
        }

        Ok(())
    }

    /// Runs one single stage and updates state & handles OOM fallback
    pub async fn run_single_stage(
        &self,
        stage_index: usize,
        config: &AppConfig,
        log_tx: Option<mpsc::UnboundedSender<ProcessLogLine>>,
    ) -> Result<()> {
        let (stage_id, input_wsl, output_wsl, meta_wsl) = {
            let mut st = self.state.lock().await;
            if stage_index >= st.stages.len() {
                return Err(anyhow::anyhow!(
                    "Index fázy {} je mimo rozsahu (0..{})",
                    stage_index,
                    st.stages.len()
                ));
            }
            st.current_stage_index = stage_index;
            let s = &mut st.stages[stage_index];
            s.status = StageStatus::Running;
            s.started_at_ms = Some(chrono::Utc::now().timestamp_millis());
            s.progress_percent = 5.0;
            s.error_message = None;
            (
                s.id,
                st.input_video_path_wsl.clone(),
                st.output_video_path_wsl.clone().unwrap_or_default(),
                st.metadata_json_path_wsl.clone().unwrap_or_default(),
            )
        };

        // If simulate mode is on, run mock simulation
        if config.simulate_mode {
            self.run_mock_stage(stage_id, stage_index, log_tx).await?;
            return Ok(());
        }

        // Build CLI command with strict POSIX shell escaping for security
        let cmd = self.build_stage_command(stage_id, &input_wsl, &output_wsl, &meta_wsl, config);

        let res = WslExecutor::run_streaming_command(
            &config.wsl_distro,
            &cmd,
            log_tx.clone(),
            Some(Duration::from_secs(3600)),
            Some(self.is_cancelled.clone()),
        )
        .await?;

        // If cancelled by user
        if res.error_kind == Some(ProcessErrorKind::Cancelled) {
            let mut st = self.state.lock().await;
            if let Some(s) = st.stages.get_mut(stage_index) {
                s.status = StageStatus::Skipped;
            }
            return Ok(());
        }

        // Handle failure and potential LatentSync OOM fallback to MuseTalk
        if !res.success {
            if stage_id == PipelineStageId::Lipsync
                && config.lipsync_fallback_on_oom
                && res.error_kind == Some(ProcessErrorKind::OutOfMemoryGpu)
            {
                if let Some(ref tx) = log_tx {
                    let _ = tx.send(ProcessLogLine {
                        stream: "system".to_string(),
                        message: "⚠️ LatentSync zlyhal na OOM. Automaticky aktivujem záchranný fallback: MuseTalk Engine (~4.5 GB VRAM)...".to_string(),
                        timestamp_ms: chrono::Utc::now().timestamp_millis(),
                        is_progress: false,
                        progress_percent: None,
                        step_tag: Some("fallback".to_string()),
                    });
                }

                // Retry with MuseTalk
                let mut fallback_config = config.clone();
                fallback_config.lipsync_engine = LipsyncEngine::MuseTalk;
                let fallback_cmd = self.build_stage_command(
                    stage_id,
                    &input_wsl,
                    &output_wsl,
                    &meta_wsl,
                    &fallback_config,
                );

                let retry_res = WslExecutor::run_streaming_command(
                    &config.wsl_distro,
                    &fallback_cmd,
                    log_tx.clone(),
                    Some(Duration::from_secs(3600)),
                    Some(self.is_cancelled.clone()),
                )
                .await?;

                if retry_res.success {
                    let mut st = self.state.lock().await;
                    st.active_lipsync_engine = "MuseTalk (OOM Fallback)".to_string();
                    if let Some(s) = st.stages.get_mut(stage_index) {
                        s.status = StageStatus::Completed;
                        s.progress_percent = 100.0;
                        s.completed_at_ms = Some(chrono::Utc::now().timestamp_millis());
                        s.user_suggestion =
                            Some("Úspešne dokončené cez záchranný MuseTalk engine.".to_string());
                    }
                    return Ok(());
                }
            }

            // Otherwise mark as failed
            let mut st = self.state.lock().await;
            let stage_name = if let Some(s) = st.stages.get_mut(stage_index) {
                s.status = StageStatus::Failed;
                s.error_message = Some(res.stderr.clone());
                s.user_suggestion = res.user_remedy;
                s.name.clone()
            } else {
                format!("Fáza {}", stage_index + 1)
            };
            return Err(anyhow::anyhow!(
                "Fáza {} zlyhala: {}",
                stage_name,
                res.stderr
            ));
        }

        // Success
        {
            let mut st = self.state.lock().await;
            if let Some(s) = st.stages.get_mut(stage_index) {
                s.status = StageStatus::Completed;
                s.progress_percent = 100.0;
                s.completed_at_ms = Some(chrono::Utc::now().timestamp_millis());
            }
        }

        Ok(())
    }

    /// Builds the specific bash command using strict shell escaping for security
    fn build_stage_command(
        &self,
        stage_id: PipelineStageId,
        input_wsl: &str,
        output_wsl: &str,
        meta_wsl: &str,
        config: &AppConfig,
    ) -> String {
        let python_bin = PathMapper::escape_bash_arg(&format!(
            "{}/bin/python",
            config.venv_path.trim_end_matches('/')
        ));
        let script_dir = format!("{}/scripts", config.workspace_dir.trim_end_matches('/'));

        let q_in = PathMapper::escape_bash_arg(input_wsl);
        let q_out = PathMapper::escape_bash_arg(output_wsl);
        let q_meta = PathMapper::escape_bash_arg(meta_wsl);
        let q_ws = PathMapper::escape_bash_arg(&config.workspace_dir);

        match stage_id {
            PipelineStageId::Demux => format!(
                "{} {}/stage_1_demux.py --input {} --workspace {}",
                python_bin, script_dir, q_in, q_ws
            ),
            PipelineStageId::Asr => format!(
                "{} {}/stage_2_asr.py --input {} --workspace {} --engine {} --device {} --model {}",
                python_bin, script_dir, q_in, q_ws,
                match config.asr_engine {
                    crate::config::app_config::AsrEngine::WhisperSk => "whisper_sk",
                    crate::config::app_config::AsrEngine::FasterWhisper => "faster_whisper",
                },
                match config.asr_device {
                    crate::config::app_config::AsrDevice::GpuRocm => "rocm",
                    crate::config::app_config::AsrDevice::Cpu => "cpu",
                },
                PathMapper::escape_bash_arg(&config.whisper_sk_model_id)
            ),
            PipelineStageId::Translate => format!(
                "{} {}/stage_3_translate.py --workspace {} --meta {} --model {} --src {} --tgt {}",
                python_bin, script_dir, q_ws, q_meta,
                PathMapper::escape_bash_arg(&config.mt_model_id),
                PathMapper::escape_bash_arg(&config.source_lang),
                PathMapper::escape_bash_arg(&config.target_lang)
            ),
            PipelineStageId::Review => "echo 'Review stage completed'".to_string(),
            PipelineStageId::Tts => format!(
                "{} {}/stage_4_tts.py --workspace {} --meta {} --engine {} --voice {} --speed {:.2}",
                python_bin, script_dir, q_ws, q_meta,
                match config.tts_engine {
                    crate::config::app_config::TtsEngine::Piper => "piper",
                    crate::config::app_config::TtsEngine::Kokoro => "kokoro",
                    crate::config::app_config::TtsEngine::CoquiXtts => "coqui",
                },
                PathMapper::escape_bash_arg(&config.tts_voice),
                config.tts_speed_factor
            ),
            PipelineStageId::Lipsync => format!(
                "{} {}/stage_5_lipsync.py --input {} --workspace {} --meta {} --engine {} --batch-size {} --rocm-sdpa-fallback {}",
                python_bin, script_dir, q_in, q_ws, q_meta,
                match config.lipsync_engine {
                    LipsyncEngine::LatentSync15 => "latentsync",
                    LipsyncEngine::MuseTalk => "musetalk",
                },
                config.lipsync_batch_size,
                if config.rocm_sdpa_fallback { "1" } else { "0" }
            ),
            PipelineStageId::Mux => format!(
                "{} {}/stage_6_mux.py --input {} --output {} --workspace {} --meta {} --ducking {:.1}",
                python_bin, script_dir, q_in, q_out, q_ws, q_meta, config.ducking_level_db
            ),
        }
    }

    /// Simulation runner for tests or UI development
    async fn run_mock_stage(
        &self,
        stage_id: PipelineStageId,
        stage_index: usize,
        log_tx: Option<mpsc::UnboundedSender<ProcessLogLine>>,
    ) -> Result<()> {
        let stage_name = match stage_id {
            PipelineStageId::Demux => "Demuxing",
            PipelineStageId::Asr => "ASR Prepis",
            PipelineStageId::Translate => "Preklad NLLB-200",
            PipelineStageId::Review => "Kontrola",
            PipelineStageId::Tts => "Syntéza Reči",
            PipelineStageId::Lipsync => "Lip-sync Animácia",
            PipelineStageId::Mux => "Muxing & Titulky",
        };

        for step in 1..=5 {
            if self.is_cancelled.load(Ordering::SeqCst) {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
            let pct = (step as f32) * 20.0;

            {
                let mut st = self.state.lock().await;
                if let Some(s) = st.stages.get_mut(stage_index) {
                    s.progress_percent = pct;
                }
            }

            if let Some(ref tx) = log_tx {
                let _ = tx.send(ProcessLogLine {
                    stream: "stdout".to_string(),
                    message: format!(
                        "[SIMULÁCIA] {}: Spracúvam krok {}/5 ({} %)...",
                        stage_name, step, pct
                    ),
                    timestamp_ms: chrono::Utc::now().timestamp_millis(),
                    is_progress: true,
                    progress_percent: Some(pct),
                    step_tag: Some(format!("{:?}", stage_id)),
                });
            }
        }

        {
            let mut st = self.state.lock().await;
            if let Some(s) = st.stages.get_mut(stage_index) {
                s.status = StageStatus::Completed;
                s.progress_percent = 100.0;
                s.completed_at_ms = Some(chrono::Utc::now().timestamp_millis());
            }
        }

        Ok(())
    }
}
