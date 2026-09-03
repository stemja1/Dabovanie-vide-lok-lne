use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::{Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

pub const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessLogLine {
    pub stream: String, // "stdout" | "stderr" | "system"
    pub message: String,
    pub timestamp_ms: i64,
    pub is_progress: bool,
    pub progress_percent: Option<f32>,
    pub step_tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessExecutionResult {
    pub exit_code: i32,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub error_kind: Option<ProcessErrorKind>,
    pub user_remedy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessErrorKind {
    OutOfMemoryGpu,
    OutOfMemorySystem,
    RocmDriverError,
    XformersIncompatible,
    MissingModelWeights,
    MissingPackage,
    FfmpegError,
    Timeout,
    Cancelled,
    Unknown,
}

pub struct WslExecutor;

impl WslExecutor {
    /// Builds a Command to execute bash inside the target WSL distro without spawning console windows.
    pub fn build_command(distro: &str, bash_command: &str) -> Command {
        Self::build_command_with_user(distro, None, bash_command)
    }

    /// Builds a Command to execute bash as root inside target WSL distro.
    pub fn build_command_as_root(distro: &str, bash_command: &str) -> Command {
        Self::build_command_with_user(distro, Some("root"), bash_command)
    }

    /// Builds a Command with optional specified user (e.g. "root" or default).
    pub fn build_command_with_user(
        distro: &str,
        user: Option<&str>,
        bash_command: &str,
    ) -> Command {
        #[cfg(target_os = "windows")]
        {
            let mut cmd = Command::new("wsl.exe");
            cmd.creation_flags(CREATE_NO_WINDOW);
            if let Some(u) = user {
                cmd.args(["-d", distro, "-u", u, "--", "bash", "-c", bash_command]);
            } else {
                cmd.args(["-d", distro, "--", "bash", "-c", bash_command]);
            }
            cmd
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = distro;
            let _ = user;
            let mut cmd = Command::new("bash");
            cmd.args(["-c", bash_command]);
            cmd
        }
    }

    /// Runs a short command and collects its stdout and stderr safely.
    pub async fn run_command_output(distro: &str, bash_command: &str) -> Result<Output> {
        let mut cmd = Self::build_command(distro, bash_command);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let out = cmd.output().await.with_context(|| {
            format!(
                "Failed to execute command in WSL distro '{}': {}",
                distro, bash_command
            )
        })?;
        Ok(out)
    }

    /// Runs a streaming command as default user
    pub async fn run_streaming_command(
        distro: &str,
        bash_command: &str,
        log_sender: Option<mpsc::UnboundedSender<ProcessLogLine>>,
        timeout_duration: Option<Duration>,
        cancel_flag: Option<Arc<AtomicBool>>,
    ) -> Result<ProcessExecutionResult> {
        Self::run_streaming_command_internal(
            distro,
            None,
            bash_command,
            log_sender,
            timeout_duration,
            cancel_flag,
        )
        .await
    }

    /// Runs a streaming command as root (e.g. for apt-get install)
    pub async fn run_streaming_command_as_root(
        distro: &str,
        bash_command: &str,
        log_sender: Option<mpsc::UnboundedSender<ProcessLogLine>>,
        timeout_duration: Option<Duration>,
        cancel_flag: Option<Arc<AtomicBool>>,
    ) -> Result<ProcessExecutionResult> {
        Self::run_streaming_command_internal(
            distro,
            Some("root"),
            bash_command,
            log_sender,
            timeout_duration,
            cancel_flag,
        )
        .await
    }

    /// Internal implementation for streaming process runner
    async fn run_streaming_command_internal(
        distro: &str,
        user: Option<&str>,
        bash_command: &str,
        log_sender: Option<mpsc::UnboundedSender<ProcessLogLine>>,
        timeout_duration: Option<Duration>,
        cancel_flag: Option<Arc<AtomicBool>>,
    ) -> Result<ProcessExecutionResult> {
        let mut cmd = Self::build_command_with_user(distro, user, bash_command);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().with_context(|| {
            format!(
                "Failed to spawn process in WSL distro '{}': {}",
                distro, bash_command
            )
        })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Nepodarilo sa zachytiť stdout procesu"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow::anyhow!("Nepodarilo sa zachytiť stderr procesu"))?;

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let sender_stdout = log_sender.clone();
        let sender_stderr = log_sender.clone();

        let stdout_task = tokio::spawn(async move {
            let mut collected: Vec<String> = Vec::new();
            while let Ok(Some(line)) = stdout_reader.next_line().await {
                let parsed_progress = Self::parse_progress_line(&line);
                if let Some(ref tx) = sender_stdout {
                    let log = ProcessLogLine {
                        stream: "stdout".to_string(),
                        message: line.clone(),
                        timestamp_ms: chrono::Utc::now().timestamp_millis(),
                        is_progress: parsed_progress.is_some(),
                        progress_percent: parsed_progress,
                        step_tag: None,
                    };
                    let _ = tx.send(log);
                }
                collected.push(line);
            }
            collected
        });

        let stderr_task = tokio::spawn(async move {
            let mut collected: Vec<String> = Vec::new();
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                let parsed_progress = Self::parse_progress_line(&line);
                if let Some(ref tx) = sender_stderr {
                    let log = ProcessLogLine {
                        stream: "stderr".to_string(),
                        message: line.clone(),
                        timestamp_ms: chrono::Utc::now().timestamp_millis(),
                        is_progress: parsed_progress.is_some(),
                        progress_percent: parsed_progress,
                        step_tag: None,
                    };
                    let _ = tx.send(log);
                }
                collected.push(line);
            }
            collected
        });

        // Polling loop to support both timeout and immediate cancellation
        let poll_interval = Duration::from_millis(100);
        let start_time = std::time::Instant::now();

        let exit_status = loop {
            // Check cancellation
            if let Some(ref flag) = cancel_flag {
                if flag.load(Ordering::SeqCst) {
                    let _ = child.kill().await;
                    if let Some(ref tx) = log_sender {
                        let _ = tx.send(ProcessLogLine {
                            stream: "system".to_string(),
                            message:
                                "Proces bol manuálne zrušený používateľom a bezpečne ukončený."
                                    .to_string(),
                            timestamp_ms: chrono::Utc::now().timestamp_millis(),
                            is_progress: false,
                            progress_percent: None,
                            step_tag: None,
                        });
                    }
                    return Ok(ProcessExecutionResult {
                        exit_code: -1,
                        success: false,
                        stdout: String::new(),
                        stderr: "Operácia bola zrušená používateľom.".to_string(),
                        error_kind: Some(ProcessErrorKind::Cancelled),
                        user_remedy: None,
                    });
                }
            }

            // Check timeout
            if let Some(max_dur) = timeout_duration {
                if start_time.elapsed() >= max_dur {
                    let _ = child.kill().await;
                    if let Some(ref tx) = log_sender {
                        let _ = tx.send(ProcessLogLine {
                            stream: "system".to_string(),
                            message: format!(
                                "Proces vypršal po {} sekundách a bol ukončený.",
                                max_dur.as_secs()
                            ),
                            timestamp_ms: chrono::Utc::now().timestamp_millis(),
                            is_progress: false,
                            progress_percent: None,
                            step_tag: None,
                        });
                    }
                    return Ok(ProcessExecutionResult {
                        exit_code: -1,
                        success: false,
                        stdout: String::new(),
                        stderr: "Process timed out".to_string(),
                        error_kind: Some(ProcessErrorKind::Timeout),
                        user_remedy: Some("Operácia trvala pridlho. Skontrolujte zaťaženie GPU a skúste znížiť rozlíšenie alebo zvoliť MuseTalk.".to_string()),
                    });
                }
            }

            // Check if process finished
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    tokio::time::sleep(poll_interval).await;
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Chyba pri čakaní na proces: {}", e));
                }
            }
        };

        let stdout_lines = stdout_task.await.unwrap_or_default();
        let stderr_lines = stderr_task.await.unwrap_or_default();

        let exit_code = exit_status.code().unwrap_or(-1);
        let full_stdout = stdout_lines.join("\n");
        let full_stderr = stderr_lines.join("\n");
        let combined_logs = format!("{}\n{}", full_stdout, full_stderr);

        let (error_kind, remedy) = if exit_code != 0 {
            Self::diagnose_error(&combined_logs)
        } else {
            (None, None)
        };

        Ok(ProcessExecutionResult {
            exit_code,
            success: exit_code == 0,
            stdout: full_stdout,
            stderr: full_stderr,
            error_kind,
            user_remedy: remedy,
        })
    }

    /// Extracts numerical percentage from progress bars (e.g. `[PROGRESS:45.5%]` or `45%|...`)
    pub fn parse_progress_line(line: &str) -> Option<f32> {
        if let Some(pos) = line.find("[PROGRESS:") {
            let slice = &line[pos + 10..];
            if let Some(end) = slice.find('%') {
                if let Ok(val) = slice[..end].trim().parse::<f32>() {
                    return Some(val.clamp(0.0, 100.0));
                }
            }
        }
        if let Some(percent_pos) = line.find("%|") {
            let start = line[..percent_pos].rfind(' ').map(|p| p + 1).unwrap_or(0);
            if let Ok(val) = line[start..percent_pos].trim().parse::<f32>() {
                return Some(val.clamp(0.0, 100.0));
            }
        }
        None
    }

    /// Diagnoses Python / ROCm / PyTorch errors and provides clear remedies
    pub fn diagnose_error(log: &str) -> (Option<ProcessErrorKind>, Option<String>) {
        let lower = log.to_lowercase();

        if lower.contains("outofmemoryerror")
            || lower.contains("out of memory")
            || lower.contains("hip out of memory")
            || lower.contains("cuda out of memory")
        {
            return (
                Some(ProcessErrorKind::OutOfMemoryGpu),
                Some("GPU VRAM bola vyčerpaná (OOM). Odporúčanie: Prepnite lip-sync engine na 'MuseTalk' (šetrí ~3.5 GB VRAM) alebo znížte rozlíšenie vstupného videa na 720p v nastaveniach.".to_string()),
            );
        }

        if lower.contains("killed") || lower.contains("oom-killer") {
            return (
                Some(ProcessErrorKind::OutOfMemorySystem),
                Some("Systémová RAM bola vyčerpaná a proces bol ukončený OS. Uistite sa, že nemáte na pozadí spustené iné náročné aplikácie.".to_string()),
            );
        }

        if lower.contains("xformers")
            || lower.contains("flashattention")
            || lower.contains("no kernel image is available for execution")
        {
            return (
                Some(ProcessErrorKind::XformersIncompatible),
                Some("Detegovaná nekompatibilita xFormers/FlashAttention s ROCm architektúrou. Povoľte v nastaveniach 'ROCm Native SDPA Fallback' pre automatickú náhradu cez natívne PyTorch SDPA jadro.".to_string()),
            );
        }

        if lower.contains("hip error") || (lower.contains("rocm") && lower.contains("driver")) {
            return (
                Some(ProcessErrorKind::RocmDriverError),
                Some("Chyba ROCm ovládača alebo HIP runtime. Skontrolujte prístup k /dev/kfd a /dev/dri v Ubuntu WSL2 alebo preinštalujte ROCm balíčky.".to_string()),
            );
        }

        if lower.contains("filenotfounderror")
            && (lower.contains("model")
                || lower.contains(".pt")
                || lower.contains(".bin")
                || lower.contains(".onnx"))
        {
            return (
                Some(ProcessErrorKind::MissingModelWeights),
                Some("Chýbajú váhy modelu. Otvorte 'Setup Wizard / Diagnostika' a stiahnite chýbajúce checkpointy.".to_string()),
            );
        }

        if lower.contains("modulenotfounderror") || lower.contains("importerror") {
            return (
                Some(ProcessErrorKind::MissingPackage),
                Some("Chýba potrebný Python balík vo virtuálnom prostredí. Spustite Setup Wizard pre kontrolu a doinštalovanie závislostí.".to_string()),
            );
        }

        if lower.contains("ffmpeg")
            && (lower.contains("invalid data") || lower.contains("error opening"))
        {
            return (
                Some(ProcessErrorKind::FfmpegError),
                Some("FFmpeg narazil na poškodený vstupný súbor alebo nepodporovaný kodek. Skontrolujte zdrojové video.".to_string()),
            );
        }

        (
            Some(ProcessErrorKind::Unknown),
            Some(
                "Fáza zlyhala s neznámou chybou. Skontrolujte podrobný výstup v Log paneli nižšie."
                    .to_string(),
            ),
        )
    }
}
