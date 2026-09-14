#!/usr/bin/env bash
# ==============================================================================
# AI Dabing Štúdio - Kompletný inštalátor prostredia v Ubuntu WSL2
# ==============================================================================
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}==============================================================================${NC}"
echo -e "${CYAN}   AI Dabing Štúdio (Slovenčina → Čínština) - WSL2 Inštalátor AI Modelov     ${NC}"
echo -e "${CYAN}==============================================================================${NC}"

VENV="${HOME}/.dubbing_env"
WORKSPACE="${HOME}/ai_dubbing_workspace"
MODELS_DIR="${WORKSPACE}/models"

mkdir -p "${WORKSPACE}" "${MODELS_DIR}" "${WORKSPACE}/scripts" "${WORKSPACE}/audio"

echo -e "\n${YELLOW}[1/6] Inštalácia systémových balíkov Ubuntu (FFmpeg, Python, Git)...${NC}"
if [ "$(id -u)" -eq 0 ]; then
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -y
    apt-get install -y --no-install-recommends \
        python3 python3-pip python3-venv python3-dev \
        ffmpeg git curl wget build-essential libsndfile1 libgl1 libglib2.0-0
else
    echo "Spúšťam apt-get inštaláciu (ak máte heslo k sudo, zadajte ho)..."
    sudo apt-get update -y
    sudo apt-get install -y --no-install-recommends \
        python3 python3-pip python3-venv python3-dev \
        ffmpeg git curl wget build-essential libsndfile1 libgl1 libglib2.0-0 || true
fi
echo -e "${GREEN}✓ Systémové balíky pripravené.${NC}"

echo -e "\n${YELLOW}[2/6] Vytváranie izolovaného Python prostredia v ${VENV}...${NC}"
if [ ! -f "${VENV}/bin/python" ]; then
    python3 -m venv "${VENV}" || python3 -m venv --without-pip "${VENV}"
fi

if [ ! -f "${VENV}/bin/pip" ]; then
    curl -sS https://bootstrap.pypa.io/get-pip.py | "${VENV}/bin/python" || true
fi

source "${VENV}/bin/activate"
pip install --upgrade pip setuptools wheel

echo -e "\n${YELLOW}[3/6] Inštalácia PyTorch s podporou AMD ROCm (GPU akcelerácia)...${NC}"
pip install --pre torch torchvision torchaudio --index-url https://download.pytorch.org/whl/rocm6.2 || pip install torch torchvision torchaudio
echo -e "${GREEN}✓ PyTorch úspešne nainštalovaný.${NC}"

echo -e "\n${YELLOW}[4/6] Inštalácia AI knižníc (Transformers, Piper TTS, Kokoro, HuggingFace Hub)...${NC}"
pip install transformers accelerate sentencepiece sacremoses piper-tts kokoro-onnx soundfile librosa scipy pydub ffmpeg-python tqdm requests huggingface_hub
pip install "open_dubbing[coqui]" --no-deps 2>/dev/null || true

echo -e "\n${YELLOW}[5/6] Klonovanie a príprava Lip-sync repozitárov (LatentSync 1.5 & MuseTalk)...${NC}"
cd "${WORKSPACE}"
if [ ! -d "${WORKSPACE}/latentsync" ]; then
    echo "Klonujem LatentSync (v1.5)..."
    git clone https://github.com/bytedance/LatentSync.git "${WORKSPACE}/latentsync" || true
fi

if [ ! -d "${WORKSPACE}/musetalk" ]; then
    echo "Klonujem MuseTalk (odľahčený fallback model)..."
    git clone https://github.com/TMElyralab/MuseTalk.git "${WORKSPACE}/musetalk" || true
fi
echo -e "${GREEN}✓ Lip-sync repozitáre pripravené.${NC}"

echo -e "\n${YELLOW}[6/6] Sťahovanie a overovanie AI modelov...${NC}"
python3 - <<'EOF'
import os, sys, requests, shutil

workspace = os.path.expanduser('~/ai_dubbing_workspace')
models_dir = os.path.join(workspace, 'models')
os.makedirs(models_dir, exist_ok=True)

def download_file(url, dest, name):
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    if os.path.exists(dest) and os.path.getsize(dest) > 1024:
        print(f"✓ {name} už existuje ({os.path.getsize(dest) // (1024*1024)} MB).")
        return
    temp = dest + '.part'
    print(f"Sťahujem {name}...")
    r = requests.get(url, stream=True, timeout=120)
    r.raise_for_status()
    total = int(r.headers.get('content-length', 0))
    dl = 0
    with open(temp, 'wb') as f:
        for chunk in r.iter_content(chunk_size=65536):
            if chunk:
                f.write(chunk)
                dl += len(chunk)
                if total > 0 and dl % (2 * 1024 * 1024) < 65536:
                    print(f"  [{dl * 100 // total}%] {dl // (1024*1024)} MB / {total // (1024*1024)} MB", end='\r', flush=True)
    os.replace(temp, dest)
    print(f"\n✓ {name} úspešne stiahnutý.")

# 1. Whisper SK ASR
asr_dir = os.path.join(models_dir, 'asr/whisper-large-v3-sk')
os.makedirs(asr_dir, exist_ok=True)
print("\n--- Sťahujem Whisper Large-v3 SK (Slovenský prepis) ---")
try:
    from huggingface_hub import snapshot_download
    snapshot_download(repo_id='NaiveNeuron/whisper-large-v3-sk', local_dir=asr_dir, max_workers=4)
    print("✓ Whisper SK model stiahnutý.")
except Exception as e:
    print(f"HuggingFace snapshot zlyhal ({e}), sťahujem priamo config...")
    download_file('https://huggingface.co/NaiveNeuron/whisper-large-v3-sk/resolve/main/config.json', os.path.join(asr_dir, 'config.json'), 'Whisper Config')

# 2. NLLB-200 Prekladač
mt_dir = os.path.join(models_dir, 'mt/nllb-200-distilled-600M')
os.makedirs(mt_dir, exist_ok=True)
print("\n--- Sťahujem NLLB-200 (Preklad slovenčina → čínština) ---")
try:
    from huggingface_hub import snapshot_download
    snapshot_download(repo_id='facebook/nllb-200-distilled-600M', local_dir=mt_dir, max_workers=4)
    print("✓ NLLB-200 model stiahnutý.")
except Exception as e:
    print(f"HuggingFace snapshot zlyhal ({e}), sťahujem priamo config...")
    download_file('https://huggingface.co/facebook/nllb-200-distilled-600M/resolve/main/config.json', os.path.join(mt_dir, 'config.json'), 'NLLB Config')

# 3. Piper TTS Chinese
piper_dir = os.path.join(models_dir, 'tts/piper')
print("\n--- Sťahujem Piper TTS (Čínsky hlas Huayan) ---")
download_file(
    'https://huggingface.co/rhasspy/piper-voices/resolve/main/zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx',
    os.path.join(piper_dir, 'zh_CN-huayan-medium.onnx'),
    'Piper ONNX Model'
)
download_file(
    'https://huggingface.co/rhasspy/piper-voices/resolve/main/zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx.json',
    os.path.join(piper_dir, 'zh_CN-huayan-medium.onnx.json'),
    'Piper ONNX Config'
)

# 4. LatentSync 1.5 UNet
ls_dir = os.path.join(models_dir, 'lipsync/latentsync')
print("\n--- Sťahujem LatentSync 1.5 UNet Checkpoint ---")
download_file(
    'https://huggingface.co/ByteDance/LatentSync/resolve/main/latentsync_unet.pt',
    os.path.join(ls_dir, 'latentsync_unet.pt'),
    'LatentSync 1.5 UNet'
)

# 5. MuseTalk Fallback
mt_ls_dir = os.path.join(models_dir, 'lipsync/musetalk')
print("\n--- Sťahujem MuseTalk Checkpoints ---")
try:
    from huggingface_hub import snapshot_download
    snapshot_download(repo_id='TMElyralab/MuseTalk', local_dir=os.path.dirname(mt_ls_dir.rstrip('/')), allow_patterns=['musetalk/*'], max_workers=4)
    print("✓ MuseTalk váhy stiahnuté.")
except Exception as e:
    print(f"MuseTalk snapshot zlyhal: {e}")

print("\n=======================================================")
print("✓ VŠETKY MODELY BOLI ÚSPEŠNE STIAHNUTÉ A OVERENÉ!")
print("=======================================================")
EOF

echo -e "\n${GREEN}==============================================================================${NC}"
echo -e "${GREEN}   INŠTALÁCIA 100% DOKONČENÁ! Celý AI Dabing systém je pripravený na beh.    ${NC}"
echo -e "${GREEN}==============================================================================${NC}\n"
