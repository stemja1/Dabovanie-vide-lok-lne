#!/usr/bin/env python3
"""
Fáza 1: Extrakcia audia a demuxing videa
Extrahuje 16kHz mono WAV a 24kHz master WAV pomocou FFmpeg.
"""

import argparse
import os
import subprocess
import sys
import json

def run_demux(input_video: str, workspace: str):
    print(f"=== Fáza 1: Extrakcia audia a demuxing pre: {input_video} ===")
    os.makedirs(workspace, exist_ok=True)
    audio_dir = os.path.join(workspace, "audio")
    os.makedirs(audio_dir, exist_ok=True)

    wav_16k = os.path.join(audio_dir, "extracted_audio_16k.wav")
    wav_24k = os.path.join(audio_dir, "extracted_audio_24k.wav")
    video_no_audio = os.path.join(workspace, "video_no_audio.mp4")

    print("[PROGRESS:10.0%]")
    print(f"[FFmpeg] Extrahujem 16kHz mono audio pre Whisper ASR -> {wav_16k}")
    cmd_16k = [
        "ffmpeg", "-y", "-i", input_video,
        "-vn", "-acodec", "pcm_s16le", "-ar", "16000", "-ac", "1",
        wav_16k
    ]
    subprocess.run(cmd_16k, check=True)

    print("[PROGRESS:50.0%]")
    print(f"[FFmpeg] Extrahujem 24kHz stereo master audio pre TTS/Mux -> {wav_24k}")
    cmd_24k = [
        "ffmpeg", "-y", "-i", input_video,
        "-vn", "-acodec", "pcm_s16le", "-ar", "24000", "-ac", "2",
        wav_24k
    ]
    subprocess.run(cmd_24k, check=True)

    print("[PROGRESS:80.0%]")
    print(f"[FFmpeg] Izolujem čisté video bez audia -> {video_no_audio}")
    cmd_v = [
        "ffmpeg", "-y", "-i", input_video,
        "-an", "-c:v", "copy",
        video_no_audio
    ]
    subprocess.run(cmd_v, check=True)

    print("[PROGRESS:100.0%]")
    print("=== Fáza 1: Demuxing úspešne dokončený ===")

def main():
    parser = argparse.ArgumentParser(description="Stage 1: Video Demux & Audio Extraction")
    parser.add_argument("--input", required=True, help="Path to input video file")
    parser.add_argument("--workspace", required=True, help="Path to workspace directory")
    args = parser.parse_args()

    try:
        run_demux(args.input, args.workspace)
    except Exception as e:
        print(f"CHYBA vo Fáze 1: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
