# Oprava dabingového workflow — poznámky

Dátum: 2026-10-02
Branch: `main` (commit `2085c0d`)

GUI fungovalo, ale samotný dabingový reťazec nie. Nižšie je zoznam chýb, ktoré
bránili behu, a čo sa s nimi urobilo. Každá oprava má odkaz na overenie.

---

## 1. `scripts/` sa nikdy neskopíroval do WSL

**Príznak:** fáza 1 končila `can't open file '.../stage_1_demux.py'`.

`orchestrator.rs` aj `installer.rs` hľadali skripty v troch natvrdo zapísaných
cestách:

```
/mnt/c/Dabovanie-vide-lok-lne-main/scripts
/mnt/c/*/Dabovanie-vide-lok-lne*/scripts
/mnt/c/*/*/scripts
```

Tieto globs sedeli len na rozloženie developer's stroja. Inštalácia navyše
preklep chybu potláčala cez `2>/dev/null || true` a `let _ =`, takže sa v UI
neprejavila.

**Oprava:** `scripts/*.py` sú teraz Tauri resource (`tauri.conf.json` →
`bundle.resources`) a `PipelineOrchestrator::ensure_scripts_synced` ich kopíruje
z `resource_dir()`. Chyby sa propagujú a pipeline sa zastaví s konkrétnou hláškou.

Pozor na detail: `resource_dir()` vracia **Windows** cestu, ale príkaz beží
v bashi vo WSL. Cesta sa preto prevedie cez `PathMapper::win_to_wsl` na
`/mnt/<drive>/...`; bez toho by `cp` na Windows ceste zlyhal.

---

## 2. 4. fáza (kontrola metadát) bola nefunkčná

**Príznak:** používateľ upravil čínsky preklad, nič sa nezmenilo.

`UtteranceTable.tsx` volal `get_demo_utterance_metadata` (vždy 3 hardcoded vety)
a ukladal do relatívnej cesty `'utterance_metadata.json'`, teda do CWD procesu.
Fáza 4/5/6 čítajú `{video_dir}/{stem}_utterance_metadata.json` — ten súbor nikto
nečítal ani neupravoval.

**Oprava:** nový príkaz `get_review_utterance_metadata` vracia dokument **plus
cestu**, z ktorej bol načítaný, a `is_real_run`. Editor ukladá späť na tú istú
cestu. Pred spustením TTS sa čaká na úspešné uloženie, inak by TTS bežal proti
neprekladovanému textu.

---

## 3. Koniec dialógu nemal dabing

**Príznak:** posledná veta vo výslednom videu bola ticho.

Fáza 3 zapisovala `total_duration` ako **súčet dĺžok** replík. To ignoruje ticho
medzi vetami, takže hodnota bola kratšia než video. Fáza 4 podľa nej alokovala
buffer pre master stopu a posledná kontrola `target_idx < total_samples` ticho
vyhadzovala všetko, čo doň nezmestilo.

**Oprava:** `total_duration` je teraz najneskôrší `end_time` (zhodné s Rust
`recalculate_timings`). Fáza 4 navyše:

- normalizuje kanály a vzorkovaciu frekvenciu segmentu (Piper 22.05 kHz vs.
  master 24 kHz — predtým to znelo vyššie a pomalšie),
- hlási, ktoré segmenty chýbajú alebo prekročili dĺžku stopy,
- **vyhodí** prázdnu tichú stopu namiesto toho, aby ju pustil ďalej.

Frontend počíta `total_duration` rovnakým spôsobom (predtým súčet, čo
nezodpovedalo backendu).

---

## 4. LatentSync / MuseTalk repozitáre sa nenašli

**Príznak:** `RuntimeError: Modul latentsync ani inferenčný skript neboli nájdené.`

Inštalátor klonoval do `workspace/latentsync` a `workspace/musetalk` (malé
písmená), ale `stage_5_lipsync.py` hľadal `LatentSync/inference.py` a
`MuseTalk/inference.py` (veľké). **Filesystem vo WSL je case-sensitive.**

**Oprava:** klonovanie aj `checker.rs` používajú `LatentSync` / `MuseTalk`.
Diagnostika navyše prestala hlásiť oba repozitáre ako chýbajúce po úspešnej
inštalácii.

---

## 5. `stage_5` volalo neexistujúce API

`from latentsync.pipelines.lipsync_pipeline import LipsyncPipeline` s volaním
`pipeline(video_path=..., audio_path=..., output_path=...)` — taký modul ani
konštrukt v LatentSync 1.5 neexistujú. Vždy to skončilo `ImportError` a prepadlo
do vetvy, ktorá zase nenašla skript (chyba #4).

**Oprava:** volá sa reálne CLI repozitára. Podľa zdrojového kódu upstream:

- **LatentSync** vstupuje cez `scripts/inference.py` (nie `inference.py` v koreni)
  a UNet YAML hľadá v `configs/unet/stage2.yaml`. **Nemá príznak `--batch_size`** —
  jeho poslanie by skončilo argparse chybou, takže sa už neposiela (ostáva
  v logu). Vstupuje sa aj na oboch možných umiestneniach skriptu.
- **MuseTalk** (`scripts/inference.py`) naopak **nemá `--video_path` ani
  `--audio_path`**. Načítava YAML „inference config“, kde každá úloha nesie vlastné
  cesty. Pôvodný kód posielal `--inference_config` s cestou na *checkpoint*, čo je
  úplne iná vec — CLI teda nikdy nevidelo vstup. Teraz sa YAML generuje na
  každý beh s `video_path`/`audio_path`/`result_name`.

Weighty sa už nehľadajú v repozitári (MuseTalk očakáva `./models/...`), ale
ukazujú sa priamo na `models/lipsync/musetalk/`, kam ich sťahuje Setup Wizard.
Ten pôvodne sťahoval `allow_patterns=['musetalk/*']` do zlého priečinka, takže
v závislosti od verzie vznikol buď prázdny adresár, alebo `models/musetalk/...`
a `stage_5` hľadal `musetalk.json` priamo v `models/lipsync/musetalk/`. Teraz
inštalátor sťahuje konkrétne súbory a normalizuje rozloženie na
`unet.pth` / `config.json` / `whisper/`.

Výstup sa presunie na `lipsync_output.mp4`, ktorý očakáva fáza 6. Chýbajúci
výstup je chyba, nie tichý návrat.

---

## 6. Fáza 6 ticho produkovala video bez dabingu

```python
if not os.path.exists(lipsync_video):
    lipsync_video = input_video   # ticho, bez dabingu, bez varovania
```

**Oprava:** chýbajúci lip-sync alebo TTS výstup je teraz `FileNotFoundError` s
návodom, ktorú fázu treba spustiť.

---

## 7. Mux orával výsledné video

`amix=...:duration=longest` v kombinácii s `-shortest` skrátil výstup na
najkratší vstup — často na dabingovú stopu, ktorá je kratšia než video.

**Oprava:** `duration=first` (viazané na pôvodný zvuk) a explicitná normalizácia
formátu hlasovej stopy. Chyba ffmpeg-u sa teraz vrací s chybovým kódom a
výrekom stderr namiesto tichého úspechu bez súboru.

---

## 8. `faster_whisper` engine nefungoval

`--engine` sa prijímal, ale `run_asr` vždy použil `transformers` pipeline.
Prepínač v Settings bol teda neúčinný.

**Oprava:** `_run_faster_whisper` používa CTranslate2 (`language="sk"`,
`word_timestamps=True`, `vad_filter=True`) a `_run_transformers_whisper` pôvodnú
cestu. Výstup oboch sa normalizuje cez `_asr_result_to_whisper_format`, takže
zvyšok fázy je rovnaký. Chýbajúci balík dáva konkrétnu hlášku, nie
`ModuleNotFoundError` z polovice stavby.

---

## Ďalšie zistené chyby (opravené po ceste)

- **SRT timestampy.** `format_timestamp_srt` počítal milisekúndy nezávislo na
  ostatných poliach, takže 3.9999 s dávalo `00:00:03,1000` — neplatný SRT.
  Teraz sa zaokrúhľuje celá hodnota a až potom sa rozkladá. Rovnako opravené
  na frontende (`formatSrtTimestamp`).
- **Interpunkcia v ASR texte.** Whisper tokenizuje `reť` a `!` zvlášť, takže
  text bol `reť !`. Pribudlo `_join_tokens`, ktoré interpunkciu prilepí k
  predchádzajúcemu slovu.
- **Začiatok repliky.** `utt_start` bol natvrdo `0.0`, takže každá replika
  zdedila falošnú úvodnú pauzu. Teraz sa berie z prvého slova.
- **`max_new_tokens=128`** v ASR pipeline bolo príliš nízke na 30 s chunky —
  preklep na 440 (Whisper default).
- **Globálna rýchlosť TTS.** `speed_factor: 1.0` (default z ASR) vždy
  prebila `global_speed` zo Settings. Hodnota 1.0 sa teraz chápe ako
  „neprekonané“ a prepadne na globálne nastavenie.
- **Prázdne metadáta.** Fáza 4 s prázdnym zoznamom vytvorila tichú stopu a pokračovala.
  Teraz to je chyba.
- **Chybajúce kontroly vstupu.** `set_input_video` prijímal prázdnu/neexistujúcu
  cestu a `start_pipeline` sa spustil aj bez videa. Teraz oboje fail-nú s
  konkrétnou hláškou. Kontroluje sa aj UNC cesta, ktorú WSL nespracuje.
- **Ploché timeouty.** Všetky fázy mali 60 min. Teraz `stage_timeout()` — ASR 4 h,
  lip-sync 6 h, MT/TTS 2 h, demux/mux 30 min.
- **Fallback pri chýbajúcich latentsync.config.json.** Kontrola hľadala
  `models/lipsync/latentsync/unet_config.json`, ktorý sa nikdy nestahoval; UNet
  YAML je súčasťou repozitára. Teraz sa používa `configs/unet/stage2.yaml`.
- **ROCm patch zlyhanie.** `rocm_attention_patch.py` importuje `torch` na úrovni
  modulu, takže chýbajúci `torch` spôsobil `ImportError` ešte pred overením,
  či vôbec existujú modely. Fáza 5 teraz patch volá až po svojich kontrolách a
  zlyhanie patchu je varovanie, nie fatálna chyba (je to optimalizácia).
- **Import `torch` vo fáze 5** presunutý na použitie — kontrola „spustil si
  Setup Wizard?“ nesmie skončiť tracebackom z `torch`.
- **Resume po kontrole.** `unwrap_or(4)` pri hľadaní fázy TTS mohol obnoviť
  run na zlom mieste. Teraz sa hľadá podľa ID a chyba sa nepotláča.
- **Fallback po OOM.** Spúšťal sa iba pri diagnostikovanom OOM. Teraz aj pri
  chýbajúcich váhach/chybajúcom balíku/chybe ROCm, a ak zlyhá aj MuseTalk,
  hlásia sa **obe** chyby.
- **Prázdny kód v `installer.rs`.** Po sťahovaní LatentSync váhy bol duplicitný
  úryvok Python kódu, ktorý by spôsobil `SyntaxError` pri behu inštalátora.

---

## Čo sa overilo

**Rust:** `cargo test` — **56 testov prešlo, 0 zlyhaní** (vrátane 6 nových v
`tests/test_review_document_resolution.rs` a 2 nových v `checker.rs`).
`cargo check --tests` bez chýb.

**TypeScript:** `tsc --noEmit` bez chýb, `npm run build` (tsc + vite) exit 0.

**Python:** všetkých 8 skriptov prejde `py_compile`.

**Priebeh od demux po mux na skutočnom videu** (18 s, 640x360, 2 vstupné streamy):

| Fáza | Výsledok |
|---|---|
| 1 Demux | 3 súbory, progress 10/50/80/100 % |
| 3 Preklad | `total_duration = 17.5` (najneskôrší `end_time`, nie súčet 7.8) |
| 4 TTS | stopa 20.5 s @ 24 kHz mono; všetky 3 segmenty na svojich pozíciách |
| 5 Lip-sync | LatentSync aj MuseTalk CLI dostávajú správne argumenty |
| 6 Mux | výstup **18.0 s** = dĺžka vstupu, SRT 3 cue |

Regresný test: po odstránení `lipsync_output.mp4` fáza 6 skončí exit kódom 1 a
**nevytvorí** žiadny výstupný súbor (predtým ticho prepadla na pôvodné video).

Jednotkové testy v pythone pokrývajú: SRT hraničné hodnoty, preskakovanie prázdnych
cue, zdanie slov s/bez slovnej časovej značky, segmentový režim ASR, normalizáciu
a resampling 22.05 → 24 kHz, globálnu rýchlosť TTS, prázdne metadáta, guardy
fáz 5/6.

### Chyby, ktoré odhalilo až `cargo check`

Dve sa vyskytli v **mojich** opravách a boli by prešli bez kompilácie:

1. `installer.rs`: `f'...do {w_dst}'` v bloku, ktorý je súčasťou Rust `format!`
   stringu. `format!` to zan interprets `{w_dst}` ako svoj argument a do skriptu
   sa zapísala prázdna cesta. Treba `{{w_dst}}`.
2. `checker.rs`: môj test kontroloval `include_str!("checker.rs")` na
   výskyt `test -d "$WORKSPACE/latentsync"` — zhoda bola v **samotnom texte
   testu**. Nahradené samostatnou funkciou `build_repos_check_cmd`, ktorá sa dá
   otestovať priamo.

### Argumenty lip-sync CLI boli overené proti upstream zdrojáku

- LatentSync `scripts/inference.py`: `--unet_config_path`, `--inference_ckpt_path`,
  `--video_path`, `--audio_path`, `--video_out_path`, `--inference_steps`,
  `--guidance_scale`, `--temp_dir`, `--seed`, `--enable_deepcache`.
  **`--batch_size` medzi nimi nie je.**
- MuseTalk `scripts/inference.py`: `--ffmpeg_path`, `--gpu_id`, `--vae_type`,
  `--unet_config`, `--unet_model_path`, `--whisper_dir`, `--inference_config`,
  `--bbox_shift`, `--result_dir`, `--extra_margin`, `--fps`,
  `--audio_padding_length_left/right`, `--batch_size`, `--output_vid_name`,
  `--use_saved_coord`, `--saved_coord`, `--use_float16`, `--parsing_mode`,
  `--left_cheek_width`, `--right_cheek_width`, `--version`.
  **`--video_path` a `--audio_path` medzi nimi nie sú.**

---

## Čo sa overiť NEDALO

- **Skutočný ASR / MT / TTS / lip-sync.** Na tomto stroji nie je WSL s ROCm ani
  modely. Fáza 3 prebehla so stubom transformers (aby sa overilo
  `total_duration`, guardy a zápis), TTS a lip-sync v `--simulate`. Pre LatentSync
  aj MuseTalk sa použili falošné CLI (stub, ktorý vypíše argumenty a vyrobí mp4),
  aby sa overilo, že sa volajú správne a výsledok sa presúva na očakávané miesto.
- **Kompilácia Rustu** prešla až po doinštalovaní MSVC toolchainu (Visual Studio
  2022 Build Tools) a po uvoľnení pamäte. Podrobnosti nižšie.
- **Skutočný `resource_dir()`** — overí sa až pri `tauri build`, nie pri `cargo check`.

Pred nasadením odporúčam na čistom stroji spustiť Setup Wizard a prejsť celý
pipeline na jednom krátkom videu so slovenskou rečou.

---

## Prostredie, v ktorom sa to overovalo

Na stroji chýbali `git`, `node`, `cargo` a MSVC. Doinštalované cez `winget`
(Node.js 24.19.0), `rustup-init` (Rust 1.99.0) a Visual Studio 2022 Build Tools
(C:\BuildTools, MSVC 14.44 + Windows SDK 10.0.26100).

`cargo check` prvý raz zlyhal na pamäti, nie na kóde:

```
memory allocation of 14221328 bytes failed
error: could not compile `memchr` (lib)
```

Príčina: `OODefragTray` mal ~15 GB commitu a stránkovací súbor bol na `G:`
(16–24 GB). Po zrušení `OODefragTray` + `oodag` a `cargo check -j 2` prešiel.

Potom to zase zlyhalo na mieste:

```
error: failed to build archive at .../ai_dubbing_lib.lib:
There is not enough space on the disk. (os error 112)
```

`target/` mal 6,9 GB a na `C:` bolo voľných 1 GB. Testy sa preto zostavili s
`CARGO_TARGET_DIR=F:\cargo-target\aidubbing`.

### Test `test_workspace_var_assignment_blocks_command_substitution`

Tento test **nevie zlyhať**, spúšťa totiž `bash` cez `Command::new("bash")`.
Na Windows je `bash` vo WindowsApps **launcher WSL**, nie Git Bash, a tento stroj
nemá nainštalovanú žiadnu distribúciu, takže skript sa nikdy nevykoná. Test
teraz výstup kontroluje a v takomto prostredí sa preskočí s hláškou namiesto
toho, aby zlyhal. Na stroji s WSL distribúciou alebo Git Bashom beží normálne.

### Varovania po `cargo check`

Tri pre-existing varovania o `unused import: std::os::windows::process::CommandExt`
v `installer.rs`, `bridge.rs` a `executor.rs`. Nie súvisia s týmito opravami.
