use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::sync::{mpsc, Mutex};

use crate::config::app_config::{AppConfig, LipsyncEngine};
use crate::pipeline::stages::{PipelineStageId, PipelineStageInfo, StageFactory, StageStatus};
use crate::wsl::executor::{ProcessErrorKind, ProcessLogLine, WslExecutor};
use crate::wsl::path_mapper::PathMapper;

/// Per-stage wall-clock budget.
///
/// The previous code applied a flat 60-minute timeout to every stage. That is
/// fine for ffmpeg muxing but cuts off long ASR and lip-sync runs on bigger
/// videos, which scale with the video's duration. Stages that scale with input
/// length get a longer budget; cheap stages stay short so a hang is reported
/// quickly instead of leaving the UI "running" indefinitely.
fn stage_timeout(stage_id: PipelineStageId) -> Duration {
    match stage_id {
        // Whisper large-v3 transcription is the slowest CPU-bound step.
        PipelineStageId::Asr => Duration::from_secs(4 * 3600),
        // Lip-sync inference is the slowest GPU step by a wide margin.
        PipelineStageId::Lipsync => Duration::from_secs(6 * 3600),
        // Translation is per-utterance but a long video means many utterances.
        PipelineStageId::Translate => Duration::from_secs(2 * 3600),
        // TTS shells out per utterance, so it also scales with segment count.
        PipelineStageId::Tts => Duration::from_secs(2 * 3600),
        // ffmpeg demux/mux are I/O bound and quick.
        PipelineStageId::Demux | PipelineStageId::Mux | PipelineStageId::Review => {
            Duration::from_secs(30 * 60)
        }
    }
}

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

    /// Ensures all Python pipeline scripts are present in the WSL workspace directory.
    ///
    /// The scripts are bundled as a Tauri resource (see `tauri.conf.json`
    /// `bundle.resources`), so they ship next to the executable and are found via
    /// `resource_dir()` instead of the previously hardcoded `/mnt/c/...` guesses.
    /// Those guesses only matched a developer's specific folder layout and left
    /// `$WORKSPACE/scripts` empty on every real installation, so stage 1 died with
    /// "can't open file".
    ///
    /// Errors are propagated instead of being swallowed with `let _ =`: a failed
    /// sync is a hard, actionable failure the user must see.
    pub async fn ensure_scripts_synced(
        app: &AppHandle,
        distro: &str,
        workspace_dir: &str,
    ) -> Result<()> {
        let source_dir = app
            .path()
            .resource_dir()
            .map_err(|e| {
                anyhow::anyhow!("Nepodarilo sa určiť resource priečinok aplikácie: {}", e)
            })?
            .join("scripts");

        if !source_dir.join("stage_1_demux.py").is_file() {
            anyhow::bail!(
                "Python skripty sa nepodarilo nájsť v: {}. Reštartuj aplikáciu po aktualizácii.",
                source_dir.display()
            );
        }

        // `resource_dir()` is a Windows path, but this command runs inside bash on
        // the WSL side, where a raw `C:\...` path does not resolve. Convert it to
        // the /mnt/<drive>/... form the distro can actually read, and reject a
        // resource dir that lives somewhere WSL cannot mount (e.g. a network or
        // UNC location) instead of failing later with an opaque `cp` error.
        let src_wsl = PathMapper::win_to_wsl(&source_dir.to_string_lossy());
        if src_wsl.starts_with("//") {
            anyhow::bail!(
                "Priečinok aplikácie sa nachádza na ceste nedostupnej pre WSL: {}. \
                 Nainštalujte aplikáciu na lokálny disk.",
                source_dir.display()
            );
        }

        // `workspace_dir` is user-editable AppConfig data — always escape it before
        // splicing into a shell command (see `PathMapper::escape_bash_arg`).
        let ws = PathMapper::escape_bash_arg(workspace_dir.trim_end_matches('/'));
        let src = PathMapper::escape_bash_arg(&src_wsl);
        let cmd = format!(
            r#"
WORKSPACE={0}
WORKSPACE="${{WORKSPACE/#\~/$HOME}}"
SRC={1}
mkdir -p "$WORKSPACE/scripts"
cp -f "$SRC"/*.py "$WORKSPACE/scripts/"
if [ ! -f "$WORKSPACE/scripts/stage_1_demux.py" ]; then
    echo "SYNC_FAILED: stage_1_demux.py sa nepodarilo skopírovať z $SRC" >&2
    exit 1
fi
echo "SYNC_OK: $WORKSPACE/scripts"

# `$SRC` is a /mnt/<drive>/... path. Report the resolved location so a failure
# points at the real directory rather than an ambiguous filename.
ls -1 "$WORKSPACE/scripts"/*.py >&2 || true
"#,
            ws, src
        );

        let out = WslExecutor::run_command_output(distro, &cmd).await?;
        if !out.status.success() {
            anyhow::bail!(
                "Synchronizácia Python skriptov do WSL zlyhala: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(())
    }

    /// Prepares pipeline for a given input video
    pub async fn set_input_video(&self, win_video_path: &str, distro: &str) {
        // Reject an unusable path up front. Deriving the WSL path from an
        // already-WSL path, or from a file that does not exist, produced commands
        // that only failed much later inside ffmpeg.
        let trimmed = win_video_path.trim();
        // Clear the previous selection up front. Every rejection branch below used
        // to leave the old `input_video_path_win` in place, so the UI still showed
        // "video selected" with the Start button enabled while the new path was
        // rejected.
        {
            let mut st = self.state.lock().await;
            st.input_video_path_win = String::new();
            st.input_video_path_wsl = String::new();
            st.output_video_path_win = None;
            st.output_video_path_wsl = None;
            st.metadata_json_path_win = None;
            st.metadata_json_path_wsl = None;
            st.error_summary = None;
            st.stages = StageFactory::build_default_stages();
            st.current_stage_index = 0;
        }
        if trimmed.is_empty() {
            let mut st = self.state.lock().await;
            st.error_summary = Some("Vstupná cesta k videu je prázdna.".to_string());
            return;
        }
        if trimmed.contains('\0') {
            let mut st = self.state.lock().await;
            st.error_summary = Some("Vstupná cesta k videu obsahuje neplatné znaky.".to_string());
            return;
        }
        if !std::path::Path::new(trimmed).is_file() {
            let mut st = self.state.lock().await;
            st.error_summary = Some(format!("Vstupné video sa nenašlo na disku: '{}'", trimmed));
            return;
        }
        if PathMapper::win_to_wsl(trimmed).starts_with("//") {
            let mut st = self.state.lock().await;
            st.error_summary = Some(
                "Vstupné video leží na sieti/UNC ceste, ktorú WSL nedokáže spracovať. \
                 Skopírujte ho na lokálny disk (napr. C:\\) a skúste znova."
                    .to_string(),
            );
            return;
        }

        let input_wsl = PathMapper::win_to_wsl(trimmed);
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
        st.input_video_path_win = trimmed.to_string();
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
        app: &AppHandle,
        config: AppConfig,
        log_tx: Option<mpsc::UnboundedSender<ProcessLogLine>>,
    ) -> Result<()> {
        {
            let mut st = self.state.lock().await;
            if st.is_running {
                anyhow::bail!(
                    "Pipeline už beží. Počkajte na dokončenie alebo zrušte aktuálny proces."
                );
            }
            // Refuse to start without a usable input video. The run used to begin
            // and fail at stage 1 with a raw ffmpeg "No such file or directory"
            // that never hinted the real problem was the missing selection.
            if st.input_video_path_wsl.trim().is_empty() {
                anyhow::bail!(
                    "Nie je vybraný vstupný video súbor. Použite tlačidlo 'Vybrať video'."
                );
            }
            if let Some(err) = st.error_summary.clone() {
                anyhow::bail!("{}", err);
            }
            st.is_running = true;
            st.is_paused_for_review = false;
            st.error_summary = None;
            st.active_lipsync_engine = match config.lipsync_engine {
                LipsyncEngine::LatentSync15 => "LatentSync 1.5".to_string(),
                LipsyncEngine::MuseTalk => "MuseTalk".to_string(),
            };
        }

        self.reset_cancel();

        // Ensure scripts are synced to workspace directory in WSL. A failure here
        // aborts the run: every later stage would otherwise fail with a confusing
        // "file not found" for the very same missing script.
        if let Err(e) =
            Self::ensure_scripts_synced(app, &config.wsl_distro, &config.workspace_dir).await
        {
            let mut st = self.state.lock().await;
            st.is_running = false;
            let message = format!("Pipeline sa nespustil: {:#}", e);
            st.error_summary = Some(message.clone());
            // Also surface it on the stage the user is looking at. `error_summary`
            // is not rendered anywhere in the UI, so without this the single most
            // common startup failure was completely invisible.
            if let Some(s) = st.stages.first_mut() {
                s.status = StageStatus::Failed;
                s.error_message = Some(message);
                s.user_suggestion = Some(
                    "Spustite znova Setup Wizard alebo kliknite na 'Opakovať krok'.".to_string(),
                );
            }
            if let Some(ref tx) = log_tx {
                let _ = tx.send(ProcessLogLine {
                    stream: "system".to_string(),
                    message: format!("=== PIPELINE NESPUSTENÝ: {:#} ===", e),
                    timestamp_ms: chrono::Utc::now().timestamp_millis(),
                    is_progress: false,
                    progress_percent: None,
                    step_tag: Some("error".to_string()),
                });
            }
            return Err(e);
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
                        st.is_running = false;
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
        let (tts_index, stages_count) = {
            let mut st = self.state.lock().await;
            if st.is_running {
                anyhow::bail!("Pipeline už beží.");
            }
            if !st.is_paused_for_review {
                anyhow::bail!("Pipeline nie je v stave pozastavenia na kontrolu metadát.");
            }
            st.is_paused_for_review = false;
            st.is_running = true;
            for s in &mut st.stages {
                if s.id == PipelineStageId::Review {
                    s.status = StageStatus::Completed;
                    s.progress_percent = 100.0;
                    s.completed_at_ms = Some(chrono::Utc::now().timestamp_millis());
                }
            }
            // Resume from the TTS stage. The old `unwrap_or(4)` silently fell back
            // to a hardcoded index that only happens to be TTS with the current
            // stage list; a changed or reordered list would have resumed in the
            // wrong place. Resolve by id, and bail out loudly if TTS is absent.
            let tts_pos = match st.stages.iter().position(|s| s.id == PipelineStageId::Tts) {
                Some(pos) => pos,
                None => {
                    st.is_running = false;
                    st.is_paused_for_review = true;
                    anyhow::bail!(
                        "Fáza TTS sa v zozname fáz nenachádza; pipeline nemožno pokračovať."
                    );
                }
            };
            (tts_pos, st.stages.len())
        };

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

        // Resume from TTS stage to the end
        for idx in tts_index..stages_count {
            if self.is_cancelled.load(Ordering::SeqCst) {
                let mut st = self.state.lock().await;
                st.is_running = false;
                if let Some(s) = st.stages.get_mut(idx) {
                    s.status = StageStatus::Skipped;
                }
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
                message: "=== AI DABINGOVÝ PIPELINE ÚSPEŠNE DOKONČENÝ! ===".to_string(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                is_progress: false,
                progress_percent: Some(100.0),
                step_tag: Some("complete".to_string()),
            });
        }

        Ok(())
    }

    /// Records a stage failure on the shared state without running the stage.
    async fn mark_stage_failed(
        &self,
        stage_index: usize,
        message: String,
        suggestion: Option<String>,
    ) {
        let mut st = self.state.lock().await;
        st.is_running = false;
        st.error_summary = Some(message.clone());
        if let Some(s) = st.stages.get_mut(stage_index) {
            s.status = StageStatus::Failed;
            s.error_message = Some(message);
            s.user_suggestion = suggestion;
        }
    }

    /// Runs a single pipeline stage
    pub async fn run_single_stage(
        &self,
        stage_index: usize,
        config: &AppConfig,
        log_tx: Option<mpsc::UnboundedSender<ProcessLogLine>>,
    ) -> Result<()> {
        let (stage_id, _stage_name) = {
            let mut st = self.state.lock().await;
            // Two pipelines running at once write the same `audio_segments/*.wav`,
            // `{stem}_utterance_metadata.json` and `lipsync_output.mp4`, and both
            // overwrite `current_stage_index`. The result is corrupted output
            // rather than an error. `start_pipeline` and `continue_after_review`
            // already had this guard; the standalone path did not.
            if st.is_running {
                anyhow::bail!(
                    "Pipeline už beží. Počkajte na dokončenie alebo zrušte aktuálny proces."
                );
            }
            st.current_stage_index = stage_index;
            st.is_running = true;
            if let Some(s) = st.stages.get_mut(stage_index) {
                s.status = StageStatus::Running;
                s.progress_percent = 0.0;
                s.started_at_ms = Some(chrono::Utc::now().timestamp_millis());
                s.completed_at_ms = None;
                s.error_message = None;
                (s.id, s.name.clone())
            } else {
                // Bounds-check before claiming the run, otherwise a bad index
                // leaves `is_running` stuck at true and blocks the whole app.
                st.is_running = false;
                return Err(anyhow::anyhow!("Index fázy {} mimo rozsahu", stage_index));
            }
        };

        // `is_cancelled` is sticky: `cancel()` sets it and only `start_pipeline`
        // ever cleared it. After any cancel, every later single-stage run was
        // killed on its first poll iteration and silently reported `Ok(())`,
        // flipping the stage to `Skipped` without running anything. Clear it
        // here exactly like `start_pipeline` does at its own start.
        self.reset_cancel();

        let (input_wsl, output_wsl, meta_wsl) = {
            let st = self.state.lock().await;
            (
                st.input_video_path_wsl.clone(),
                st.output_video_path_wsl.clone().unwrap_or_default(),
                st.metadata_json_path_wsl.clone().unwrap_or_default(),
            )
        };

        // Refuse to run a stage with no input video. Every stage command
        // interpolates these paths, and stage 1 would fail deep inside ffmpeg
        // with a confusing "No such file or directory" instead of a clear message.
        if stage_id != PipelineStageId::Mux && input_wsl.trim().is_empty() {
            self.mark_stage_failed(
                stage_index,
                "Nie je vybraný vstupný video súbor.".to_string(),
                Some("Pomocou tlačidla 'Vybrať video' najprv vyberte vstupné video.".to_string()),
            )
            .await;
            anyhow::bail!("Nie je vybraný vstupný video súbor.");
        }

        // The TTS/lipsync/mux chain all read the metadata file produced by stage 3.
        // Without it these stages fail with "Chýba súbor metadát", so check up
        // front and name the stage the user actually has to run.
        let requires_metadata = matches!(
            stage_id,
            PipelineStageId::Translate
                | PipelineStageId::Tts
                | PipelineStageId::Lipsync
                | PipelineStageId::Mux
        );
        if requires_metadata && meta_wsl.trim().is_empty() {
            self.mark_stage_failed(
                stage_index,
                "Cesta k súboru utterance_metadata nie je nastavená.".to_string(),
                Some("Spustite najprv fázy 1–3 (Demux, ASR, Preklad).".to_string()),
            )
            .await;
            anyhow::bail!("Cesta k súboru utterance_metadata nie je nastavená.");
        }

        // If simulate mode is on, run mock simulation
        if config.simulate_mode {
            self.run_mock_stage(stage_id, stage_index, log_tx).await?;
            return Ok(());
        }

        // Build CLI command with strict POSIX shell escaping for security and $HOME resolution
        let cmd = self.build_stage_command(stage_id, &input_wsl, &output_wsl, &meta_wsl, config);

        // The stage's Python script reports real progress via `[PROGRESS:n%]`
        // lines. `WslExecutor` parses them, but that value only ever reached the
        // log channel - nothing wrote it into `st.stages[i].progress_percent`,
        // so the UI showed `0.0 %` for a 40-minute ASR run and then jumped
        // straight to 100 %. Tap the channel here and keep the stage state in
        // sync while the child process runs.
        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ProcessLogLine>();
        let progress_task = if log_tx.is_some() {
            let state = self.state.clone();
            let outer_tx = log_tx.clone();
            Some(tokio::spawn(async move {
                while let Some(line) = progress_rx.recv().await {
                    if let Some(pct) = line.progress_percent {
                        let mut st = state.lock().await;
                        if let Some(s) = st.stages.get_mut(stage_index) {
                            // Only advance, never rewind: out-of-order lines from
                            // stdout and stderr must not make the bar jump back.
                            if s.status == StageStatus::Running && pct > s.progress_percent {
                                s.progress_percent = pct;
                            }
                        }
                    }
                    if let Some(tx) = &outer_tx {
                        let _ = tx.send(line);
                    }
                }
            }))
        } else {
            None
        };

        let res = WslExecutor::run_streaming_command(
            &config.wsl_distro,
            &cmd,
            if log_tx.is_some() {
                Some(progress_tx.clone())
            } else {
                None
            },
            // Per-stage timeout. A single flat 1-hour budget killed long ASR and
            // lip-sync runs on large videos; ASR and lip-sync get a longer budget
            // because they scale with video length, while the fast stages stay short.
            Some(stage_timeout(stage_id)),
            Some(self.is_cancelled.clone()),
        )
        .await?;

        // Dropping the sender lets the forwarder task observe end-of-stream and
        // finish; without this it would linger for the app's lifetime.
        drop(progress_tx);
        if let Some(task) = progress_task {
            let _ = task.await;
        }

        // If cancelled by user
        if res.error_kind == Some(ProcessErrorKind::Cancelled) {
            // Returning `Ok(())` here was the worst variant of this bug: if the
            // cancelled stage was the last one, `start_pipeline`'s loop simply
            // ended and fell through to the "PIPELINE ÚSPEŠNE DOKONČENÝ"
            // message with 100 % progress. A cancellation is not a success.
            {
                let mut st = self.state.lock().await;
                st.is_running = false;
                if let Some(s) = st.stages.get_mut(stage_index) {
                    s.status = StageStatus::Skipped;
                    s.user_suggestion =
                        Some("Fáza bola zrušená používateľom pred dokončením.".to_string());
                }
            }
            anyhow::bail!("Fáza {} bola zrušená používateľom.", stage_index + 1);
        }

        // Handle failure and potential LatentSync OOM fallback to MuseTalk
        if !res.success {
            // A missing lip-sync checkout or checkpoint is the single most common
            // cause of stage 6 failing, and MuseTalk needs a strictly smaller VRAM
            // budget than LatentSync anyway. The fallback used to trigger only on a
            // diagnosed OOM, so this case just failed with no suggested action even
            // when MuseTalk was installed and would have worked.
            let lipsync_recoverable = stage_id == PipelineStageId::Lipsync
                && config.lipsync_fallback_on_oom
                && config.lipsync_engine == LipsyncEngine::LatentSync15
                && (res.error_kind == Some(ProcessErrorKind::OutOfMemoryGpu)
                    || res.error_kind == Some(ProcessErrorKind::MissingModelWeights)
                    || res.error_kind == Some(ProcessErrorKind::MissingPackage)
                    || res.error_kind == Some(ProcessErrorKind::RocmDriverError));

            if lipsync_recoverable {
                let reason = match res.error_kind {
                    Some(ProcessErrorKind::MissingModelWeights) => "chýbajúce váhy modelu",
                    Some(ProcessErrorKind::MissingPackage) => "chýbajúci Python balík",
                    Some(ProcessErrorKind::RocmDriverError) => "chyba ROCm ovládača",
                    _ => "vyčerpanie VRAM (OOM)",
                };
                if let Some(ref tx) = log_tx {
                    let _ = tx.send(ProcessLogLine {
                        stream: "system".to_string(),
                        message: format!(
                            "⚠️ LatentSync 1.5 zlyhal ({reason}). Automaticky aktivujem záchranný fallback: MuseTalk Engine (~4.5 GB VRAM)..."
                        ),
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
                    // Same per-stage budget as the primary attempt, not the old
                    // flat hour that applied before timeouts were stage-aware.
                    Some(stage_timeout(stage_id)),
                    Some(self.is_cancelled.clone()),
                )
                .await?;

                if retry_res.success {
                    let mut st = self.state.lock().await;
                    st.is_running = false;
                    st.error_summary = None;
                    st.active_lipsync_engine = "MuseTalk (Fallback)".to_string();
                    if let Some(s) = st.stages.get_mut(stage_index) {
                        s.status = StageStatus::Completed;
                        s.progress_percent = 100.0;
                        s.completed_at_ms = Some(chrono::Utc::now().timestamp_millis());
                        s.user_suggestion =
                            Some("Úspešne dokončené cez záchranný MuseTalk engine.".to_string());
                    }
                    return Ok(());
                }

                // The fallback also failed. Report both errors: showing only the
                // original LatentSync failure hid the fact that MuseTalk was tried
                // and why it did not work either.
                if let Some(ref tx) = log_tx {
                    let _ = tx.send(ProcessLogLine {
                        stream: "system".to_string(),
                        message: format!(
                            "❌ Záchranný MuseTalk engine tiež zlyhal:\n{}",
                            retry_res.stderr.trim()
                        ),
                        timestamp_ms: chrono::Utc::now().timestamp_millis(),
                        is_progress: false,
                        progress_percent: None,
                        step_tag: Some("fallback_failed".to_string()),
                    });
                }
            }

            // Otherwise mark as failed. `res.stderr` was the only field surfaced,
            // but the wrapper's own diagnostics (OOM/ROCm/missing weights) are often
            // the actionable part, so include the remedy when there is one.
            let mut st = self.state.lock().await;
            st.is_running = false;
            let stage_name = if let Some(s) = st.stages.get_mut(stage_index) {
                s.status = StageStatus::Failed;
                s.error_message = Some(res.stderr.clone());
                s.user_suggestion = res.user_remedy.clone();
                s.name.clone()
            } else {
                format!("Fáza {}", stage_index + 1)
            };
            let remedy = res
                .user_remedy
                .map(|r| format!(" Rada: {}", r))
                .unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Fáza {} zlyhala: {}{}",
                stage_name,
                res.stderr.trim(),
                remedy
            ));
        }

        // Success
        {
            let mut st = self.state.lock().await;
            st.is_running = false;
            // A stage that now succeeds must clear the previous failure, or
            // `start_pipeline` keeps bailing on the stale `error_summary` and the
            // user cannot start a new run at all.
            st.error_summary = None;
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
        let venv_setup = PathMapper::bash_var_with_home_expansion(
            "VENV",
            config.venv_path.trim_end_matches('/'),
        );
        let ws_setup = PathMapper::bash_var_with_home_expansion(
            "WORKSPACE",
            config.workspace_dir.trim_end_matches('/'),
        );

        let q_in = PathMapper::escape_bash_arg(input_wsl);
        let q_out = PathMapper::escape_bash_arg(output_wsl);
        let q_meta = PathMapper::escape_bash_arg(meta_wsl);

        let setup_env = format!(
            r#"{0} {1} PYTHON_BIN="$VENV/bin/python"; SCRIPT_DIR="$WORKSPACE/scripts"; "#,
            venv_setup, ws_setup
        );

        match stage_id {
            PipelineStageId::Demux => format!(
                "{}$PYTHON_BIN $SCRIPT_DIR/stage_1_demux.py --input {} --workspace \"$WORKSPACE\"",
                setup_env, q_in
            ),
            PipelineStageId::Asr => format!(
                "{}$PYTHON_BIN $SCRIPT_DIR/stage_2_asr.py --input {} --workspace \"$WORKSPACE\" --engine {} --device {} --model {}",
                setup_env, q_in,
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
                "{}$PYTHON_BIN $SCRIPT_DIR/stage_3_translate.py --workspace \"$WORKSPACE\" --meta {} --model {} --src {} --tgt {}",
                setup_env, q_meta,
                PathMapper::escape_bash_arg(&config.mt_model_id),
                PathMapper::escape_bash_arg(&config.source_lang),
                PathMapper::escape_bash_arg(&config.target_lang)
            ),
            PipelineStageId::Review => "echo 'Review stage completed'".to_string(),
            PipelineStageId::Tts => format!(
                "{}$PYTHON_BIN $SCRIPT_DIR/stage_4_tts.py --workspace \"$WORKSPACE\" --meta {} --engine {} --voice {} --speed {:.2}",
                setup_env, q_meta,
                match config.tts_engine {
                    crate::config::app_config::TtsEngine::Piper => "piper",
                    crate::config::app_config::TtsEngine::Kokoro => "kokoro",
                    crate::config::app_config::TtsEngine::CoquiXtts => "coqui",
                },
                PathMapper::escape_bash_arg(&config.tts_voice),
                config.tts_speed_factor
            ),
            PipelineStageId::Lipsync => format!(
                "{}$PYTHON_BIN $SCRIPT_DIR/stage_5_lipsync.py --input {} --workspace \"$WORKSPACE\" --meta {} --engine {} --batch-size {} --rocm-sdpa-fallback {}",
                setup_env, q_in, q_meta,
                match config.lipsync_engine {
                    LipsyncEngine::LatentSync15 => "latentsync",
                    LipsyncEngine::MuseTalk => "musetalk",
                },
                config.lipsync_batch_size,
                if config.rocm_sdpa_fallback { "1" } else { "0" }
            ),
            PipelineStageId::Mux => format!(
                "{}$PYTHON_BIN $SCRIPT_DIR/stage_6_mux.py --input {} --output {} --workspace \"$WORKSPACE\" --meta {} --ducking {:.1}",
                setup_env, q_in, q_out, q_meta, config.ducking_level_db
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
