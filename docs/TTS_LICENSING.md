# Licenčné Usmernenie pre Text-to-Speech (TTS) Enginy

Pri komerčnom nasadení AI dabingového riešenia je kľúčové dodržať autorské a licenčné podmienky použitých modelov.

---

## 1. Prehľad Podporovaných TTS Modelov

| TTS Engine | Licencia | Komerčné Použitie | Hardvérové Nároky | Podpora Čínštiny |
| :--- | :--- | :--- | :--- | :--- |
| **Piper TTS** | **MIT** |  **Plne povolené** | Ultra-ľahký (CPU / ROCm) |  Áno (`zh_CN-huayan-medium`) |
| **Kokoro TTS** | **Apache 2.0** |  **Plne povolené** | Nízke (~1.5 GB VRAM) |  Áno (Multilingual 82M) |
| **Coqui XTTS-v2**| **CPML** | ❌ **Zakázané pre komerciu** | Stredné (~3.5 GB VRAM) |  Áno (Zero-shot klonovanie) |

---

## 2. Dôležité Upozornenie k Coqui XTTS-v2 (CPML Licencia)
- Pôvodná spoločnosť Coqui AI ukončila svoju činnosť v januári 2024.
- Váhy modelu XTTS-v2 boli vydané pod licenciou **CPML (Coqui Public Model License)**, ktorá výslovne zakazuje akékoľvek komerčné využitie. Keďže spoločnosť zanikla, nie je v súčasnosti možné zakúpiť komerčnú licenciu.
- **Odporúčanie:** Model Coqui XTTS-v2 je v našej aplikácii v GUI označený varovným štítkom a je určený výhradne na interné testovanie a hodnotenie kvality.

---

## 3. Komerčne Bezpečné Alternatívy v Aplikácii

### A. Piper TTS (MIT Licencia — Predvolený Engine)
- Špičková rýchlosť syntézy (až 15x real-time na CPU/GPU).
- Žiadne licenčné obmedzenia pre komerčné produkty alebo predaj dabovaných videí.
- Čínsky model `zh_CN-huayan-medium.onnx` dosahuje vysokú zrozumiteľnosť a nízku latenciu.

### B. Kokoro TTS (Apache 2.0 Licencia)
- Moderný model s 82 miliónmi parametrov.
- Vysoká intonačná vernosť a prirodzený prednes.
- Umožňuje bezstarostné komerčné nasadenie vďaka štandardnej Apache 2.0 licencii.
