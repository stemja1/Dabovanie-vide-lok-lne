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

# A hung child (piper waiting on stdin, a wedged ffmpeg) used to block the whole
# stage until the 2 h Rust-side budget expired. The wrapper timeout exists, but a
# per-call timeout reports the problem at the right place instead.
TTS_SUBPROCESS_TIMEOUT = 300  # seconds per single TTS/ffmpeg call


def _resolve_inside_workspace(abs_workspace: str, relative: str) -> str:
    """Join `relative` onto the workspace and refuse anything that escapes it.

    `str.startswith` is not a containment check: a sibling directory such as
    `<workspace>_evil` also passes it. `commonpath` compares path components.
    """
    candidate = os.path.abspath(os.path.join(abs_workspace, relative))
    try:
        inside = os.path.commonpath([candidate, abs_workspace]) == abs_workspace
    except ValueError:  # different drives on Windows
        inside = False
    if not inside:
        raise ValueError(
            f"Bezpečnostná chyba: Cieľový súbor '{relative}' uniká mimo workspace!"
        )
    return candidate


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
    subprocess.run(
        cmd,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=True,
        timeout=TTS_SUBPROCESS_TIMEOUT,
    )

def run_tts(workspace: str, meta_path: str, engine: str, voice: str, global_speed: float, simulate: bool = False):
    print(f"=== Fáza 4: Čínska Syntéza Reči (Engine: {engine}, Voice: {voice}, Rýchlosť: {global_speed:.2f}) ===")
    print("[PROGRESS:5.0%]")

    if not os.path.exists(meta_path):
        print(f"Chýba súbor metadát: {meta_path}", file=sys.stderr)
        sys.exit(1)

    with open(meta_path, "r", encoding="utf-8") as f:
        doc = json.load(f)

    utterances = doc.get("utterances", [])
    if not utterances:
        raise RuntimeError(
            "Metadáta neobsahujú žiadne repliky. Spustite najprv fázy 1–3 "
            "(Demux, ASR, Preklad) a uistite sa, že ASR našlo reč vo videu."
        )
    abs_workspace = os.path.abspath(workspace)

    audio_segments_dir = os.path.join(abs_workspace, "audio_segments")
    os.makedirs(audio_segments_dir, exist_ok=True)

    master_dubbed_wav = os.path.join(abs_workspace, "audio", "dubbed_speech_track.wav")
    os.makedirs(os.path.dirname(master_dubbed_wav), exist_ok=True)

    total = len(utterances)
    sample_rate = 24000

    print(f"[TTS] Začínam generovanie pre {total} replík...")

    for i, utt in enumerate(utterances):
        utt_id = utt.get("id") or f"utt_{i+1:03d}"
        zh_text = (utt.get("chinese_text") or "").strip()
        # `duration` comes from a JSON file the UI lets the user edit, so `null`
        # or a string must not crash the whole stage.
        try:
            target_duration = max(0.2, float(utt.get("duration") or 3.0))
        except (TypeError, ValueError):
            target_duration = 3.0
        rel_target = utt.get("target_audio_file") or f"audio_segments/{utt_id}.wav"

        out_seg_path = _resolve_inside_workspace(abs_workspace, rel_target)

        os.makedirs(os.path.dirname(out_seg_path), exist_ok=True)

        if not zh_text:
            zh_text = "本地AI配音测试。"

        # A per-utterance `speed_factor` of 1.0 is the default stored by the ASR
        # stage, so it must not swallow the user's global speed setting. Treat an
        # untouched 1.0 as "not overridden" and fall back to the configured value.
        speed = utt.get("speed_factor", 1.0)
        try:
            speed = float(speed)
        except (TypeError, ValueError):
            speed = 1.0
        if not (0.25 <= speed <= 4.0):
            speed = 1.0
        if abs(speed - 1.0) < 1e-6:
            speed = float(global_speed) if 0.25 <= float(global_speed) <= 4.0 else 1.0
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
            # The voice name from Settings must actually select the model, otherwise
            # the selector silently does nothing on the default engine.
            piper_model = os.path.join(
                abs_workspace, "models/tts/piper", f"{voice}.onnx"
            )
            if not os.path.exists(piper_model):
                fallback = os.path.join(
                    abs_workspace, "models/tts/piper/zh_CN-huayan-medium.onnx"
                )
                if not os.path.exists(fallback):
                    raise FileNotFoundError(
                        f"Chýba Piper model na ceste: {piper_model}. "
                        "Stiahnite ho v Setup Wizarde."
                    )
                piper_model = fallback

            # Invoke Piper as a module of the interpreter running this script.
            # Resolving the bare `piper` console script through PATH fails because
            # the orchestrator never activates the venv, so `$VENV/bin` is not on
            # PATH and the default engine died with FileNotFoundError.
            res = subprocess.run(
                [sys.executable, "-m", "piper", "--model", piper_model,
                 "--output_file", out_seg_path],
                input=zh_text.encode("utf-8"),
                capture_output=True,
                timeout=TTS_SUBPROCESS_TIMEOUT,
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

    # Build master synchronized audio track matching original video timeline.
    #
    # `total_duration` used to be taken straight from the document, but stage 3
    # wrote it as the SUM of segment durations, which ignores the silence between
    # segments and is therefore shorter than the video. The track was sized too
    # small and every utterance starting near the end was silently dropped by the
    # `target_idx < total_samples` guard, so the tail of the dialogue had no Chinese
    # voice at all. Derive the length from the latest end_time instead, matching the
    # Rust `recalculate_timings` logic.
    print(f"[TTS] Vytváram zarovnanú zvukovú stopu: {master_dubbed_wav}")
    timeline_end = max(
        [float(utt.get("end_time", 0.0) or 0.0) for utt in utterances] or [0.0]
    )
    total_dur = max(timeline_end, float(doc.get("total_duration", 0.0) or 0.0))
    total_samples = int(sample_rate * (total_dur + 3.0))

    # The master track used to be a Python `list[int]` filled by a per-sample
    # loop and finally written with `struct.pack(f"<{n}h", *samples)`. A 10-minute
    # video is ~14.5 M samples: that is roughly 0.5 GB of boxed ints, a
    # 14.5 M-argument call, and minutes of pure-Python looping on a machine that
    # also has to hold Whisper/NLLB. numpy is already a hard dependency via
    # onnxruntime, and the stdlib fallback keeps the stage working without it.
    try:
        import numpy as np

        master = np.zeros(total_samples, dtype=np.int32)
        use_numpy = True
    except ImportError:  # pragma: no cover - numpy is normally present
        master = [0] * total_samples
        use_numpy = False

    missing_segments = []
    truncated_segments = []

    for idx_utt, utt in enumerate(utterances):
        utt_id = utt.get("id") or f"utt_{idx_utt + 1:03d}"
        st_sample = int(float(utt.get("start_time", 0.0) or 0.0) * sample_rate)
        # `dict.get(key, default)` evaluates the default eagerly, so the old
        # f-string here raised KeyError for any utterance without an `id` even
        # when `target_audio_file` was present.
        seg_file = _resolve_inside_workspace(
            abs_workspace,
            utt.get("target_audio_file") or f"audio_segments/{utt_id}.wav",
        )
        if not os.path.exists(seg_file):
            missing_segments.append(utt_id)
            continue

        with wave.open(seg_file, "r") as wf:
            # Normalise to the master's format: mono, 16-bit, 24 kHz. TTS
            # backends emit their own rate (Kokoro 24 kHz, Piper 22.05 kHz) and
            # `struct.unpack` assumed mono without checking, so a mismatched
            # segment either raised or was placed at the wrong pitch/length.
            n_channels = wf.getnchannels()
            seg_rate = wf.getframerate() or sample_rate
            sampwidth = wf.getsampwidth()
            frames = wf.readframes(wf.getnframes())

        if sampwidth != 2:
            print(
                f"[TTS] UPOZORNENIE: {utt_id} má {sampwidth * 8}-bit vzorkovanie, preskočujem.",
                file=sys.stderr,
            )
            missing_segments.append(utt_id)
            continue

        if use_numpy:
            seg = np.frombuffer(frames, dtype="<i2").astype(np.int32)
            if n_channels > 1:
                usable = (len(seg) // n_channels) * n_channels
                seg = seg[:usable].reshape(-1, n_channels).mean(axis=1).astype(np.int32)
        else:
            n_samples = len(frames) // (2 * n_channels)
            seg_data = struct.unpack(f"<{n_samples * n_channels}h", frames)
            if n_channels > 1:
                seg_data = tuple(
                    sum(seg_data[i : i + n_channels]) // n_channels
                    for i in range(0, n_samples * n_channels, n_channels)
                )
            seg = list(seg_data)
            n_samples = len(seg)

        # Resample linearne, aby sa segment nestal vyšším/pomalejším.
        if seg_rate != sample_rate:
            ratio = seg_rate / float(sample_rate)
            if use_numpy:
                src_len = len(seg)
                out_len = max(1, int(src_len / ratio))
                pos = np.arange(out_len, dtype=np.float64) * ratio
                i0 = np.clip(pos.astype(np.int64), 0, src_len - 1)
                i1 = np.clip(i0 + 1, 0, src_len - 1)
                frac = pos - i0
                seg = (seg[i0] * (1.0 - frac) + seg[i1] * frac).astype(np.int32)
            else:
                out_len = max(1, int(n_samples / ratio))
                resampled = [0] * out_len
                for i in range(out_len):
                    src = i * ratio
                    i0 = int(src)
                    i1 = min(i0 + 1, n_samples - 1)
                    frac = src - i0
                    resampled[i] = int(seg[i0] * (1.0 - frac) + seg[i1] * frac)
                seg = resampled

        if st_sample >= total_samples:
            truncated_segments.append(utt_id)
            continue

        room = total_samples - st_sample
        if len(seg) > room:
            truncated_segments.append(utt_id)
            seg = seg[:room]

        if use_numpy:
            master[st_sample : st_sample + len(seg)] += seg
        else:
            for idx, sample in enumerate(seg):
                target_idx = st_sample + idx
                cur = master[target_idx] + sample
                master[target_idx] = max(-32768, min(32767, cur))

    if missing_segments:
        print(
            f"[TTS] UPOZORNENIE: chýbajúce audio segmente ({len(missing_segments)}): "
            f"{', '.join(missing_segments[:10])}",
            file=sys.stderr,
        )
    if truncated_segments:
        print(
            f"[TTS] UPOZORNENIE: segmenty prekročili dĺžku stopy ({len(truncated_segments)}): "
            f"{', '.join(truncated_segments[:10])}",
            file=sys.stderr,
        )

    # Clip once, at the end: summing int32 segments can exceed the int16 range
    # exactly where two replikas overlap.
    if use_numpy:
        pcm = np.clip(master, -32768, 32767).astype("<i2").tobytes()
        is_silent = not bool(np.any(master))
    else:
        pcm = struct.pack(f"<{len(master)}h", *master)
        is_silent = not any(master)

    # The silence check used to require `missing_segments` and
    # `truncated_segments` to be empty, which is precisely the case where every
    # TTS segment failed: the guard was skipped, a silent 24 kHz WAV was written
    # and the stage exited 0, so lip-sync and mux produced a plausible video
    # with no dubbing at all. Silence is always an error here.
    if is_silent:
        raise RuntimeError(
            "TTS stavila zarovnanú stopu, ale tá je úplne tichá. "
            "Skontrolujte vygenerované TTS segmenty a nastavenia hlasu."
        )

    with wave.open(master_dubbed_wav, "w") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(sample_rate)
        wf.writeframes(pcm)

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
