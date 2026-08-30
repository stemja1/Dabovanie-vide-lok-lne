# ROCm Špecifiká a Mechanizmus Fallbacku (AMD RX 7700 XT)

## 1. Problém s xFormers a FlashAttention-2 na ROCm
Väčšina otvorených modelov pre difúznu tvárovú animáciu (vrátane LatentSync) je primárne optimalizovaná pre NVIDIA CUDA ekosystém s explicitnou závislosťou na `xformers.ops.memory_efficient_attention` alebo `flash_attn`.

Na grafických kartách AMD Radeon (architektúra RDNA3 / ROCm) vedie priame volanie kompilovaných xFormers binárok k chybám ako:
- `ImportError: cannot import name 'memory_efficient_attention' from 'xformers.ops'`
- `HIP error: invalid device function` alebo `no kernel image is available for execution`.

---

## 2. Riešenie: Natívny PyTorch SDPA Patch (`rocm_attention_patch.py`)
V aplikácii je implementovaný modul `scripts/rocm_attention_patch.py`, ktorý pri štarte lip-sync fázy automaticky:
1. Skontroluje prítomnosť ROCm prostredia (`torch.version.hip`).
2. Nastaví globálne backendové prepínače PyTorch:
   ```python
   torch.backends.cuda.enable_flash_sdp(True)
   torch.backends.cuda.enable_mem_efficient_sdp(True)
   torch.backends.cuda.enable_math_sdp(True)
   ```
3. Dynamicky premapuje `XFormersAttnProcessor` v knižnici `diffusers` na natívny `AttnProcessor2_0` (využívajúci `torch.nn.functional.scaled_dot_product_attention`).

Tým sa dosiahne plná hardvérová akcelerácia na AMD Radeon RX 7700 XT bez nutnosti kompilácie nestabilných knižníc tretích strán.

---

## 3. OOM Fallback Mechanizmus: LatentSync 1.5 -> MuseTalk
1. **Prečo LatentSync 1.5 a nie 1.6?**
   - LatentSync 1.5 má pri inferencii nárok na **~6.5 – 8 GB VRAM**, čo sa bezpečne zmestí do 12 GB kapacity RX 7700 XT.
   - LatentSync 1.6 má vyššiu pamäťovú réžiu (>11 GB), čo na 12 GB karte vytvára vysoké riziko OOM (Out Of Memory) pádu.
2. **Automatický Fallback na MuseTalk:**
   - Ak LatentSync z akéhokoľvek dôvodu (napr. vysoké rozlíšenie videa 4K alebo alokácia pamäte iným procesom) zlyhá s chybou `OutOfMemoryError`, Rust orchestrátor túto udalosť okamžite deteguje.
   - Namiesto pádu celej aplikácie sa do GUI odošle notifikácia a automaticky sa spustí odľahčený engine **MuseTalk** (~4.5 GB VRAM), ktorý fázu lip-syncu úspešne dokončí.
