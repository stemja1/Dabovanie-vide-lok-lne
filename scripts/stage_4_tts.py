#!/usr/bin/env python3
"""
Fáza 4: Čínska Syntéza Reči (TTS)
Generuje reč pre každú repliku z utterance_metadata.json.
Podporuje:
- Piper TTS (MIT - Komerčne bezpečné)
- Kokoro TTS (Apache 2.0 - Komerčne bezpečné)
- Coqui XTTS-v2 (CPML - Nekomerečné / Testovacie)
Zabezpečuje zarovnanie dĺžky audia s originálnym videom pomocou chained atempo / time-stretching.
"""

import argparse
import os
import sys
import json
import subprocess
import wave
import struct
import math
import gc

def adjust_audio_speed(input_wav: str, output_wav: str, speed_factor: float):
    """
    Adjusts audio speed using ffmpeg atempo filters.
    Chains multiple atempo filters if speed_factor > 2.0 or < 0.5.
    """
    if abs(speed_factor - 1.0) < 0.02:
        if input_wav != output_wav:
            import shutil
            shutil.copy2(input_wav, output_wav)
        return

    filters = []
    remaining = speed_factor
    while remaining > 2.0:
        filters.append("atempo=2.0")
        remaining /= 2.0
    while remaining < 0.5:
        filters.append("atempo=0.5")
        remaining /= 0.5
    filters.append(f"atempo={remaining:.4f}")

    filter_str = ",".join(filters)
    cmd = [
        "ffmpeg", "-y", "-i", input_wav,
        "-filter:a", filter_str,
        "-vn", output_wav
    ]
    subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)

def run_tts(workspace: str, meta_path: str, engine: str, voice: str, global_speed: float, simulate: bool = False):
    print(f"=== Fáza 4: Čínska Syntéza Reči (Engine: {engine}, Voice: {voice}, Rýchlosť: {global_speed:.2f}) ===")
    print("[PROGRESS:5.0%]")

    if not os.path.exists(meta_path):
        print(f"Chýba súbor metadát: {meta_path}", file=sys.stderr)
        sys.exit(1)

    with open(meta_path, "r", encoding="utf-8") as f:
        doc = json.load(f)

    utterances = doc.get("utterances", [])
    abs_workspace = os.path.abspath(workspace)

    audio_segments_dir = os.path.join(abs_workspace, "audio_segments")
    os.makedirs(audio_segments_dir, exist_ok=True)

    master_dubbed_wav = os.path.join(abs_workspace, "audio", "dubbed_speech_track.wav")
    os.makedirs(os.path.dirname(master_dubbed_wav), exist_ok=True)

    total = len(utterances)
    sample_rate = 24000

    print(f"[TTS] Začínam generovanie pre {total} replík...")

    for i, utt in enumerate(utterances):
        utt_id = utt.get("id", f"utt_{i+1:03d}")
        zh_text = utt.get("chinese_text", "").strip()
        target_duration = max(0.2, utt.get("duration", 3.0))
        rel_target = utt.get("target_audio_file", f"audio_segments/{utt_id}.wav")

        # Security check: Prevent path traversal outside workspace
        out_seg_path = os.path.abspath(os.path.join(abs_workspace, rel_target))
        if not out_seg_path.startswith(abs_workspace):
            raise ValueError(f"Bezpečnostná chyba: Cieľový súbor '{rel_target}' uniká mimo workspace!")

        os.makedirs(os.path.dirname(out_seg_path), exist_ok=True)

        if not zh_text:
            zh_text = "本地AI配音测试。"

        speed = utt.get("speed_factor", global_speed)
        print(f"[{i+1}/{total}] Syntetizujem '{utt_id}': {zh_text} (trvanie: {target_duration:.2f}s, speed: {speed:.2f})")

        if simulate:
            # Explicit simulation mode with vocal harmonic synthesis
            num_samples = int(sample_rate * target_duration)
            with wave.open(out_seg_path, "w") as wf:
                wf.setnchannels(1)
                wf.setsampwidth(2)
                wf.setframerate(sample_rate)
                raw_bytes = bytearray()
                for n in range(num_samples):
                    t = float(n) / sample_rate
                    f0 = 220.0 + 25.0 * math.sin(2.0 * math.pi * 1.2 * t)
                    val = 0.5 * math.sin(2.0 * math.pi * f0 * t) + 0.25 * math.sin(2.0 * math.pi * 2.0 * f0 * t)
                    env = min(1.0, (t / 0.04), ((target_duration - t) / 0.04)) if target_duration > 0.08 else 1.0
                    sample_int = int(val * env * 16000)
                    raw_bytes.extend(struct.pack("<h", max(-32768, min(32767, sample_int))))
                wf.writeframes(raw_bytes)
        elif engine == "piper":
            piper_model = os.path.join(abs_workspace, "models/tts/piper/zh_CN-huayan-medium.onnx")
            if not os.path.exists(piper_model):
                raise FileNotFoundError(f"Chýba Piper model na ceste: {piper_model}. Stiahnite ho v Setup Wizarde.")
            
            # Run Piper CLI
            res = subprocess.run(
                ["piper", "--model", piper_model, "--output_file", out_seg_path],
                input=zh_text.encode("utf-8"),
                capture_output=True
            )
            if res.returncode != 0:
                raise RuntimeError(f"Piper TTS zlyhal: {res.stderr.decode('utf-8', errors='replace')}")
            
            if abs(speed - 1.0) >= 0.03:
                temp_speed = out_seg_path + ".tmp.wav"
                adjust_audio_speed(out_seg_path, temp_speed, speed)
                os.replace(temp_speed, out_seg_path)
        elif engine == "kokoro":
            kokoro_model = os.path.join(abs_workspace, "models/tts/kokoro/kokoro-v0_19.onnx")
            if not os.path.exists(kokoro_model):
                raise FileNotFoundError(f"Chýba Kokoro ONNX model na ceste: {kokoro_model}. Stiahnite ho v Setup Wizarde.")
            
            try:
                from kokoro_onnx import Kokoro
                import soundfile as sf
                kokoro = Kokoro(kokoro_model, os.path.join(abs_workspace, "models/tts/kokoro/voices.json"))
                samples, sr = kokoro.create(zh_text, voice=voice, speed=speed, lang="zh")
                sf.write(out_seg_path, samples, sr)
            except Exception as e:
                raise RuntimeError(f"Kokoro TTS generovanie zlyhalo: {e}")
        elif engine == "coqui":
            coqui_dir = os.path.join(abs_workspace, "models/tts/coqui-xtts-v2")
            if not os.path.isdir(coqui_dir):
                raise FileNotFoundError(f"Chýba Coqui XTTS-v2 model na ceste: {coqui_dir}. Stiahnite ho v Setup Wizarde.")
            try:
                from TTS.api import TTS
                tts = TTS(model_path=coqui_dir, config_path=os.path.join(coqui_dir, "config.json"))
                tts.tts_to_file(text=zh_text, file_path=out_seg_path, language="zh-cn", speed=speed)
            except Exception as e:
                raise RuntimeError(f"Coqui XTTS-v2 generovanie zlyhalo: {e}")
        else:
            raise ValueError(f"Neznámy TTS engine: '{engine}'")

        pct = 10.0 + (float(i + 1) / float(total)) * 75.0
        print(f"[PROGRESS:{pct:.1f}%]")

    # Build master synchronized audio track matching original video timeline
    print(f"[TTS] Vytváram zarovnanú zvukovú stopu: {master_dubbed_wav}")
    total_dur = doc.get("total_duration", 30.0)
    total_samples = int(sample_rate * (total_dur + 3.0))
    master_samples = [0] * total_samples

    for utt in utterances:
        st_sample = int(utt.get("start_time", 0.0) * sample_rate)
        seg_file = os.path.abspath(os.path.join(abs_workspace, utt.get("target_audio_file", f"audio_segments/{utt['id']}.wav")))
        if os.path.exists(seg_file):
            with wave.open(seg_file, "r") as wf:
                n_frames = wf.getnframes()
                frames = wf.readframes(n_frames)
                seg_data = struct.unpack(f"<{n_frames}h", frames)
                for idx, sample in enumerate(seg_data):
                    target_idx = st_sample + idx
                    if target_idx < total_samples:
                        cur = master_samples[target_idx] + sample
                        master_samples[target_idx] = max(-32768, min(32767, cur))

    with wave.open(master_dubbed_wav, "w") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(sample_rate)
        out_bytes = bytearray()
        for s in master_samples:
            out_bytes.extend(struct.pack("<h", s))
        wf.writeframes(out_bytes)

    gc.collect()

    print("[PROGRESS:100.0%]")
    print("=== Fáza 4: Syntéza reči úspešne dokončená ===")

def main():
    parser = argparse.ArgumentParser(description="Stage 4: Chinese TTS Synthesis")
    parser.add_argument("--workspace", required=True)
    parser.add_argument("--meta", required=True)
    parser.add_argument("--engine", default="piper")
    parser.add_argument("--voice", default="zh_CN-huayan-medium")
    parser.add_argument("--speed", type=float, default=1.0)
    parser.add_argument("--simulate", action="store_true", default=False)
    args = parser.parse_args()

    try:
        run_tts(args.workspace, args.meta, args.engine, args.voice, args.speed, args.simulate)
    except Exception as e:
        print(f"CHYBA vo Fáze 4 (TTS): {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
