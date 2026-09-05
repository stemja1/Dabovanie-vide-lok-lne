use crate::wsl::executor::{ProcessLogLine, WslExecutor};
use crate::wsl::path_mapper::PathMapper;
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

            // `distro` is user-editable AppConfig data (`wsl_distro`), embedded here
            // inside a PowerShell string. PowerShell only needs `'` doubled to stay
            // inert, but an unescaped value could otherwise break out of
            // `-ArgumentList '...'` and inject further commands into the
            // surrounding `-Command` script — the same bug class as bod A for bash.
            // The concatenation is wrapped in `(...)` to force PowerShell expression
            // -mode parsing; without it, `-ArgumentList 'a' + 'b'` in command mode
            // would pass `+` as a separate literal argument instead of concatenating.
            let distro_ps = PathMapper::escape_powershell_arg(distro);
            let ps_script = format!(
                r#"Start-Process wsl.exe -ArgumentList ('--install -d ' + {0} + ' --no-launch') -Verb RunAs -Wait"#,
                distro_ps
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

        if let Some(ref tx) = log_tx {
            let _ = tx.send(ProcessLogLine {
                stream: "system".to_string(),
                message: ">>> Spúšťam inštaláciu systémových balíkov Ubuntu (ffmpeg, git, python3-pip, python3-venv, libsndfile1)...".to_string(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                is_progress: false,
                progress_percent: Some(10.0),
                step_tag: Some("system_packages".to_string()),
            });
        }

        let cmd = r#"
export DEBIAN_FRONTEND=noninteractive
apt-get update -y
apt-get install -y --no-install-recommends \
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
echo ">>> Systémové balíky úspešne nainštalované."
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

        // `venv_path`/`workspace_dir` are user-editable AppConfig data — escape them
        // with `PathMapper::escape_bash_arg` before splicing into the shell script.
        // A bare `VENV="{0}"` would still let bash expand `$(...)`/backticks
        // embedded in the config value.
        let venv_q = PathMapper::escape_bash_arg(venv_path);
        let workspace_q = PathMapper::escape_bash_arg(workspace_dir);
        let cmd = format!(
            r#"
export PYTHONUNBUFFERED=1
VENV={0}
WORKSPACE={1}
VENV="${{VENV/#\~/$HOME}}"
WORKSPACE="${{WORKSPACE/#\~/$HOME}}"

mkdir -p "$VENV" "$WORKSPACE"
if [ ! -f "$VENV/bin/python" ]; then
    echo ">>> Vytváram izolované Python virtuálne prostredie v $VENV..."
    python3 -m venv "$VENV" || python3 -m venv --without-pip "$VENV"
fi

if [ ! -f "$VENV/bin/pip" ]; then
    echo ">>> Inštalujem pip do virtuálneho prostredia..."
    curl -sS https://bootstrap.pypa.io/get-pip.py | "$VENV/bin/python" || true
fi

source "$VENV/bin/activate"
echo ">>> Aktualizujem pip, setuptools, wheel..."
pip install --upgrade pip setuptools wheel

echo ">>> Inštalujem PyTorch s podporou AMD ROCm (whl/rocm6.2)..."
pip install --pre torch torchvision torchaudio --index-url https://download.pytorch.org/whl/rocm6.2

echo ">>> Inštalujem dabingové knižnice (transformers, accelerate, piper-tts, kokoro-onnx, soundfile, requests, huggingface_hub)..."
pip install transformers accelerate sentencepiece sacremoses piper-tts kokoro-onnx soundfile librosa scipy pydub ffmpeg-python tqdm requests huggingface_hub
pip install "open_dubbing[coqui]" --no-deps || true

mkdir -p "$WORKSPACE/scripts"
for cand in /mnt/c/Dabovanie-vide-lok-lne-main/scripts /mnt/c/*/Dabovanie-vide-lok-lne*/scripts /mnt/c/*/*/scripts; do
    if [ -d "$cand" ] && [ -f "$cand/stage_1_demux.py" ]; then
        cp -ru "$cand"/*.py "$WORKSPACE/scripts/" 2>/dev/null || true
        break
    fi
done
echo ">>> Python & ROCm prostredie je úspešne nakonfigurované."
"#,
            venv_q, workspace_q
        );

        let res = WslExecutor::run_streaming_command(
            distro,
            &cmd,
            log_tx,
            Some(std::time::Duration::from_secs(3600)),
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

        let venv_q = PathMapper::escape_bash_arg(venv_path);
        let workspace_q = PathMapper::escape_bash_arg(workspace_dir);
        let cmd = format!(
            r#"
export PYTHONUNBUFFERED=1
VENV={0}
WORKSPACE={1}
VENV="${{VENV/#\~/$HOME}}"
WORKSPACE="${{WORKSPACE/#\~/$HOME}}"

mkdir -p "$WORKSPACE"
cd "$WORKSPACE"
source "$VENV/bin/activate" 2>/dev/null || true

# 1. LatentSync 1.5 (Use LatentSync v1.5 for 6.5-8GB VRAM constraint)
if [ ! -d "$WORKSPACE/latentsync" ]; then
    echo ">>> Klonujem repozitár LatentSync (v1.5)..."
    git clone https://github.com/bytedance/LatentSync.git "$WORKSPACE/latentsync"
    cd "$WORKSPACE/latentsync"
    pip install -r requirements.txt || true
    pip install diffusers omegaconf einops decord face-alignment mediapipe
fi

# 2. MuseTalk (Ultra-lightweight fallback engine for ROCm)
cd "$WORKSPACE"
if [ ! -d "$WORKSPACE/musetalk" ]; then
    echo ">>> Klonujem repozitár MuseTalk..."
    git clone https://github.com/TMElyralab/MuseTalk.git "$WORKSPACE/musetalk"
    cd "$WORKSPACE/musetalk"
    pip install -r requirements.txt || true
    pip install mmpose mmcv mmengine
fi
echo ">>> AI Repozitáre pre lip-sync sú úspešne pripravené."
"#,
            venv_q, workspace_q
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

        // `venv_path`/`workspace_dir` are user-editable AppConfig data. Two
        // precautions here, not just one:
        //   1. Escape them with `escape_bash_arg` before the `VENV=...`/`WORKSPACE=...`
        //      bash assignment (a bare double-quoted `"{0}"` would still let bash
        //      expand `$(...)`/backticks embedded in the value).
        //   2. `export WORKSPACE` and read it back inside the embedded Python script
        //      via `os.environ['WORKSPACE']` instead of re-interpolating the raw
        //      value a second time as a Python string literal (`'{1}'`). The whole
        //      `"$PY" -c "..."` script sits inside a bash *double-quoted* string, so
        //      a second raw substitution there would reopen the exact same
        //      injection vector one level down, even with the bash variable fixed.
        let venv_q = PathMapper::escape_bash_arg(venv_path);
        let workspace_q = PathMapper::escape_bash_arg(workspace_dir);
        let py_downloader = format!(
            r#"
export PYTHONUNBUFFERED=1
VENV={0}
WORKSPACE={1}
VENV="${{VENV/#\~/$HOME}}"
WORKSPACE="${{WORKSPACE/#\~/$HOME}}"
export WORKSPACE

PY="$VENV/bin/python"
if [ ! -f "$PY" ]; then
    PY="python3"
fi

"$PY" -c "
import os, sys, requests, shutil

sys.stdout.reconfigure(line_buffering=True)
model_id = '{2}'
workspace = os.path.expanduser(os.environ['WORKSPACE'])
target_dir = os.path.join(workspace, 'models')
os.makedirs(target_dir, exist_ok=True)

print(f'>>> Začínam overovanie a sťahovanie modelu: {{model_id}}', flush=True)

def download_file_with_progress(url, dest_path, desc_name):
    os.makedirs(os.path.dirname(dest_path), exist_ok=True)
    if os.path.exists(dest_path) and os.path.getsize(dest_path) > 0:
        print(f'✓ Súbor {{desc_name}} už existuje ({{os.path.getsize(dest_path) // 1024}} KB).', flush=True)
        return
    print(f'Sťahujem {{desc_name}} z {{url}}...', flush=True)
    r = requests.get(url, stream=True, timeout=60)
    r.raise_for_status()
    total = int(r.headers.get('content-length', 0))
    downloaded = 0
    with open(dest_path, 'wb') as f:
        for chunk in r.iter_content(chunk_size=65536):
            if chunk:
                f.write(chunk)
                downloaded += len(chunk)
                if total > 0:
                    pct = (downloaded / total) * 100
                    if downloaded % (512 * 1024) < 65536 or downloaded >= total:
                        print(f'[PROGRESS:{{pct:.1f}}%] {{desc_name}}: {{downloaded // (1024*1024)}} MB / {{total // (1024*1024)}} MB ({{pct:.1f}}%)', flush=True)
    print(f'✓ Súbor {{desc_name}} úspešne stiahnutý.', flush=True)

if model_id == 'whisper-large-v3-sk':
    asr_dir = os.path.join(workspace, 'models/asr/whisper-large-v3-sk')
    os.makedirs(asr_dir, exist_ok=True)
    print('Sťahujem NaiveNeuron/whisper-large-v3-sk (ASR model pre slovenčinu)...', flush=True)
    try:
        from huggingface_hub import snapshot_download
        snapshot_download(repo_id='NaiveNeuron/whisper-large-v3-sk', local_dir=asr_dir, max_workers=4)
        print('✓ Whisper SK model stiahnutý cez HuggingFace Hub.', flush=True)
    except Exception as e:
        print(f'Skúšam priame sťahovanie konfigurácie Whisper SK: {{e}}', flush=True)
        download_file_with_progress(
            'https://huggingface.co/NaiveNeuron/whisper-large-v3-sk/resolve/main/config.json',
            os.path.join(asr_dir, 'config.json'),
            'whisper_config.json'
        )
    print('✓ Whisper SK model je pripravený.', flush=True)

elif model_id == 'nllb-200-distilled-600m':
    mt_dir = os.path.join(workspace, 'models/mt/nllb-200-distilled-600M')
    os.makedirs(mt_dir, exist_ok=True)
    print('Sťahujem facebook/nllb-200-distilled-600M (SK -> ZH prekladač)...', flush=True)
    try:
        from huggingface_hub import snapshot_download
        snapshot_download(repo_id='facebook/nllb-200-distilled-600M', local_dir=mt_dir, max_workers=4)
        print('✓ NLLB-200 model stiahnutý cez HuggingFace Hub.', flush=True)
    except Exception as e:
        print(f'Skúšam priame sťahovanie konfigurácie NLLB-200: {{e}}', flush=True)
        download_file_with_progress(
            'https://huggingface.co/facebook/nllb-200-distilled-600M/resolve/main/config.json',
            os.path.join(mt_dir, 'config.json'),
            'nllb_config.json'
        )
    print('✓ NLLB-200 model je pripravený.', flush=True)

elif model_id == 'piper-zh-huayan':
    piper_dir = os.path.join(workspace, 'models/tts/piper')
    os.makedirs(piper_dir, exist_ok=True)
    download_file_with_progress(
        'https://huggingface.co/rhasspy/piper-voices/resolve/main/zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx',
        os.path.join(piper_dir, 'zh_CN-huayan-medium.onnx'),
        'zh_CN-huayan-medium.onnx (Piper Hlas)'
    )
    download_file_with_progress(
        'https://huggingface.co/rhasspy/piper-voices/resolve/main/zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx.json',
        os.path.join(piper_dir, 'zh_CN-huayan-medium.onnx.json'),
        'zh_CN-huayan-medium.onnx.json (Konfigurácia)'
    )
    print('✓ Piper TTS čínsky hlas je stiahnutý a overený.', flush=True)

elif model_id == 'kokoro-v019':
    kokoro_dir = os.path.join(workspace, 'models/tts/kokoro')
    os.makedirs(kokoro_dir, exist_ok=True)
    download_file_with_progress(
        'https://huggingface.co/hexgrad/Kokoro-82M/resolve/main/kokoro-v0_19.onnx',
        os.path.join(kokoro_dir, 'kokoro-v0_19.onnx'),
        'kokoro-v0_19.onnx (Kokoro TTS)'
    )
    print('✓ Kokoro TTS model je pripravený.', flush=True)

elif model_id == 'coqui-xtts-v2':
    coqui_dir = os.path.join(workspace, 'models/tts/coqui-xtts-v2')
    os.makedirs(coqui_dir, exist_ok=True)
    print('Sťahujem coqui/XTTS-v2 (viacjazyčný TTS s klonovaním hlasu, licencia CPML)...', flush=True)
    try:
        from huggingface_hub import snapshot_download
        snapshot_download(repo_id='coqui/XTTS-v2', local_dir=coqui_dir, max_workers=4)
        print('✓ Coqui XTTS-v2 checkpoint stiahnutý cez HuggingFace Hub.', flush=True)
    except Exception as e:
        print(f'Skúšam priame sťahovanie konfigurácie Coqui XTTS-v2: {{e}}', flush=True)
        download_file_with_progress(
            'https://huggingface.co/coqui/XTTS-v2/resolve/main/config.json',
            os.path.join(coqui_dir, 'config.json'),
            'coqui_xtts_v2_config.json'
        )
    print('✓ Coqui XTTS-v2 checkpoint je pripravený.', flush=True)

elif model_id == 'latentsync-1-5':
    ls_dir = os.path.join(workspace, 'models/lipsync/latentsync')
    os.makedirs(ls_dir, exist_ok=True)
    download_file_with_progress(
        'https://huggingface.co/ByteDance/LatentSync/resolve/main/latentsync_unet.pt',
        os.path.join(ls_dir, 'latentsync_unet.pt'),
        'latentsync_unet.pt (LatentSync 1.5 UNet)'
    )
    print('✓ LatentSync 1.5 váhy sú pripravené.', flush=True)

elif model_id == 'musetalk-weights':
    mt_dir = os.path.join(workspace, 'models/lipsync/musetalk')
    os.makedirs(mt_dir, exist_ok=True)
    print('Sťahujem TMElyralab/MuseTalk (odľahčený fallback lip-sync model)...', flush=True)
    try:
        from huggingface_hub import snapshot_download
        snapshot_download(
            repo_id='TMElyralab/MuseTalk',
            local_dir=os.path.dirname(mt_dir.rstrip('/')),
            allow_patterns=['musetalk/*'],
            max_workers=4,
        )
        print('✓ MuseTalk váhy stiahnuté cez HuggingFace Hub.', flush=True)
    except Exception as e:
        print(f'Chyba pri sťahovaní MuseTalk cez HuggingFace Hub: {{e}}', flush=True)
        raise
    print('✓ MuseTalk váhy sú pripravené.', flush=True)

print('HOTOVO', flush=True)
"
"#,
            venv_q, workspace_q, model_id
        );

        let res = WslExecutor::run_streaming_command(
            distro,
            &py_downloader,
            log_tx,
            Some(std::time::Duration::from_secs(3600)),
            Some(self.is_cancelled.clone()),
        )
        .await?;
        Ok(res.success)
    }
}
