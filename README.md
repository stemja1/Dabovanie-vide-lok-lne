# AI Dabing Štúdio — Slovenčina do Čínštiny (SK → ZH)

Desktopová GUI aplikácia napísaná v **Ruste (Tauri v2 + React/TypeScript)**, ktorá orchestruje kompletný lokálny AI dabingový reťazec:

$$\text{Slovenské Video} \longrightarrow \text{Whisper-SK ASR} \longrightarrow \text{NLLB-200 Preklad} \longrightarrow \text{Interaktívny Editor} \longrightarrow \text{Piper/Kokoro TTS} \longrightarrow \text{LatentSync 1.5 / MuseTalk} \longrightarrow \text{Výstupné Video}$$

---

## ⚡ Ako program spustiť (po nainštalovaní základných prerekvizít)

Akonáhle máte jednorazovo nastavené nástroje, pre každodennú prácu si vyberte **jednu z nasledujúcich možností**:

### Spôsob 1: Dvojklik na `start_app.bat` (Najpohodlnejšie)
1. Otvorte priečinok `C:\Dabovanie-vide-lok-lne-main`.
2. Dvakrát kliknite na **`start_app.bat`**.
> 💡 *Tip: Kliknite pravým tlačidlom na `start_app.bat` $\rightarrow$ **Odoslať kam $\rightarrow$ Pracovná plocha (vytvoriť odkaz)** pre spúšťanie priamo z plochy.*

---

### Spôsob 2: Cez bežný PowerShell
Otvorte bežné okno PowerShellu:
```powershell
cd C:\Dabovanie-vide-lok-lne-main
npm run tauri dev
```

---

### Spôsob 3: Vytvorenie samostatného `.exe` súboru (Release)
Ak chcete vyrobiť trvalý `.exe` program pre Windows bez potreby terminálu:
```powershell
npm run tauri build
```
Váš hotový program nájdete v: `src-tauri/target/release/ai-dubbing-orchestrator.exe` a inštalátor v `src-tauri/target/release/bundle/msi/`.

---

## 🚀 Jednorazová inštalácia nástrojov (pre nové PC / prvý štart)

---

## ❓ Riešenie častých situácií pri prvom spúšťaní

1. **Blokovanie skriptov v PowerShell (`ExecutionPolicy`):**
   - **Chyba:** `File ... cannot be loaded because running scripts is disabled on this system.`
   - **Riešenie:** V PowerShell zadajte príkaz:
     ```powershell
     Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
     ```
     Alebo spustite skript s bypassom: `powershell -ExecutionPolicy Bypass -File .\install_prerequisites.ps1`
     Alebo použite `start_app.bat` (dávkové súbory `.bat` systém nikdy neblokuje).

2. **`npm : The term 'npm' is not recognized...`**
   - **Dôvod:** Nástroj `npm` je súčasťou programu **Node.js**. Ak Node.js nie je na počítači nainštalovaný, Windows príkaz `npm` nepozná.
   - **Riešenie:** Spustite `.\install_prerequisites.ps1` alebo nainštalujte Node.js z [nodejs.org](https://nodejs.org).

3. **`rustup : The term 'rustup' is not recognized...`**
   - **Dôvod:** Po inštalácii nového programu do Windows je potrebné zatvoriť a znovu otvoriť okno PowerShellu, aby sa načítali nové systémové cesty (`PATH`).

4. **`The requested operation requires elevation.`**
   - **Dôvod:** Príkaz `wsl --install` vyžaduje práva správcu (Spustiť ako správca / Run as Administrator).

---

## ⚙️ Potrebujem aplikáciu najprv skompilovať?

- **Pre okamžitý beh / testovanie:** Nie! Príkaz `npm run tauri dev` (alebo dvojklik na `start_app.bat`) **automaticky skompiluje Rust backend na pozadí**, spustí Vite a otvorí okno aplikácie.
- **Pre vytvorenie samostatného inštalátora (.msi / .exe):** Spustite `npm run tauri build`. Hotový inštalačný balíček nájdete v `src-tauri/target/release/bundle/msi/`.

---

## 🎯 Hlavné Vlastnosti a Funkcionalita

1. **Rust Orchestrátor (Tauri v2):**
   - Riadi Python subprocessy vo WSL2 bez blokovania UI.
   - Streamovanie výstupu `stdout`/`stderr` s percentuálnym progress barom v reálnom čase.
   - Idempotentná kontrola a správa závislostí.

2. **Idempotentný Setup Wizard:**
   - Automatická detekcia a inštalácia WSL2 + Ubuntu 24.04 s podporou UAC elevácie.
   - Inštalácia systémových balíkov (`ffmpeg`, `git`, `python3-venv`, `libsndfile1`).
   - Nastavenie virtuálneho prostredia s PyTorch ROCm akceleráciou pre grafické karty AMD Radeon.
   - Sťahovanie a verifikácia modelových checkpointov (Whisper-SK, NLLB-200, Piper TTS, LatentSync 1.5, MuseTalk).

3. **Optimalizácia pre 16 GB RAM a 12 GB VRAM (AMD RX 7700 XT):**
   - **Striktne sekvenčný beh:** Každý model sa načíta do pamäte samostatne a po dokončení fázy sa uvoľní.
   - **LatentSync 1.5:** Používa overenú verziu 1.5 (~7.5 GB VRAM) a predchádza preťaženiu pamäte z verzie 1.6.
   - **ROCm Native SDPA Fallback:** Automaticky nahrádza CUDA xFormers za natívne PyTorch SDPA jadro (`rocm_attention_patch.py`).
   - **Inteligentný OOM Fallback:** Pri vyčerpaní pamäte GPU automaticky prepne na odľahčený **MuseTalk** engine (~4.5 GB VRAM).

4. **Komerčne Bezpečné Licencovanie TTS:**
   - Podpora pre **Piper TTS (MIT)** a **Kokoro TTS (Apache 2.0)** vhodné pre komerčné nasadenie.
   - Jasné vizuálne označenie nekomerčných modelov (Coqui XTTS-v2 pod CPML licenciou).

5. **Interaktívny Kontrolný Medzikrok:**
   - Po fázach ASR a prekladu sa pipeline pozastaví a zobrazí editovateľnú tabuľku s replikami, časovými značkami a čínskym textom priamo v GUI.

6. **Prehrávač s Porovnaním a Titulkami:**
   - Náhľad finálneho videa vedľa originálu s možnosťou prepínania slovenských, čínskych alebo dvojjazyčných titulkov.

---

## 💻 Cieľový Hardvér a Prostredie

- **Operačný systém:** Windows 11 (64-bit)
- **Procesor:** AMD Ryzen 5 5600 (alebo ekvivalent)
- **Grafická karta:** AMD Radeon RX 7700 XT (12 GB VRAM)
- **Systémová pamäť:** 16 GB RAM
- **Backendové prostredie:** WSL2 / Ubuntu 24.04 LTS, ROCm 6.4.2 / PyTorch ROCm 6.2+

---

## 📁 Štruktúra Projektu

```
├── install_prerequisites.ps1 # Automatický inštalátor Node.js, Rustu, MSVC a WSL2
├── start_app.bat          # Spúšťač aplikácie jedným klikom
├── src-tauri/             # Rust backend (Tauri v2, WSL runner, orchestrátor, monitory)
│   ├── src/
│   │   ├── config/        # Správa TOML konfigurácie a perzistencia
│   │   ├── wsl/           # WSL bridge, spúšťač procesov, preklad ciest
│   │   ├── wizard/        # Setup Wizard, diagnostika a manifest modelov
│   │   ├── pipeline/      # Orchestrátor fáz, parser metadát, VRAM estimator
│   │   ├── monitor/       # Monitorovanie RAM a VRAM v reálnom čase
│   │   └── commands/      # Tauri IPC príkazy volané z frontendu
├── src/                   # React 18 + Tailwind CSS frontend
│   ├── components/        # Pipeline Štúdio, Editor Metadát, Wizard, Prehrávač, Logy
│   ├── types/             # TypeScript typové definície
│   └── utils/             # Prepojenie na Tauri a formátovače
├── scripts/               # Python CLI skripty pre jednotlivé fázy vo WSL2
│   ├── stage_1_demux.py   # Demuxing a extrakcia zvuku (FFmpeg)
│   ├── stage_2_asr.py     # Slovenský prepis (Whisper-SK / faster-whisper)
│   ├── stage_3_translate.py # Preklad SK -> ZH (NLLB-200)
│   ├── stage_4_tts.py     # Syntéza čínskeho hlasu (Piper / Kokoro / Coqui)
│   ├── stage_5_lipsync.py # Lip-sync (LatentSync 1.5 / MuseTalk)
│   ├── stage_6_mux.py     # Finálny muxing a vpečenie titulkov (FFmpeg)
│   └── rocm_attention_patch.py # PyTorch SDPA optimalizácia pre AMD ROCm
├── docs/                  # Podrobná dokumentácia v slovenčine
│   ├── NAVOD_NA_POUZITIE.html # Vizuálny interaktívny manuál pre prehliadač
│   ├── ARCHITECTURE.md    # Detailná systémová architektúra
│   ├── SETUP_GUIDE_SK.md  # Návod na inštaláciu krok za krokom
│   ├── ROCM_FALLBACK.md   # Sprievodca ROCm SDPA a OOM fallbackom
│   ├── TTS_LICENSING.md   # Licenčné pravidlá (MIT/Apache vs CPML)
│   └── TEST_SCENARIOS.md  # 4 End-to-end testovacie scenáre
└── config.template.toml   # Vzorový konfiguračný súbor
```

---

## 🧪 Spustenie Testov

```bash
cd src-tauri
cargo test
```

Všetkých 23 testov (overenie konfigurácie, preklad UNC ciest, parsovanie utterance metadát, VRAM budgeting a detekcia OOM chýb) sa vykonajú a overia integritu systému.

---

## 📄 Licencia
Projekt je dostupný pod otvorenou licenciou **MIT**. Jednotlivé AI modely podliehajú svojim príslušným licenciám uvedeným v sekcii `docs/TTS_LICENSING.md`.
