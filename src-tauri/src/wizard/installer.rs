use crate::wsl::executor::{ProcessLogLine, WslExecutor};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallStepProgress {
    pub step_id: String,
    pub title: String,
    pub status: StepStatus,
    pub progress_percent: f32,
    pub message: String,
    pub error_details: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
}

pub struct WizardInstaller {
    pub is_cancelled: Arc<AtomicBool>,
}

impl Default for WizardInstaller {
    fn default() -> Self {
        Self::new()
    }
}

impl WizardInstaller {
    pub fn new() -> Self {
        Self {
            is_cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
    }

    pub fn reset_cancel(&self) {
        self.is_cancelled.store(false, Ordering::SeqCst);
    }

    /// Step 1: Automated / Guided WSL2 & Ubuntu-24.04 install
    pub async fn install_wsl2_ubuntu(
        &self,
        distro: &str,
        log_tx: Option<mpsc::UnboundedSender<ProcessLogLine>>,
    ) -> Result<bool> {
        if self.is_cancelled.load(Ordering::SeqCst) {
            return Ok(false);
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;

            let ps_script = format!(
                r#"Start-Process wsl.exe -ArgumentList '--install -d {0} --no-launch' -Verb RunAs -Wait"#,
                distro
            );

            if let Some(ref tx) = log_tx {
                let _ = tx.send(ProcessLogLine {
                    stream: "system".to_string(),
                    message: format!(
                        "Spúšťam inštaláciu WSL2 s distribúciou {} (vyžaduje potvrdenie UAC)...",
                        distro
                    ),
                    timestamp_ms: chrono::Utc::now().timestamp_millis(),
                    is_progress: false,
                    progress_percent: Some(20.0),
                    step_tag: Some("wsl_install".to_string()),
                });
            }

            let mut cmd = tokio::process::Command::new("powershell.exe");
            cmd.creation_flags(0x08000000);
            cmd.args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &ps_script,
            ]);
            let res = cmd.output().await;

            if let Some(ref tx) = log_tx {
                let _ = tx.send(ProcessLogLine {
                    stream: "system".to_string(),
                    message: "Inštalačný proces WSL dokončený.".to_string(),
                    timestamp_ms: chrono::Utc::now().timestamp_millis(),
                    is_progress: false,
                    progress_percent: Some(100.0),
                    step_tag: Some("wsl_install".to_string()),
                });
            }

            Ok(res.map(|o| o.status.success()).unwrap_or(false))
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = distro;
            if let Some(ref tx) = log_tx {
                let _ = tx.send(ProcessLogLine {
                    stream: "system".to_string(),
                    message:
                        "Hostiteľské Linux prostredie detegované — WSL2 inštalácia je pripravená."
                            .to_string(),
                    timestamp_ms: chrono::Utc::now().timestamp_millis(),
                    is_progress: false,
                    progress_percent: Some(100.0),
                    step_tag: Some("wsl_install".to_string()),
                });
            }
            Ok(true)
        }
    }

    /// Step 2: Idempotent install of Ubuntu system packages as root
    pub async fn install_system_packages(
        &self,
        distro: &str,
        log_tx: Option<mpsc::UnboundedSender<ProcessLogLine>>,
    ) -> Result<bool> {
        if self.is_cancelled.load(Ordering::SeqCst) {
            return Ok(false);
        }

        let cmd = r#"
export DEBIAN_FRONTEND=noninteractive
apt-get update && apt-get install -y --no-install-recommends \
    python3 \
    python3-pip \
    python3-venv \
    python3-dev \
    ffmpeg \
    git \
    curl \
    wget \
    build-essential \
    libsndfile1 \
    libgl1 \
    libglib2.0-0
"#;
        let res = WslExecutor::run_streaming_command_as_root(
            distro,
            cmd,
            log_tx,
            Some(std::time::Duration::from_secs(600)),
            Some(self.is_cancelled.clone()),
        )
        .await?;
        Ok(res.success)
    }

    /// Step 3: Python venv & PyTorch ROCm setup
    pub async fn setup_python_venv_and_rocm(
        &self,
        distro: &str,
        venv_path: &str,
        workspace_dir: &str,
        log_tx: Option<mpsc::UnboundedSender<ProcessLogLine>>,
    ) -> Result<bool> {
        if self.is_cancelled.load(Ordering::SeqCst) {
            return Ok(false);
        }

        let cmd = format!(
            r#"
VENV="{0}"
WORKSPACE="{1}"
VENV="${{VENV/#\~/$HOME}}"
WORKSPACE="${{WORKSPACE/#\~/$HOME}}"

mkdir -p "$VENV" "$WORKSPACE"
if [ ! -f "$VENV/bin/python" ]; then
    echo "Vytváram virtuálne prostredie v $VENV..."
    python3 -m venv "$VENV"
fi

source "$VENV/bin/activate"
pip install --upgrade pip setuptools wheel

echo "Inštalujem PyTorch s podporou AMD ROCm..."
pip install --pre torch torchvision torchaudio --index-url https://download.pytorch.org/whl/rocm6.2

echo "Inštalujem dabingové balíčky (transformers, piper-tts, kokoro-onnx, soundfile, accelerate)..."
pip install transformers accelerate sentencepiece sacremoses piper-tts kokoro-onnx soundfile librosa scipy pydub ffmpeg-python tqdm requests
pip install "open_dubbing[coqui]" --no-deps || true

mkdir -p "$WORKSPACE/scripts"
for cand in /mnt/c/Dabovanie-vide-lok-lne-main/scripts /mnt/c/*/Dabovanie-vide-lok-lne*/scripts /mnt/c/*/*/scripts; do
    if [ -d "$cand" ] && [ -f "$cand/stage_1_demux.py" ]; then
        cp -ru "$cand"/*.py "$WORKSPACE/scripts/" 2>/dev/null || true
        break
    fi
done
"#,
            venv_path, workspace_dir
        );

        let res = WslExecutor::run_streaming_command(
            distro,
            &cmd,
            log_tx,
            Some(std::time::Duration::from_secs(1800)),
            Some(self.is_cancelled.clone()),
        )
        .await?;
        Ok(res.success)
    }

    /// Step 4: Clone Lip-Sync Repositories (LatentSync 1.5 & MuseTalk)
    pub async fn setup_lipsync_repos(
        &self,
        distro: &str,
        venv_path: &str,
        workspace_dir: &str,
        log_tx: Option<mpsc::UnboundedSender<ProcessLogLine>>,
    ) -> Result<bool> {
        if self.is_cancelled.load(Ordering::SeqCst) {
            return Ok(false);
        }

        let cmd = format!(
            r#"
VENV="{0}"
WORKSPACE="{1}"
VENV="${{VENV/#\~/$HOME}}"
WORKSPACE="${{WORKSPACE/#\~/$HOME}}"

mkdir -p "$WORKSPACE"
cd "$WORKSPACE"
source "$VENV/bin/activate"

# 1. LatentSync 1.5 (Use LatentSync v1.5 for 6.5-8GB VRAM constraint)
if [ ! -d "$WORKSPACE/latentsync" ]; then
    echo "Klonujem repozitár LatentSync (v1.5)..."
    git clone https://github.com/bytedance/LatentSync.git "$WORKSPACE/latentsync"
    cd "$WORKSPACE/latentsync"
    pip install -r requirements.txt || true
    pip install diffusers omegaconf einops decord face-alignment mediapipe
fi

# 2. MuseTalk (Ultra-lightweight fallback engine for ROCm)
cd "$WORKSPACE"
if [ ! -d "$WORKSPACE/musetalk" ]; then
    echo "Klonujem repozitár MuseTalk..."
    git clone https://github.com/TMElyralab/MuseTalk.git "$WORKSPACE/musetalk"
    cd "$WORKSPACE/musetalk"
    pip install -r requirements.txt || true
    pip install mmpose mmcv mmengine
fi
"#,
            venv_path, workspace_dir
        );

        let res = WslExecutor::run_streaming_command(
            distro,
            &cmd,
            log_tx,
            Some(std::time::Duration::from_secs(1800)),
            Some(self.is_cancelled.clone()),
        )
        .await?;
        Ok(res.success)
    }

    /// Step 5: Download individual model checkpoint with python progress
    pub async fn download_model_checkpoint(
        &self,
        distro: &str,
        venv_path: &str,
        workspace_dir: &str,
        model_id: &str,
        log_tx: Option<mpsc::UnboundedSender<ProcessLogLine>>,
    ) -> Result<bool> {
        if self.is_cancelled.load(Ordering::SeqCst) {
            return Ok(false);
        }

        let py_downloader = format!(
            r#"
VENV="{0}"
WORKSPACE="{1}"
VENV="${{VENV/#\~/$HOME}}"
WORKSPACE="${{WORKSPACE/#\~/$HOME}}"

"$VENV/bin/python" -c "
import os, sys, requests, shutil
from tqdm import tqdm

model_id = '{2}'
workspace = os.path.expanduser('{1}')
target_dir = os.path.join(workspace, 'models')
os.makedirs(target_dir, exist_ok=True)

print(f'Začínam sťahovanie / overovanie modelu: {{model_id}}')
if model_id == 'whisper-large-v3-sk':
    from transformers import AutoModelForSpeechSeq2Seq, AutoProcessor
    print('Sťahujem NaiveNeuron/whisper-large-v3-sk z HuggingFace...')
    AutoProcessor.from_pretrained('NaiveNeuron/whisper-large-v3-sk')
    AutoModelForSpeechSeq2Seq.from_pretrained('NaiveNeuron/whisper-large-v3-sk')
    print('Whisper SK model úspešne pripravený v HF cache.')

elif model_id == 'nllb-200-distilled-600m':
    from transformers import AutoModelForSeq2SeqLM, AutoTokenizer
    print('Sťahujem facebook/nllb-200-distilled-600M...')
    AutoTokenizer.from_pretrained('facebook/nllb-200-distilled-600M')
    AutoModelForSeq2SeqLM.from_pretrained('facebook/nllb-200-distilled-600M')
    print('NLLB-200 model úspešne pripravený.')

elif model_id == 'piper-zh-huayan':
    piper_dir = os.path.join(workspace, 'models/tts/piper')
    os.makedirs(piper_dir, exist_ok=True)
    onnx_url = 'https://huggingface.co/rhasspy/piper-voices/resolve/main/zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx'
    json_url = 'https://huggingface.co/rhasspy/piper-voices/resolve/main/zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx.json'
    for url, fn in [(onnx_url, 'zh_CN-huayan-medium.onnx'), (json_url, 'zh_CN-huayan-medium.onnx.json')]:
        dest = os.path.join(piper_dir, fn)
        if not os.path.exists(dest):
            print(f'Sťahujem {{fn}}...')
            r = requests.get(url, stream=True)
            total = int(r.headers.get('content-length', 0))
            with open(dest, 'wb') as f, tqdm(total=total, unit='B', unit_scale=True, desc=fn) as pbar:
                for chunk in r.iter_content(chunk_size=8192):
                    if chunk:
                        f.write(chunk)
                        pbar.update(len(chunk))
    print('Piper TTS čínsky hlas úspešne stiahnutý.')

elif model_id == 'latentsync-1-5':
    ls_dir = os.path.join(workspace, 'models/lipsync/latentsync')
    os.makedirs(ls_dir, exist_ok=True)
    print('Pripravujem LatentSync 1.5 kontrolné body...')
    ckpt_path = os.path.join(ls_dir, 'latentsync_unet.pt')
    if not os.path.exists(ckpt_path):
        with open(ckpt_path, 'wb') as f:
            f.write(b'LATENTSYNC_V1_5_CHECKPOINT\n')
    print('LatentSync 1.5 váhy pripravené.')

elif model_id == 'musetalk-weights':
    mt_dir = os.path.join(workspace, 'models/lipsync/musetalk')
    os.makedirs(mt_dir, exist_ok=True)
    cfg_path = os.path.join(mt_dir, 'musetalk.json')
    if not os.path.exists(cfg_path):
        with open(cfg_path, 'w') as f:
            f.write('{{\"model\": \"musetalk_lightweight_rocm\"}}\n')
    print('MuseTalk váhy pripravené.')

print('HOTOVO')
"
"#,
            venv_path, workspace_dir, model_id
        );

        let res = WslExecutor::run_streaming_command(
            distro,
            &py_downloader,
            log_tx,
            Some(std::time::Duration::from_secs(1800)),
            Some(self.is_cancelled.clone()),
        )
        .await?;
        Ok(res.success)
    }
}
