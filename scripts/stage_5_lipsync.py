#!/usr/bin/env python3
"""
Fáza 5: Lip-sync Face Animation (LatentSync 1.5 / MuseTalk)
Synchronizuje pohyb pier vo videu s vygenerovaným čínskym audiom.
Podporuje:
- LatentSync 1.5 (UNet model ~7.5 GB VRAM) s automatickou ROCm SDPA náhradou
- MuseTalk (~4.5 GB VRAM) ako rýchly a bezpečný fallback
"""

import argparse
import os
import sys
import json
import shutil
import subprocess

# `torch` is imported lazily. The ROCm attention patch and the engines need it,
# but the import is expensive and the error paths below (missing checkpoint,
# missing repo, missing config) do not -- importing it up front meant a plain
# "you have not run the Setup Wizard yet" surfaced as a torch traceback.

try:
    from rocm_attention_patch import apply_rocm_sdpa_patch
except ImportError:
    # The orchestrator runs the stage with the absolute script path, so the
    # sibling module is not always importable. Add the script directory before
    # retrying, otherwise the ROCm optimisation silently degrades to a no-op.
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    try:
        from rocm_attention_patch import apply_rocm_sdpa_patch
    except ImportError:
        def apply_rocm_sdpa_patch():
            print(
                "[UPOZORNENIE] rocm_attention_patch.py sa nenašiel; "
                "SDPA optimalizácia je vypnutá.",
                file=sys.stderr,
            )

def _yaml_quote(value: str) -> str:
    """Quotes a path for a YAML scalar.

    Windows paths contain backslashes and often a drive colon, both of which
    change meaning in unquoted YAML. Single quotes keep them literal; an embedded
    single quote is escaped by doubling it.
    """
    return "'" + str(value).replace("'", "''") + "'"


def run_lipsync(input_video: str, workspace: str, meta_path: str, engine: str, batch_size: int, rocm_sdpa: bool, simulate: bool = False):
    print(f"=== Fáza 5: Lip-sync Animácia ({engine.upper()}) ===")
    print(f"[Lip-sync] Parametre: Batch size = {batch_size}, ROCm SDPA = {rocm_sdpa}, Simulate = {simulate}")
    print("[PROGRESS:5.0%]")

    if rocm_sdpa:
        try:
            apply_rocm_sdpa_patch()
        except Exception as e:
            # The patch is an optimisation, not a requirement. Failing to import
            # it must not abort lip-sync, otherwise a missing `diffusers` turns
            # into an unexplained stage failure.
            print(
                f"[UPOZORNENIE] ROCm SDPA patch sa nepodarilo aplikovať ({e}); "
                "pokračujem bez neho.",
                file=sys.stderr,
            )

    abs_workspace = os.path.abspath(workspace)
    audio_track = os.path.join(abs_workspace, "audio", "dubbed_speech_track.wav")
    lipsync_out_video = os.path.join(abs_workspace, "lipsync_output.mp4")

    if not os.path.exists(audio_track):
        raise FileNotFoundError(f"Chýba vygenerovaná stopa reči: {audio_track}")

    if not os.path.exists(input_video):
        raise FileNotFoundError(f"Chýba vstupné video: {input_video}")

    if os.path.getsize(audio_track) == 0:
        raise RuntimeError(
            f"Stopa reči je prázdna: {audio_track}. Fáza TTS nevygenerovala žiadny zvuk."
        )

    print(f"[Lip-sync] Načítavam video: {input_video} a reč: {audio_track}")
    print("[PROGRESS:20.0%]")

    if simulate:
        print("[Lip-sync] Spúšťam explicitný simulačný režim pre lip-sync...")
        print("[PROGRESS:60.0%]")
        cmd = [
            "ffmpeg", "-y", "-i", input_video, "-i", audio_track,
            "-c:v", "libx264", "-preset", "fast", "-crf", "19",
            "-c:a", "aac", "-b:a", "192k",
            "-map", "0:v:0", "-map", "1:a:0",
            "-shortest",
            lipsync_out_video
        ]
        res = subprocess.run(cmd, capture_output=True)
        if res.returncode != 0:
            raise RuntimeError(f"FFmpeg simulácia zlyhala: {res.stderr.decode('utf-8', errors='replace')}")
        print("[PROGRESS:100.0%]")
        print(f"=== Fáza 5: Lip-sync úspešne dokončený (Simulácia) -> {lipsync_out_video} ===")
        return

    # Production inference paths
    if engine == "latentsync":
        ckpt_path = os.path.join(abs_workspace, "models", "lipsync", "latentsync", "latentsync_unet.pt")
        config_path = os.path.join(abs_workspace, "models", "lipsync", "latentsync", "unet_config.json")

        print(f"[LatentSync 1.5] Kontrolujem model: {ckpt_path}...")
        if not os.path.exists(ckpt_path):
            print(f"[UPOZORNENIE] LatentSync model checkpoint {ckpt_path} nebol nájdený. Stiahnite checkpoint v Setup Wizarde.", file=sys.stderr)
            raise FileNotFoundError(f"Chýba LatentSync 1.5 checkpoint: {ckpt_path}")

        latentsync_repo = os.path.join(abs_workspace, "LatentSync")
        # The entrypoint lives under `scripts/` in the upstream repo. Search both
        # locations so a flat checkout and the standard layout both work.
        latentsync_cli = None
        for candidate in (
            os.path.join(latentsync_repo, "scripts", "inference.py"),
            os.path.join(latentsync_repo, "inference.py"),
        ):
            if os.path.exists(candidate):
                latentsync_cli = candidate
                break
        if latentsync_cli is None:
            raise RuntimeError(
                "LatentSync repozitár sa nenašiel na očakávanej ceste: "
                f"{os.path.join(latentsync_repo, 'scripts', 'inference.py')}. "
                "Spustite Setup Wizard → krok 'Lip-Sync Repozitáre'."
            )

        # The UNet YAML ships with the repo under `configs/unet/`. `stage2.yaml`
        # is the full-quality v1.5 config; `stage2_efficient.yaml` is the
        # lower-VRAM variant. The workspace never contained a copy of the config,
        # so always prefer the repo's own file.
        unet_config = None
        for candidate in (
            config_path,
            os.path.join(latentsync_repo, "configs", "unet", "stage2.yaml"),
            os.path.join(latentsync_repo, "configs", "unet", "stage1.yaml"),
            os.path.join(latentsync_repo, "configs", "unet.yaml"),
        ):
            if candidate and os.path.exists(candidate):
                unet_config = candidate
                break
        if unet_config is None:
            raise RuntimeError(
                "Nenašiel sa UNet konfiguračný YAML pre LatentSync. "
                f"Skontrolujte adresár {os.path.join(latentsync_repo, 'configs', 'unet')}."
            )

        print(f"[LatentSync 1.5] UNet konfigurácia: {unet_config}")
        print("[LatentSync 1.5] Spúšťam UNet inferenciu...")
        print("[PROGRESS:50.0%]")

        # Run the repo's own CLI. Importing the package in-process (the previous
        # `latentsync.pipelines.lipsync_pipeline` path) never worked: that module
        # does not exist in LatentSync 1.5. The upstream CLI also has no
        # `--batch_size` flag, so passing one only produced an argparse error --
        # the batch size is now reported as a log line instead of being sent.
        print(f"[LatentSync 1.5] Nastavený batch size (nemá vplyv na tento CLI): {batch_size}")
        cmd = [
            sys.executable, latentsync_cli,
            "--unet_config_path", unet_config,
            "--inference_ckpt_path", ckpt_path,
            "--video_path", input_video,
            "--audio_path", audio_track,
            "--video_out_path", lipsync_out_video,
        ]
        res = subprocess.run(cmd, cwd=latentsync_repo)
        if res.returncode != 0:
            raise RuntimeError(
                f"LatentSync 1.5 inferencia zlyhala s exit kódom {res.returncode}. "
                f"Použitá konfigurácia: {unet_config}"
            )

    elif engine == "musetalk":
        # MuseTalk resolves its own weights relative to the repo (./models/...),
        # and the ones the Setup Wizard downloads land in
        # `models/lipsync/musetalk/`. Bridge the two by pointing every weight flag
        # at the downloaded files explicitly instead of relying on repo defaults,
        # which would silently fall back to paths that do not exist here.
        musetalk_repo = os.path.join(abs_workspace, "MuseTalk")
        musetalk_cli = None
        for candidate in (
            os.path.join(musetalk_repo, "scripts", "inference.py"),
            os.path.join(musetalk_repo, "inference.py"),
        ):
            if os.path.exists(candidate):
                musetalk_cli = candidate
                break
        if musetalk_cli is None:
            raise RuntimeError(
                "MuseTalk repozitár sa nenašiel na očakávanej ceste: "
                f"{os.path.join(musetalk_repo, 'scripts', 'inference.py')}. "
                "Spustite Setup Wizard → krok 'Lip-Sync Repozitáre'."
            )

        weights_dir = os.path.join(abs_workspace, "models", "lipsync", "musetalk")
        unet_config = os.path.join(weights_dir, "config.json")
        unet_weights = os.path.join(weights_dir, "unet.pth")
        whisper_dir = os.path.join(abs_workspace, "models", "lipsync", "musetalk", "whisper")

        if not os.path.exists(unet_weights):
            raise FileNotFoundError(
                f"Chýbajú MuseTalk váhy UNet: {unet_weights}. "
                "Stiahnite ich v Setup Wizarde (krok 'MuseTalk Weights')."
            )
        if not os.path.exists(unet_config):
            raise FileNotFoundError(
                f"Chýba MuseTalk UNet konfigurácia: {unet_config}. "
                "Stiahnite ju v Setup Wizarde (krok 'MuseTalk Weights')."
            )

        result_dir = os.path.join(abs_workspace, "musetalk_results")
        os.makedirs(result_dir, exist_ok=True)

        # MuseTalk's `inference.py` takes no --video_path/--audio_path flags. It
        # reads a YAML "inference config" where each task carries its own
        # video_path/audio_path, so that file has to be generated per run. The
        # previous code passed --inference_config pointing at a model checkpoint,
        # which is a different thing entirely, so the CLI never saw the input.
        inference_config = os.path.join(result_dir, "inference_config.yaml")
        with open(inference_config, "w", encoding="utf-8") as f:
            f.write("dubbing_task:\n")
            f.write(f"  video_path: {_yaml_quote(input_video)}\n")
            f.write(f"  audio_path: {_yaml_quote(audio_track)}\n")
            f.write(f"  result_name: {_yaml_quote(os.path.basename(lipsync_out_video))}\n")

        print(f"[MuseTalk] UNet váhy: {unet_weights}")
        print(f"[MuseTalk] Úloha (video/audio): {inference_config}")
        print("[MuseTalk] Spúšťam inferenciu...")
        print("[PROGRESS:50.0%]")

        cmd = [
            sys.executable, musetalk_cli,
            "--inference_config", inference_config,
            "--unet_config", unet_config,
            "--unet_model_path", unet_weights,
            "--whisper_dir", whisper_dir,
            "--result_dir", result_dir,
            "--batch_size", str(batch_size),
        ]
        if os.environ.get("MUSE_TALK_FP16") == "1":
            cmd.append("--use_float16")

        res = subprocess.run(cmd, cwd=musetalk_repo)
        if res.returncode != 0:
            raise RuntimeError(
                f"MuseTalk inferencia zlyhala s exit kódom {res.returncode}."
            )

        # MuseTalk may honour --output_vid_name or not depending on version, so
        # accept any freshly written .mp4 in the result dir. Stage 6 expects
        # `lipsync_output.mp4`; without this the earlier code let stage 6 fall
        # back to the undubbed input video.
        produced = sorted(
            (os.path.join(result_dir, f) for f in os.listdir(result_dir) if f.endswith(".mp4")),
            key=os.path.getmtime,
        )
        if not produced:
            raise RuntimeError(
                f"MuseTalk nedal žiadny .mp4 výstup do {result_dir}."
            )
        if os.path.abspath(produced[-1]) != os.path.abspath(lipsync_out_video):
            shutil.move(produced[-1], lipsync_out_video)
    else:
        raise ValueError(f"Neznámy lip-sync engine: '{engine}'")

    if not os.path.exists(lipsync_out_video) or os.path.getsize(lipsync_out_video) == 0:
        raise RuntimeError(
            f"Lip-sync skončil bez výstupného súboru: {lipsync_out_video}"
        )

    print("[PROGRESS:100.0%]")
    print(f"=== Fáza 5: Lip-sync úspešne dokončený -> {lipsync_out_video} ===")

def main():
    parser = argparse.ArgumentParser(description="Stage 5: Lip-Sync Animation")
    parser.add_argument("--input", required=True)
    parser.add_argument("--workspace", required=True)
    parser.add_argument("--meta", required=True)
    parser.add_argument("--engine", default="latentsync")
    parser.add_argument("--batch-size", type=int, default=8)
    parser.add_argument("--rocm-sdpa-fallback", default="1")
    parser.add_argument("--simulate", action="store_true", default=False)
    args = parser.parse_args()

    use_sdpa = args.rocm_sdpa_fallback in ("1", "true", "True")

    try:
        run_lipsync(args.input, args.workspace, args.meta, args.engine, args.batch_size, use_sdpa, args.simulate)
    except Exception as e:
        print(f"CHYBA vo Fáze 5 (Lip-sync): {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
