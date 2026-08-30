#!/usr/bin/env python3
"""
Master CLI pre AI Dabing Pipeline (SK -> ZH)
Umožňuje sekvenčné spustenie celého reťazca:
Demux -> ASR (Whisper-SK) -> MT (NLLB-200) -> TTS (Piper/Kokoro/Coqui) -> LipSync (LatentSync 1.5 / MuseTalk) -> Muxing
"""

import argparse
import os
import subprocess
import sys

def main():
    parser = argparse.ArgumentParser(description="Master Orchestrator CLI for Local AI Dubbing (SK -> ZH)")
    parser.add_argument("--input", required=True, help="Path to input Slovak video file")
    parser.add_argument("--workspace", default="/home/user/ai_dubbing_workspace", help="Workspace folder")
    parser.add_argument("--asr-engine", default="whisper_sk", choices=["whisper_sk", "faster_whisper"])
    parser.add_argument("--tts-engine", default="piper", choices=["piper", "kokoro", "coqui"])
    parser.add_argument("--lipsync-engine", default="latentsync", choices=["latentsync", "musetalk"])
    parser.add_argument("--rocm-sdpa", default="1", help="Enable ROCm native SDPA fallback")
    args = parser.parse_args()

    script_dir = os.path.dirname(os.path.abspath(__file__))
    input_video = os.path.abspath(args.input)
    workspace = os.path.abspath(args.workspace)
    os.makedirs(workspace, exist_ok=True)

    meta_file = os.path.join(workspace, "utterance_metadata.json")
    output_video = os.path.join(workspace, "final_dubbed_zh.mp4")

    print("==========================================================")
    print("  AI Dabing Studio — Lokálny Pipeline: Slovenčina -> Čínština")
    print(f"  Vstup: {input_video}")
    print(f"  Výstup: {output_video}")
    print("==========================================================")

    # 1. Demux
    subprocess.run([sys.executable, os.path.join(script_dir, "stage_1_demux.py"), "--input", input_video, "--workspace", workspace], check=True)

    # 2. ASR
    subprocess.run([sys.executable, os.path.join(script_dir, "stage_2_asr.py"), "--input", input_video, "--workspace", workspace, "--engine", args.asr_engine], check=True)

    # 3. Translate
    subprocess.run([sys.executable, os.path.join(script_dir, "stage_3_translate.py"), "--workspace", workspace, "--meta", meta_file], check=True)

    # 4. TTS
    subprocess.run([sys.executable, os.path.join(script_dir, "stage_4_tts.py"), "--workspace", workspace, "--meta", meta_file, "--engine", args.tts_engine], check=True)

    # 5. Lip-sync
    subprocess.run([sys.executable, os.path.join(script_dir, "stage_5_lipsync.py"), "--input", input_video, "--workspace", workspace, "--meta", meta_file, "--engine", args.lipsync_engine, "--rocm-sdpa-fallback", args.rocm_sdpa], check=True)

    # 6. Mux
    subprocess.run([sys.executable, os.path.join(script_dir, "stage_6_mux.py"), "--input", input_video, "--output", output_video, "--workspace", workspace, "--meta", meta_file], check=True)

    print("Pipeline úspešne dokončený!")

if __name__ == "__main__":
    main()
