use std::process::{Output, Stdio};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::timeout;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

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
    Unknown,
}

pub struct WslExecutor;

impl WslExecutor {
    /// Builds a Command to execute bash inside the target WSL distro (or native on Linux/macOS).
    pub fn build_command(distro: &str, bash_command: &str) -> Command {
        #[cfg(target_os = "windows")]
        {
            let mut cmd = Command::new("wsl.exe");
            cmd.args(["-d", distro, "--", "bash", "-c", bash_command]);
            cmd
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = distro;
            let mut cmd = Command::new("bash");
            cmd.args(["-c", bash_command]);
            cmd
        }
    }

    /// Runs a short command and collects its stdout and stderr.
    pub async fn run_command_output(distro: &str, bash_command: &str) -> Result<Output> {
        let mut cmd = Self::build_command(distro, bash_command);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let out = cmd.output().await.with_context(|| {
            format!("Failed to execute command in WSL distro '{}': {}", distro, bash_command)
        })?;
        Ok(out)
    }

    /// Runs a command with real-time streaming of stdout/stderr to an mpsc sender and timeout.
    pub async fn run_streaming_command(
        distro: &str,
        bash_command: &str,
        log_sender: Option<mpsc::UnboundedSender<ProcessLogLine>>,
        timeout_duration: Option<Duration>,
    ) -> Result<ProcessExecutionResult> {
        let mut cmd = Self::build_command(distro, bash_command);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().with_context(|| {
            format!("Failed to spawn process in WSL distro '{}': {}", distro, bash_command)
        })?;

        let stdout = child.stdout.take().expect("Child stdout was piped");
        let stderr = child.stderr.take().expect("Child stderr was piped");

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let captured_stdout: Vec<String> = Vec::new();
        let _captured_stderr: Vec<String> = Vec::new();

        let sender_stdout = log_sender.clone();
        let sender_stderr = log_sender.clone();

        let stdout_task = tokio::spawn(async move {
            let mut collected = Vec::new();
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
            let mut collected = Vec::new();
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

        let wait_future = child.wait();

        let status_res = if let Some(dur) = timeout_duration {
            match timeout(dur, wait_future).await {
                Ok(res) => res,
                Err(_) => {
                    let _ = child.kill().await;
                    if let Some(ref tx) = log_sender {
                        let _ = tx.send(ProcessLogLine {
                            stream: "system".to_string(),
                            message: format!("Process timed out after {} seconds and was terminated.", dur.as_secs()),
                            timestamp_ms: chrono::Utc::now().timestamp_millis(),
                            is_progress: false,
                            progress_percent: None,
                            step_tag: None,
                        });
                    }
                    return Ok(ProcessExecutionResult {
                        exit_code: -1,
                        success: false,
                        stdout: captured_stdout.join("\n"),
                        stderr: "Process timed out".to_string(),
                        error_kind: Some(ProcessErrorKind::Timeout),
                        user_remedy: Some("Operácia trvala pridlho. Skontrolujte zaťaženie GPU a skúste znížiť rozlíšenie alebo zvoliť MuseTalk.".to_string()),
                    });
                }
            }
        } else {
            wait_future.await
        };

        let stdout_lines = stdout_task.await.unwrap_or_default();
        let stderr_lines = stderr_task.await.unwrap_or_default();

        let exit_code = match status_res {
            Ok(status) => status.code().unwrap_or(-1),
            Err(e) => {
                return Err(anyhow::anyhow!("Process execution failed: {}", e));
            }
        };

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
        // Check custom format [PROGRESS:XX.X%]
        if let Some(pos) = line.find("[PROGRESS:") {
            let slice = &line[pos + 10..];
            if let Some(end) = slice.find('%') {
                if let Ok(val) = slice[..end].trim().parse::<f32>() {
                    return Some(val.clamp(0.0, 100.0));
                }
            }
        }
        // Check tqdm format: 45%|
        if let Some(percent_pos) = line.find("%|") {
            let start = line[..percent_pos].rfind(' ').map(|p| p + 1).unwrap_or(0);
            if let Ok(val) = line[start..percent_pos].trim().parse::<f32>() {
                return Some(val.clamp(0.0, 100.0));
            }
        }
        None
    }

    /// Diagnoses Python / ROCm / PyTorch errors and provides clear Slovak remedies
    pub fn diagnose_error(log: &str) -> (Option<ProcessErrorKind>, Option<String>) {
        let lower = log.to_lowercase();

        if lower.contains("outofmemoryerror") || lower.contains("out of memory") || lower.contains("hip out of memory") || lower.contains("cuda out of memory") {
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

        if lower.contains("xformers") || lower.contains("flashattention") || lower.contains("no kernel image is available for execution") {
            return (
                Some(ProcessErrorKind::XformersIncompatible),
                Some("Detegovaná nekompatibilita xFormers/FlashAttention s ROCm architektúrou. Povoľte v nastaveniach 'ROCm Native SDPA Fallback' pre automatickú náhradu cez natívne PyTorch SDPA jadro.".to_string()),
            );
        }

        if lower.contains("hip error") || lower.contains("rocm") && lower.contains("driver") {
            return (
                Some(ProcessErrorKind::RocmDriverError),
                Some("Chyba ROCm ovládača alebo HIP runtime. Skontrolujte prístup k /dev/kfd a /dev/dri v Ubuntu WSL2 alebo preinštalujte ROCm balíčky.".to_string()),
            );
        }

        if lower.contains("filenotfounderror") && (lower.contains("model") || lower.contains(".pt") || lower.contains(".bin") || lower.contains(".onnx")) {
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

        if lower.contains("ffmpeg") && (lower.contains("invalid data") || lower.contains("error opening")) {
            return (
                Some(ProcessErrorKind::FfmpegError),
                Some("FFmpeg narazil na poškodený vstupný súbor alebo nepodporovaný kodek. Skontrolujte zdrojové video.".to_string()),
            );
        }

        (
            Some(ProcessErrorKind::Unknown),
            Some("Fáza zlyhala s neznámou chybou. Skontrolujte podrobný výstup v Log paneli nižšie.".to_string()),
        )
    }
}
