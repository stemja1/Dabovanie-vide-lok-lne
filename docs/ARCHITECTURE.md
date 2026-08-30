# Architektúra AI Dabing Orchestrátora

## 1. Prehľad Systému
Aplikácia slúži ako vysoko výkonný desktopový orchestrátor a grafické rozhranie (GUI) pre lokálny AI dabingový pipeline zo **slovenčiny do čínštiny**.

- **Hostiteľské prostredie:** Windows 11 (Tauri v2 + React 18 / Tailwind CSS).
- **Výkonné ML prostredie:** WSL2 (Ubuntu 24.04 LTS) s akceleráciou AMD ROCm 6.4.2 na grafickej karte AMD Radeon RX 7700 XT (12 GB VRAM) a 16 GB systémovej RAM.
- **Architektonické pravidlo:** Žiadna natívna reimplementácia ML modelov v Ruste (Candle/Burn) — Rust riadi Python subprocessy cez `tokio::process`, streamuje stdout/stderr a synchronizuje dáta cez súborový systém.

---

## 2. Komunikačný Bridge (Windows GUI <-> WSL2)
1. **Vykonávanie príkazov:** Rust backend spúšťa `wsl.exe -d Ubuntu-24.04 -- bash -c "<skript>"`.
2. **Preklad ciest (`path_mapper.rs`):**
   - Windows formát `C:\Videos\input.mp4` sa dynamicky prekladá na `/mnt/c/Videos/input.mp4`.
   - WSL výstupy `/home/ubuntu/workspace/output.mp4` sú prístupné cez Windows UNC cestu `\\wsl.localhost\Ubuntu-24.04\home\ubuntu\workspace\output.mp4`.
3. **Streamovanie logov:** Subprocessy zapisujú výstup na stdout/stderr. Rust reader parsuje značky `[PROGRESS:xx.x%]` a posiela reaktívne JSON udalosti do React UI cez Tauri Event Emitter.

---

## 3. Sekvenčný Pipeline & Riadenie Pamäte (16GB RAM / 12GB VRAM)
Pre zaistenie stability na 16 GB RAM a 12 GB VRAM sa fázy spúšťajú **striktne sekvenčne**. Každý Python proces po dokončení fázy explicitne uvoľní prostriedky a ukončí sa pred spustením nasledujúceho modelu:

```
[Vstupné Video (MP4)]
         │
         ▼
 ┌─────────────────────────────────────────────────────────┐
 │ 1. DEMUX (FFmpeg)                                       │ -> RAM ~0.5 GB | VRAM 0 GB
 └─────────────────────────────────────────────────────────┘
         │
         ▼
 ┌─────────────────────────────────────────────────────────┐
 │ 2. ASR (Whisper-SK / NaiveNeuron)                       │ -> RAM ~4.5 GB | VRAM ~5.5 GB
 └─────────────────────────────────────────────────────────┘
         │
         ▼
 ┌─────────────────────────────────────────────────────────┐
 │ 3. PREKLAD MT (NLLB-200-distilled-600M)                 │ -> RAM ~2.0 GB | VRAM ~2.5 GB
 └─────────────────────────────────────────────────────────┘
         │
         ▼
 ┌─────────────────────────────────────────────────────────┐
 │ 4. INTERAKTÍVNA KONTROLA (GUI Editor Metadát)           │ -> Pozastavenie pipeline
 └─────────────────────────────────────────────────────────┘
         │
         ▼
 ┌─────────────────────────────────────────────────────────┐
 │ 5. TTS SYNTÉZA (Piper MIT / Kokoro Apache 2.0)          │ -> RAM ~0.8 GB | VRAM ~0.5 GB
 └─────────────────────────────────────────────────────────┘
         │
         ▼
 ┌─────────────────────────────────────────────────────────┐
 │ 6. LIP-SYNC (LatentSync 1.5 SDPA / MuseTalk Fallback)    │ -> RAM ~6.0 GB | VRAM ~7.5 GB
 └─────────────────────────────────────────────────────────┘
         │
         ▼
 ┌─────────────────────────────────────────────────────────┐
 │ 7. MUXING & TITULKY (FFmpeg Audio Ducking)              │ -> RAM ~1.0 GB | VRAM 0 GB
 └─────────────────────────────────────────────────────────┘
         │
         ▼
[Výsledné Dabované Video (MP4)]
```

---

## 4. Špecifické Optimalizácie pre AMD ROCm
- **LatentSync 1.5 namiesto 1.6:** Verzia 1.5 bezpečne spotrebuje ~7.5 GB VRAM, zatiaľ čo 1.6 prekračuje limit 12 GB VRAM.
- **ROCm Native SDPA Fallback:** Modul `rocm_attention_patch.py` nahrádza volania xFormers za natívne PyTorch SDPA (`torch.nn.functional.scaled_dot_product_attention`), čím zabraňuje pádom na AMD architektúre.
- **OOM Catch & Auto-Recovery:** Ak LatentSync narazí na `OutOfMemoryError`, orchestrátor chybu zachytí a automaticky prepne na ultra-ľahký model **MuseTalk** (~4.5 GB VRAM).
