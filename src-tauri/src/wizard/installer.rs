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

    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::SeqCst)
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

            // Report what actually happened. The success line used to be
            // emitted before the result was even inspected, so a failed or
            // cancelled UAC-elevated install still showed "dokončený / 100%".
            match res {
                Ok(o) if o.status.success() => {
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
                    Ok(true)
                }
                Ok(o) => {
                    let detail = String::from_utf8_lossy(&o.stderr)
                        .trim()
                        .chars()
                        .take(400)
                        .collect::<String>();
                    if let Some(ref tx) = log_tx {
                        let _ = tx.send(ProcessLogLine {
                            stream: "system".to_string(),
                            message: format!(
                                "Inštalácia WSL zlyhala (exit {}).{}{}",
                                o.status.code().unwrap_or(-1),
                                if detail.is_empty() { "" } else { ": " },
                                detail
                            ),
                            timestamp_ms: chrono::Utc::now().timestamp_millis(),
                            is_progress: false,
                            progress_percent: None,
                            step_tag: Some("error".to_string()),
                        });
                    }
                    Ok(false)
                }
                Err(e) => {
                    if let Some(ref tx) = log_tx {
                        let _ = tx.send(ProcessLogLine {
                            stream: "system".to_string(),
                            message: format!("Inštaláciu WSL sa nepodarilo spustiť: {}", e),
                            timestamp_ms: chrono::Utc::now().timestamp_millis(),
                            is_progress: false,
                            progress_percent: None,
                            step_tag: Some("error".to_string()),
                        });
                    }
                    Ok(false)
                }
            }
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
# `set -e` matters: without it bash returns the exit code of the LAST command
# only. These scripts used to end with an `echo`, so a failed `apt-get install`
# still reported the step as successful and the wizard moved on.
set -e
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
    libglib2.0-0t64 \
    libgomp1
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

        let venv_setup = PathMapper::bash_var_with_home_expansion("VENV", venv_path);
        let ws_setup = PathMapper::bash_var_with_home_expansion("WORKSPACE", workspace_dir);

        let cmd = format!(
            r#"
set -e
export PYTHONUNBUFFERED=1
{0}
{1}
export VENV WORKSPACE

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
if pip install --pre torch torchvision torchaudio --index-url https://download.pytorch.org/whl/rocm6.2; then
    echo ">>> ROCm PyTorch nainštalovaný."
else
    echo ">>> ROCm PyTorch sa nepodarilo nainštalovať, skúšam CPU build..."
    pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cpu
fi

echo ">>> Inštalujem dabingové knižnice..."
# faster-whisper: the ASR engine is selectable in Settings, but the package was
# never installed, so choosing it always died with ModuleNotFoundError.
# opencv-python-headless + python_speech_features + moviepy/imageio: needed by
# the MuseTalk / LatentSync stacks.
pip install transformers accelerate sentencepiece sacremoses faster-whisper piper-tts kokoro-onnx soundfile librosa scipy pydub ffmpeg-python tqdm requests huggingface_hub opencv-python-headless python_speech_features moviepy imageio
pip install "open_dubbing[coqui]" --no-deps || true

echo ">>> Python & ROCm prostredie je úspešne nakonfigurované."
echo ">>> (Python skripty sa synchronizujú z aplikačných resources pri štarte pipeline.)"
"#,
            venv_setup, ws_setup
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

        let venv_setup = PathMapper::bash_var_with_home_expansion("VENV", venv_path);
        let ws_setup = PathMapper::bash_var_with_home_expansion("WORKSPACE", workspace_dir);

        let cmd = format!(
            r#"
set -e
export PYTHONUNBUFFERED=1
{0}
{1}
export VENV WORKSPACE

mkdir -p "$WORKSPACE"
cd "$WORKSPACE"
source "$VENV/bin/activate" 2>/dev/null || true

# 1. LatentSync 1.5 (Use LatentSync v1.5 for 6.5-8GB VRAM constraint)
# Clone into the capitalised directory names that stage_5_lipsync.py looks for.
# The filesystem inside WSL is case-sensitive, so a lowercase clone path would
# never be found and stage 5 would always fail with
# "Modul latentsync ani inferenčný skript neboli nájdené."
LATENTSYNC_DIR="LatentSync"
MUSETALK_DIR="MuseTalk"

if [ ! -d "$WORKSPACE/$LATENTSYNC_DIR" ]; then
    echo ">>> Klonujem repozitár LatentSync (v1.5)..."
    git clone --depth 1 https://github.com/bytedance/LatentSync.git "$WORKSPACE/$LATENTSYNC_DIR"
    cd "$WORKSPACE/$LATENTSYNC_DIR"
    # LatentSync's requirements.txt pins `torch==2.5.1` and adds a CUDA
    # `--extra-index-url`, which REPLACES the ROCm build installed above and
    # leaves `torch.cuda.is_available()` false on AMD. Install only the packages
    # that torch itself does not already provide.
    echo ">>> Inštalujem LatentSync závislosti bez prepísania PyTorch..."
    pip install -r requirements.txt --no-deps || true
    pip install diffusers omegaconf einops face-alignment
    # Upstream `scripts/inference.py` loads the audio encoder from
    # `checkpoints/whisper/tiny.pt`, which is not part of the git repo. Without
    # it the default engine dies inside `torch.load` with a bare
    # FileNotFoundError that points nowhere useful.
    echo ">>> Sťahujem LatentSync audio encoder (whisper/tiny.pt)..."
    mkdir -p checkpoints/whisper
    curl -fL --retry 3 -o checkpoints/whisper/tiny.pt \
        https://huggingface.co/ByteDance/LatentSync/resolve/main/whisper/tiny.pt \
        || echo ">>> UPOZORNENIE: whisper/tiny.pt sa nepodarilo stiahnuť, lip-sync v LatentSync nebude fungovať."
fi

# 2. MuseTalk (Ultra-lightweight fallback engine for ROCm)
cd "$WORKSPACE"
if [ ! -d "$WORKSPACE/$MUSETALK_DIR" ]; then
    echo ">>> Klonujem repozitár MuseTalk..."
    git clone --depth 1 https://github.com/TMElyralab/MuseTalk.git "$WORKSPACE/$MUSETALK_DIR"
    cd "$WORKSPACE/$MUSETALK_DIR"
    # MuseTalk pins numpy==1.23.5 and tensorflow==2.12.0, neither of which has a
    # cp312 wheel, so the install fails outright on Ubuntu 24.04. Keep
    # --no-deps and add only what the pipeline actually imports.
    pip install -r requirements.txt --no-deps || true
    pip install librosa opencv-python-headless tqdm huggingface_hub omegaconf
fi
echo ">>> AI Repozitáre pre lip-sync sú úspešne pripravené."
"#,
            venv_setup, ws_setup
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

        let venv_setup = PathMapper::bash_var_with_home_expansion("VENV", venv_path);
        let ws_setup = PathMapper::bash_var_with_home_expansion("WORKSPACE", workspace_dir);

        let py_downloader = format!(
            r#"
export PYTHONUNBUFFERED=1
{0}
{1}
export VENV WORKSPACE

mkdir -p "$WORKSPACE" "$WORKSPACE/models"

if [ ! -f "$VENV/bin/python" ]; then
    echo ">>> Inicializujem virtuálne prostredie v $VENV..."
    python3 -m venv "$VENV" || python3 -m venv --without-pip "$VENV"
    curl -sS https://bootstrap.pypa.io/get-pip.py | "$VENV/bin/python" 2>/dev/null || true
fi

if [ -f "$VENV/bin/pip" ]; then
    "$VENV/bin/python" -c "import requests, huggingface_hub" 2>/dev/null || "$VENV/bin/pip" install requests huggingface_hub tqdm
fi

PY="$VENV/bin/python"
if [ ! -f "$PY" ]; then
    PY="python3"
fi

export AIDUBBING_MODEL_ID={2}

"$PY" -c "
import os, sys, requests, shutil, hashlib

sys.stdout.reconfigure(line_buffering=True)
# The model id arrives via the environment (single-quoted by the shell), never
# spliced into this source. Interpolating it here directly meant a step id like
# `model_x'; import os; os.system('...'); #` executed arbitrary code in WSL.
model_id = os.environ['AIDUBBING_MODEL_ID']
workspace = os.path.expanduser(os.environ['WORKSPACE'])
target_dir = os.path.join(workspace, 'models')
os.makedirs(target_dir, exist_ok=True)

print(f'>>> Začínam overovanie a sťahovanie modelu: {{model_id}}', flush=True)

def fetch_expected_sha256(repo_id, filename):
    try:
        from huggingface_hub import hf_hub_url, get_hf_file_metadata
        url = hf_hub_url(repo_id=repo_id, filename=filename)
        meta = get_hf_file_metadata(url)
        sha = getattr(meta, 'sha256', None)
        if not sha and hasattr(meta, 'etag') and meta.etag:
            candidate = meta.etag.strip('\"')
            if len(candidate) == 64 and all(c in '0123456789abcdefABCDEF' for c in candidate):
                sha = candidate
        return sha
    except Exception as e:
        print(f'[UPOZORNENIE] Nepodarilo sa zistiť SHA256 pre {{repo_id}}/{{filename}}: {{e}}', file=sys.stderr)
        return None

def verify_sha256(filepath, expected_sha):
    if not expected_sha:
        return True
    h = hashlib.sha256()
    with open(filepath, 'rb') as f:
        while True:
            chunk = f.read(65536)
            if not chunk:
                break
            h.update(chunk)
    actual = h.hexdigest()
    if actual.lower() != expected_sha.lower():
        print(f'Nesprávny SHA256 kontrolný súčet pre {{filepath}}: očakávaný {{expected_sha}}, skutočný {{actual}}', file=sys.stderr)
        return False
    return True

def download_file_with_progress(url, dest_path, desc_name, expected_sha=None):
    os.makedirs(os.path.dirname(dest_path), exist_ok=True)
    if os.path.exists(dest_path) and os.path.getsize(dest_path) > 0:
        if verify_sha256(dest_path, expected_sha):
            print(f'✓ Súbor {{desc_name}} už existuje ({{os.path.getsize(dest_path) // 1024}} KB).', flush=True)
            return
        else:
            print(f'Súbor {{desc_name}} je poškodený. Sťahujem znova...', flush=True)
            try:
                os.remove(dest_path)
            except Exception:
                pass

    temp_path = dest_path + '.part'
    print(f'Sťahujem {{desc_name}} z {{url}}...', flush=True)

    # Zrusanie bolo dorobene tak, ze zastavilo len UI. `cancel_wizard_install`
    # nastavuje jeden AtomicBool kontrolovany IBA na zaciatku kroku - nikdy vo
    # vnutri stahovania. Vsetkych 9 krokov teda pokracovalo az do konca, UI si
    # myslel, ze pouzivatel nieco zastavil, a nakoniec vyhlasil "100% HOTOVO".
    # Kontrolujeme preto stav pri kazdom bloku.
    def was_cancelled():
        return os.environ.get('AIDUBBING_CANCELLED') == '1'

    # Pokracovanie po preruseni: predtym sa `.part` otvaralo s 'wb', takze kazdy
    # pokus (aj ten po zruserii) zacinal od nuly a `.part` zostaval na disku.
    already = 0
    headers = {{}}
    if os.path.exists(temp_path):
        already = os.path.getsize(temp_path)
        if already > 0:
            headers['Range'] = f'bytes={{already}}-'
            print(f'Pokracujem od {{already // (1024*1024)}} MB (predchadzajuce stiahnutie).', flush=True)

    r = requests.get(url, stream=True, timeout=60, headers=headers)
    # Server Range nepodporuje (200 namiesto 206) -> musime zacat odznova, inak
    # by sme do suboru pripleli cely subor za uz stiahnutu cast.
    if already > 0 and r.status_code == 200:
        already = 0
        r = requests.get(url, stream=True, timeout=60)

    r.raise_for_status()

    total = int(r.headers.get('content-length', 0)) + already
    downloaded = already
    mode = 'ab' if already > 0 else 'wb'
    with open(temp_path, mode) as f:
        for chunk in r.iter_content(chunk_size=65536):
            if was_cancelled():
                # Zachovame `.part` a hlasi PRERUSENE, nie HOTOVO - dalsi pokus
                # stiahnutie dokonci od tohto miesta.
                total_txt = str(total // (1024*1024)) if total else '?'
                print(
                    f'PRERUSENE pouzivatelom: {{desc_name}} '
                    f'({{downloaded // (1024*1024)}} / {{total_txt}} MB). '
                    f'Pokracovanie pri najblizsom pokuse.',
                    file=sys.stderr, flush=True)
                sys.exit(130)
            if chunk:
                f.write(chunk)
                downloaded += len(chunk)
                if total > 0:
                    pct = (downloaded / total) * 100
                    if downloaded % (512 * 1024) < 65536 or downloaded >= total:
                        print(f'[PROGRESS:{{pct:.1f}}%] {{desc_name}}: {{downloaded // (1024*1024)}} MB / {{total // (1024*1024)}} MB ({{pct:.1f}}%)', flush=True)

    if expected_sha and not verify_sha256(temp_path, expected_sha):
        if os.path.exists(temp_path):
            os.remove(temp_path)
        raise ValueError(f'Chyba integrity: Kontrolný súčet SHA256 pre {{desc_name}} nesedí!')

    os.replace(temp_path, dest_path)
    print(f'✓ Súbor {{desc_name}} úspešne stiahnutý a overený.', flush=True)

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
        whisper_cfg_sha = fetch_expected_sha256('NaiveNeuron/whisper-large-v3-sk', 'config.json')
        download_file_with_progress(
            'https://huggingface.co/NaiveNeuron/whisper-large-v3-sk/resolve/main/config.json',
            os.path.join(asr_dir, 'config.json'),
            'whisper_config.json',
            expected_sha=whisper_cfg_sha
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
        nllb_cfg_sha = fetch_expected_sha256('facebook/nllb-200-distilled-600M', 'config.json')
        download_file_with_progress(
            'https://huggingface.co/facebook/nllb-200-distilled-600M/resolve/main/config.json',
            os.path.join(mt_dir, 'config.json'),
            'nllb_config.json',
            expected_sha=nllb_cfg_sha
        )
    print('✓ NLLB-200 model je pripravený.', flush=True)

elif model_id == 'piper-zh-huayan':
    piper_dir = os.path.join(workspace, 'models/tts/piper')
    os.makedirs(piper_dir, exist_ok=True)
    piper_onnx_sha = fetch_expected_sha256('rhasspy/piper-voices', 'zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx')
    piper_json_sha = fetch_expected_sha256('rhasspy/piper-voices', 'zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx.json')
    download_file_with_progress(
        'https://huggingface.co/rhasspy/piper-voices/resolve/main/zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx',
        os.path.join(piper_dir, 'zh_CN-huayan-medium.onnx'),
        'zh_CN-huayan-medium.onnx (Piper Hlas)',
        expected_sha=piper_onnx_sha
    )
    download_file_with_progress(
        'https://huggingface.co/rhasspy/piper-voices/resolve/main/zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx.json',
        os.path.join(piper_dir, 'zh_CN-huayan-medium.onnx.json'),
        'zh_CN-huayan-medium.onnx.json (Konfigurácia)',
        expected_sha=piper_json_sha
    )
    print('✓ Piper TTS čínsky hlas je stiahnutý a overený.', flush=True)

elif model_id == 'kokoro-v019':
    kokoro_dir = os.path.join(workspace, 'models/tts/kokoro')
    os.makedirs(kokoro_dir, exist_ok=True)
    kokoro_sha = fetch_expected_sha256('hexgrad/Kokoro-82M', 'kokoro-v0_19.onnx')
    download_file_with_progress(
        'https://huggingface.co/hexgrad/Kokoro-82M/resolve/main/kokoro-v0_19.onnx',
        os.path.join(kokoro_dir, 'kokoro-v0_19.onnx'),
        'kokoro-v0_19.onnx (Kokoro TTS)',
        expected_sha=kokoro_sha
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
        coqui_cfg_sha = fetch_expected_sha256('coqui/XTTS-v2', 'config.json')
        download_file_with_progress(
            'https://huggingface.co/coqui/XTTS-v2/resolve/main/config.json',
            os.path.join(coqui_dir, 'config.json'),
            'coqui_xtts_v2_config.json',
            expected_sha=coqui_cfg_sha
        )
    print('✓ Coqui XTTS-v2 checkpoint je pripravený.', flush=True)

elif model_id == 'latentsync-1-5':
    ls_dir = os.path.join(workspace, 'models/lipsync/latentsync')
    os.makedirs(ls_dir, exist_ok=True)
    latentsync_sha = fetch_expected_sha256('ByteDance/LatentSync', 'latentsync_unet.pt')
    download_file_with_progress(
        'https://huggingface.co/ByteDance/LatentSync/resolve/main/latentsync_unet.pt',
        os.path.join(ls_dir, 'latentsync_unet.pt'),
        'latentsync_unet.pt (LatentSync 1.5 UNet)',
        expected_sha=latentsync_sha
    )
    print('✓ LatentSync 1.5 váhy sú pripravené.', flush=True)

elif model_id == 'musetalk-weights':
    mt_dir = os.path.join(workspace, 'models/lipsync/musetalk')
    os.makedirs(mt_dir, exist_ok=True)
    print('Sťahujem TMElyralab/MuseTalk (odľahčený fallback lip-sync model)...', flush=True)
    from huggingface_hub import snapshot_download

    # Stage 5 passes --unet_model_path/--unet_config/--whisper_dir explicitly, so
    # the files must exist at predictable paths instead of the repository's own
    # relative defaults. Download into the workspace, then normalise the layout.
    #
    # The patterns must match the repo layout exactly: TMElyralab/MuseTalk stores
    # these at `musetalk/...` and `musetalkV15/...` with NO `models/` prefix, and
    # ships no whisper weights. The previous patterns prefixed everything with
    # `models/`, matched zero files, and the step failed unconditionally.
    snapshot_download(
        repo_id='TMElyralab/MuseTalk',
        local_dir=mt_dir,
        allow_patterns=[
            'musetalkV15/unet.pth',
            'musetalkV15/musetalk.json',
            'musetalk/*.json',
            'musetalk/*.bin',
        ],
        max_workers=4,
    )

    # MuseTalk v1.5 ships the UNet config as `musetalk.json`; the CLI's
    # `--unet_config` flag expects a path we control.
    cfg_src = os.path.join(mt_dir, 'musetalkV15/musetalk.json')
    cfg_dst = os.path.join(mt_dir, 'config.json')
    if not os.path.exists(cfg_dst) and os.path.exists(cfg_src):
        shutil.copy2(cfg_src, cfg_dst)
        print('✓ MuseTalk UNet konfigurácia sprístupnená ako config.json.', flush=True)

    w_src = os.path.join(mt_dir, 'musetalkV15/unet.pth')
    w_dst = os.path.join(mt_dir, 'unet.pth')
    if not os.path.exists(w_dst) and os.path.exists(w_src):
        shutil.copy2(w_src, w_dst)
    elif not os.path.exists(w_dst):
        # Accept the v1 layout too, in case the snapshot resolves differently.
        alt = os.path.join(mt_dir, 'musetalk/unet.pth')
        if os.path.exists(alt):
            shutil.copy2(alt, w_dst)

    # The MuseTalk repo does not ship whisper weights, so point the flag at the
    # directory the repo clone already created rather than requiring a copy that
    # can never arrive.
    whisper_dst = os.path.join(mt_dir, 'whisper')
    if not os.path.exists(whisper_dst):
        os.makedirs(whisper_dst, exist_ok=True)

    if not os.path.exists(w_dst):
        # Doubled braces are required here: this block lives inside a Rust
        # `format!` literal, so a single-brace placeholder would be consumed by
        # `format!` and emit an empty path into the generated script.
        raise FileNotFoundError(
            f'MuseTalk UNet váhy sa nepodarilo stiahnuť do {{w_dst}}'
        )
    print('✓ MuseTalk váhy sú pripravené.', flush=True)

else:
    # The if/elif chain used to fall through silently, so an unknown model id
    # printed "HOTOVO" and exited 0 and the wizard showed a green check.
    print(f'Neznámy model id: {{model_id}}', file=sys.stderr)
    sys.exit(1)

print('HOTOVO', flush=True)
"
"#,
            venv_setup,
            ws_setup,
            PathMapper::escape_bash_arg(model_id)
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
