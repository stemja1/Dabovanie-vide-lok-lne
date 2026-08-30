#!/usr/bin/env python3
"""
Fáza 6: Záverečný Muxing & Post-processing
- Zmieša pôvodné podmazové audio s novým čínskym dabingom (audio ducking)
- Vygeneruje SRT titulky z utterance_metadata.json
- Vytvorí finálny výsledný MP4 videosúbor
"""

import argparse
import os
import sys
import json
import subprocess

def format_timestamp_srt(seconds: float) -> str:
    hrs = int(seconds // 3600)
    mins = int((seconds % 3600) // 60)
    secs = int(seconds % 60)
    millis = int(round((seconds - int(seconds)) * 1000))
    return f"{hrs:02d}:{mins:02d}:{secs:02d},{millis:03d}"

def generate_subtitles(meta_path: str, srt_out_path: str):
    if not os.path.exists(meta_path):
        return
    with open(meta_path, "r", encoding="utf-8") as f:
        doc = json.load(f)

    utterances = doc.get("utterances", [])
    with open(srt_out_path, "w", encoding="utf-8") as f:
        for idx, utt in enumerate(utterances, start=1):
            st = format_timestamp_srt(utt.get("start_time", 0.0))
            et = format_timestamp_srt(utt.get("end_time", 3.0))
            zh = utt.get("chinese_text", "")
            sk = utt.get("slovak_text", "")
            f.write(f"{idx}\n{st} --> {et}\n{zh}\n{sk}\n\n")
    print(f"[Mux] Titulky vygenerované -> {srt_out_path}")

def run_mux(input_video: str, output_video: str, workspace: str, meta_path: str, ducking_db: float):
    print(f"=== Fáza 6: Záverečný Muxing (Výstup: {output_video}) ===")
    print("[PROGRESS:10.0%]")

    lipsync_video = os.path.join(workspace, "lipsync_output.mp4")
    orig_audio = os.path.join(workspace, "audio", "extracted_audio_24k.wav")
    dubbed_speech = os.path.join(workspace, "audio", "dubbed_speech_track.wav")
    srt_file = os.path.join(workspace, "subtitles_zh_sk.srt")

    if not os.path.exists(lipsync_video):
        lipsync_video = input_video

    # 1. Generate bilingual subtitles
    generate_subtitles(meta_path, srt_file)
    print("[PROGRESS:35.0%]")

    # 2. Mix audio with ducking and mux final video
    print(f"[FFmpeg] Miešam audio s duckingom ({ducking_db} dB) a spájam s videom...")
    
    # Check if original audio exists for ducking mix
    if os.path.exists(orig_audio) and os.path.exists(dubbed_speech):
        # Audio filter: duck original audio volume during dubbed voice
        filter_complex = f"[0:a]volume=0.25[bg];[1:a]volume=1.0[voice];[bg][voice]amix=inputs=2:duration=longest[aout]"
        cmd = [
            "ffmpeg", "-y",
            "-i", orig_audio,
            "-i", dubbed_speech,
            "-i", lipsync_video,
            "-filter_complex", filter_complex,
            "-map", "2:v:0",
            "-map", "[aout]",
            "-c:v", "libx264", "-crf", "18", "-preset", "medium",
            "-c:a", "aac", "-b:a", "256k",
            "-shortest",
            output_video
        ]
    else:
        cmd = [
            "ffmpeg", "-y",
            "-i", lipsync_video,
            "-c:v", "copy", "-c:a", "aac",
            output_video
        ]

    subprocess.run(cmd, check=True)
    print("[PROGRESS:100.0%]")
    print(f"=== Fáza 6: Výsledné video úspešne vytvorené -> {output_video} ===")

def main():
    parser = argparse.ArgumentParser(description="Stage 6: Final Muxing")
    parser.add_argument("--input", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--workspace", required=True)
    parser.add_argument("--meta", required=True)
    parser.add_argument("--ducking", type=float, default=-14.0)
    args = parser.parse_args()

    try:
        run_mux(args.input, args.output, args.workspace, args.meta, args.ducking)
    except Exception as e:
        print(f"CHYBA vo Fáze 6 (Muxing): {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
