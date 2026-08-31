# End-to-End Testovacie Scenáre

Tento dokument opisuje 4 kompletné end-to-end testovacie scenáre pre verifikáciu funkčnosti AI Dabing Štúdia.

---

## Scenár 1: Čistá Inštalácia od Nuly cez Setup Wizard
- **Cieľ:** Overiť, že aplikácia na čistom Windows 11 systéme bez predchádzajúceho nastavenia vie idempotentne skontrolovať a doinštalovať celý reťazec závislostí.
- **Postup:**
  1. Spustite aplikáciu AI Dabing Štúdio.
  2. Otvorte záložku **Setup Sprievodca**.
  3. Kliknite na **"Automaticky nainštalovať všetko"**.
  4. Sledujte priebeh v live termináli.
  5. Overte, že po dokončení svietia všetky kontrolné body na zeleno (WSL2, Distro, Balíčky, PyTorch ROCm, Repozitáre, Modelové váhy).
- **Očakávaný výsledok:** Celkový stav pripravenosti dosiahne **100%**, diagnostický banner sa sfarbí do zelena bez zlyhania krokov.

---

## Scenár 2: Štandardný Priebeh Lokálneho AI Dabingu (SK -> ZH)
- **Cieľ:** Overiť kompletnú sekvenčnú transformáciu slovenského videa do čínštiny.
- **Postup:**
  1. Na záložke **Pipeline Štúdio** vyberte vstupné slovenské video (napr. `prezentacia.mp4`).
  2. Overte, že VRAM meter indikuje bezpečné zaťaženie pre RX 7700 XT (~7.5 GB max).
  3. Kliknite na **"Spustiť AI Dabing"**.
  4. Fáza 1 (Demux) extrahuje audio.
  5. Fáza 2 (ASR Whisper-SK) prepíše slovenský text so značkami slov.
  6. Fáza 3 (NLLB-200) preloží text do čínštiny.
  7. Pipeline sa automaticky pozastaví vo Fáze 4 na kontrolu.
  8. Používateľ potvrdí metadáta a pokračuje v syntéze reči (Piper TTS) a lip-syncu (LatentSync 1.5).
  9. Fáza 7 zmieša podmaz s dabingom a vytvorí `final_dubbed_zh.mp4`.
- **Očakávaný výsledok:** Vygeneruje sa finálne MP4 video s plynulým čínskym dabingom a presným pohybom pier.

---

## Scenár 3: Simulovaný OOM a Nútený Fallback na MuseTalk
- **Cieľ:** Overiť, že pri zlyhaní LatentSync na nedostatok VRAM aplikácia nespadne, ale automaticky aktivuje MuseTalk.
- **Postup:**
  1. Nastavte v `config.toml` extrémne veľký `lipsync_batch_size = 64` alebo simulujte OOM udalosť.
  2. Spustite fázu Lip-sync.
  3. LatentSync vyvolá výnimku `torch.cuda.OutOfMemoryError: HIP out of memory`.
  4. Rust orchestrátor zachytí OOM chybový kód a zaloguje varovanie:
     `[WARN] LatentSync zlyhal na OOM. Automaticky aktivujem záchranný fallback: MuseTalk Engine...`
  5. Fáza 5 sa automaticky reštartuje s enginom `musetalk`.
- **Očakávaný výsledok:** Fáza Lip-sync sa úspešne dokončí bez pádu aplikácie a v UI sa zobrazí štítok `MuseTalk (OOM Fallback)`.

---

## Scenár 4: Interaktívna Kontrola a Úprava Metadát v GUI
- **Cieľ:** Overiť možnosť úpravy čínskeho prekladu a časovania priamo v interaktívnej tabuľke pred syntézou reči.
- **Postup:**
  1. Po dokončení fázy 3 (Preklad) sa v aplikácii automaticky aktivuje záložka **Editor Metadát & Prekladu**.
  2. Používateľ upraví text repliky `utt_001` v stĺpci čínštiny.
  3. Upraví rýchlosť syntézy na `1.10x`.
  4. Klikne na tlačidlo **"Uložiť zmeny"** a **"Potvrdiť a spustiť TTS"**.
- **Očakávaný výsledok:** Súbor `utterance_metadata.json` sa aktualizuje a nasledujúca fáza TTS vygeneruje audio s upraveným textom a rýchlosťou.
