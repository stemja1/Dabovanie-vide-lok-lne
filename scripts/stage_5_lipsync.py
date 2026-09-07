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
import subprocess
import torch

try:
    from rocm_attention_patch import apply_rocm_sdpa_patch
except ImportError:
    # Try local directory import
    sys.path.append(os.path.dirname(__file__))
    try:
        from rocm_attention_patch import apply_rocm_sdpa_patch
    except ImportError:
        def apply_rocm_sdpa_patch():
            pass

def run_lipsync(input_video: str, workspace: str, meta_path: str, engine: str, batch_size: int, rocm_sdpa: bool, simulate: bool = False):
    print(f"=== Fáza 5: Lip-sync Animácia ({engine.upper()}) ===")
    print(f"[Lip-sync] Parametre: Batch size = {batch_size}, ROCm SDPA = {rocm_sdpa}, Simulate = {simulate}")
    print("[PROGRESS:5.0%]")

    if rocm_sdpa:
        apply_rocm_sdpa_patch()

    abs_workspace = os.path.abspath(workspace)
    audio_track = os.path.join(abs_workspace, "audio", "dubbed_speech_track.wav")
    lipsync_out_video = os.path.join(abs_workspace, "lipsync_output.mp4")

    if not os.path.exists(audio_track):
        raise FileNotFoundError(f"Chýba vygenerovaná stopa reči: {audio_track}")

    if not os.path.exists(input_video):
        raise FileNotFoundError(f"Chýba vstupné video: {input_video}")

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

        print("[LatentSync 1.5] Spúšťam UNet inferenciu s dávkou (batch_size={batch_size})...")
        print("[PROGRESS:50.0%]")
        
        # Check if latentsync python package / submodule is available
        try:
            from latentsync.pipelines.lipsync_pipeline import LipsyncPipeline
            pipeline = LipsyncPipeline.from_pretrained(os.path.dirname(ckpt_path), torch_dtype=torch.float16)
            pipeline.to("cuda:0" if torch.cuda.is_available() else "cpu")
            pipeline(video_path=input_video, audio_path=audio_track, output_path=lipsync_out_video, batch_size=batch_size)
        except ImportError:
            # Fallback to CLI invocation if LatentSync script exists
            latentsync_script = os.path.join(abs_workspace, "LatentSync", "inference.py")
            if os.path.exists(latentsync_script):
                cmd = [
                    sys.executable, latentsync_script,
                    "--unet_config_path", config_path,
                    "--inference_ckpt_path", ckpt_path,
                    "--video_path", input_video,
                    "--audio_path", audio_track,
                    "--video_out_path", lipsync_out_video
                ]
                res = subprocess.run(cmd, check=True)
            else:
                raise RuntimeError("Modul latentsync ani inferenčný skript neboli nájdené.")

    elif engine == "musetalk":
        musetalk_ckpt = os.path.join(abs_workspace, "models", "lipsync", "musetalk", "musetalk.json")
        print(f"[MuseTalk] Kontrolujem checkpoint: {musetalk_ckpt}...")
        if not os.path.exists(musetalk_ckpt):
            raise FileNotFoundError(f"Chýba MuseTalk checkpoint na ceste: {musetalk_ckpt}")
        print("[MuseTalk] Spúšťam inferenciu...")
        print("[PROGRESS:50.0%]")
        # MuseTalk runner
        musetalk_script = os.path.join(abs_workspace, "MuseTalk", "inference.py")
        if os.path.exists(musetalk_script):
            cmd = [
                sys.executable, musetalk_script,
                "--inference_config", musetalk_ckpt,
                "--video_path", input_video,
                "--audio_path", audio_track,
                "--result_dir", os.path.dirname(lipsync_out_video)
            ]
            subprocess.run(cmd, check=True)
        else:
            raise RuntimeError("Modul musetalk ani inferenčný skript neboli nájdené.")
    else:
        raise ValueError(f"Neznámy lip-sync engine: '{engine}'")

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
