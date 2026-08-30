# Návod na Inštaláciu a Prípravu Prostredia (Slovenčina)

Tento návod popisuje kompletný postup inštalácie a overenia systému pre lokálny AI dabing zo slovenčiny do čínštiny na zostave s **Windows 11, AMD Ryzen 5 5600 a AMD Radeon RX 7700 XT (12 GB VRAM)**.

---

## 1. Automatická Inštalácia cez Setup Wizard v GUI (Odporúčané)
Pri prvom spustení desktopovej aplikácie sa automaticky otvorí **Setup Sprievodca**:
1. Prejdite na záložku **Setup Sprievodca**.
2. Kliknite na tlačidlo **"Automaticky nainštalovať všetko"**.
3. Sprievodca postupne a idempotentne overí a vykoná:
   - Kontrolu a inštaláciu WSL2 + Ubuntu 24.04 (vyžiada si UAC povolenie).
   - Inštaláciu systémových balíkov (`ffmpeg`, `git`, `python3-venv`, `libsndfile1`).
   - Vytvorenie Python virtuálneho prostredia v `~/.dubbing_env`.
   - Inštaláciu PyTorch s podporou ROCm (`--index-url https://download.pytorch.org/whl/rocm6.2`).
   - Klonovanie a prípravu repozitárov LatentSync 1.5 a MuseTalk.
   - Stiahnutie modelových váh (Whisper-SK, NLLB-200, Piper TTS, LatentSync checkpoint).
4. Po dokončení zobrazí prehľadný diagnostický report.

---

## 2. Manuálna Inštalácia (v prípade potreby)

### Krok 2.1: Inštalácia WSL2 vo Windows 11
Otvorte **PowerShell ako Administrátor** a spustite:
```powershell
wsl --install -d Ubuntu-24.04
```
*Poznámka: Ak ide o prvú inštaláciu WSL2, reštartujte počítač.*

### Krok 2.2: Systémové Balíčky v Ubuntu 24.04
Otvorte Ubuntu terminál vo WSL2:
```bash
sudo apt-get update && sudo apt-get upgrade -y
sudo apt-get install -y --no-install-recommends \
    python3 python3-pip python3-venv python3-dev \
    ffmpeg git curl wget build-essential libsndfile1 libgl1 libglib2.0-0
```

### Krok 2.3: Vytvorenie Virtuálneho Prostredia a PyTorch ROCm
```bash
mkdir -p ~/.dubbing_env ~/ai_dubbing_workspace
python3 -m venv ~/.dubbing_env
source ~/.dubbing_env/bin/activate

# Inštalácia PyTorch ROCm
pip install --upgrade pip setuptools wheel
pip install --pre torch torchvision torchaudio --index-url https://download.pytorch.org/whl/rocm6.2

# Inštalácia dabingových knižníc
pip install transformers accelerate sentencepiece sacremoses piper-tts kokoro-onnx soundfile librosa scipy pydub ffmpeg-python tqdm requests
```

### Krok 2.4: Príprava Lip-Sync Repozitárov
```bash
cd ~/ai_dubbing_workspace

# LatentSync 1.5
git clone https://github.com/bytedance/LatentSync.git latentsync
cd latentsync
pip install -r requirements.txt || true
pip install diffusers omegaconf einops decord face-alignment mediapipe

# MuseTalk (Záložný fallback)
cd ~/ai_dubbing_workspace
git clone https://github.com/TMElyralab/MuseTalk.git musetalk
cd musetalk
pip install -r requirements.txt || true
```

---

## 3. Spustenie Rust GUI Aplikácie
1. Uistite sa, že máte nainštalovaný Node.js (v18+) a Rust toolchain (`rustup`).
2. Spustite vývojový režim:
   ```bash
   npm run dev
   ```
   alebo zostavte produkčný Windows inštalátor cez Tauri CLI:
   ```bash
   cargo tauri build
   ```
