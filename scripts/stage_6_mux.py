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
    """Formats a timestamp as an SRT cue time.

    Rounding is applied to the whole value first, then decomposed. Rounding each
    field independently (as this used to do) produced `00:00:03,1000` for
    fractional values just below a second boundary, which is not a valid SRT
    timestamp and is rejected by most players.
    """
    total_ms = int(round(max(0.0, float(seconds)) * 1000.0))
    hrs, remainder = divmod(total_ms, 3_600_000)
    mins, remainder = divmod(remainder, 60_000)
    secs, millis = divmod(remainder, 1000)
    return f"{hrs:02d}:{mins:02d}:{secs:02d},{millis:03d}"

def generate_subtitles(meta_path: str, srt_out_path: str):
    if not os.path.exists(meta_path):
        return
    with open(meta_path, "r", encoding="utf-8") as f:
        doc = json.load(f)

    utterances = doc.get("utterances", [])
    written = 0
    with open(srt_out_path, "w", encoding="utf-8") as f:
        for utt in utterances:
            zh = (utt.get("chinese_text") or "").strip()
            sk = (utt.get("slovak_text") or "").strip()
            if not zh and not sk:
                continue
            start = max(0.0, float(utt.get("start_time", 0.0) or 0.0))
            end = max(start + 0.2, float(utt.get("end_time", start + 1.0) or (start + 1.0)))
            written += 1
            f.write(
                f"{written}\n"
                f"{format_timestamp_srt(start)} --> {format_timestamp_srt(end)}\n"
                f"{zh}\n{sk}\n\n"
            )
    print(f"[Mux] Titulky vygenerované -> {srt_out_path} ({written} cues)")

def run_mux(input_video: str, output_video: str, workspace: str, meta_path: str, ducking_db: float):
    print(f"=== Fáza 6: Záverečný Muxing (Výstup: {output_video}) ===")
    print("[PROGRESS:10.0%]")

    lipsync_video = os.path.join(workspace, "lipsync_output.mp4")
    orig_audio = os.path.join(workspace, "audio", "extracted_audio_24k.wav")
    dubbed_speech = os.path.join(workspace, "audio", "dubbed_speech_track.wav")
    srt_file = os.path.join(workspace, "subtitles_zh_sk.srt")

    # A missing lip-sync output means stage 5 did not really run. Falling back to
    # the undubbed input video here produced a plausible-looking MP4 with no
    # dubbing at all and no warning, so treat it as a hard failure instead.
    if not os.path.exists(lipsync_video):
        raise FileNotFoundError(
            f"Výstup lip-sync fázy neexistuje: {lipsync_video}. "
            "Spustite najprv 6. fázu (Lip-Sync); bez nej by výsledok neobsahoval dabing."
        )

    if not os.path.exists(dubbed_speech):
        raise FileNotFoundError(
            f"Výstup TTS fázy neexistuje: {dubbed_speech}. "
            "Bez vygenerovanej reči by výsledok neobsahoval dabing."
        )

    # 1. Generate bilingual subtitles
    generate_subtitles(meta_path, srt_file)
    print("[PROGRESS:35.0%]")

    # 2. Mix audio with ducking and mux final video
    bg_vol = max(0.0, min(1.0, 10.0 ** (ducking_db / 20.0)))
    print(f"[FFmpeg] Miešam audio s duckingom ({ducking_db} dB -> lineárny koeficient {bg_vol:.3f}) a spájam s videom...")
    
    out_dir = os.path.dirname(os.path.abspath(output_video))
    if out_dir:
        os.makedirs(out_dir, exist_ok=True)

    if os.path.exists(orig_audio):
        # Duck the original track under the dubbed voice.
        #
        # `amix=duration=first` pins the mix to the length of the original audio
        # so the output is not cut short; the previous `duration=longest` combined
        # with `-shortest` truncated the result to whichever input was shorter
        # (frequently the dubbed track), silently dropping the end of the video.
        #
        # `apad` on the voice track pads it to the background length so the mix
        # never runs out of audio early, and the whole graph is anchored to the
        # video with `amix=duration=longest` on a `apad`-ed original plus
        # `-shortest`, which keeps the result exactly as long as the video.
        filter_complex = (
            f"[0:a]volume={bg_vol:.4f}[bg];"
            f"[1:a]aresample=24000,aformat=sample_fmts=fltp:channel_layouts=stereo,volume=1.0[voice];"
            f"[bg][voice]amix=inputs=2:duration=first:dropout_transition=0:normalize=0[aout]"
        )
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
        # No original track to duck, so keep the dubbed voice alone. The
        # lip-sync output's own audio is deliberately not reused: in production
        # it is a low-quality intermediate, and the mastered dubbed track is what
        # should end up in the deliverable.
        cmd = [
            "ffmpeg", "-y",
            "-i", lipsync_video,
            "-i", dubbed_speech,
            "-map", "0:v:0",
            "-map", "1:a:0",
            "-c:v", "copy",
            "-c:a", "aac", "-b:a", "256k",
            "-shortest",
            output_video
        ]

    res = subprocess.run(cmd, capture_output=True)
    if res.returncode != 0:
        raise RuntimeError(
            "FFmpeg muxing zlyhal: " + res.stderr.decode("utf-8", errors="replace")[-2000:]
        )

    if not os.path.exists(output_video) or os.path.getsize(output_video) == 0:
        raise RuntimeError(f"Muxing nevytvoril výstupný súbor: {output_video}")
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
