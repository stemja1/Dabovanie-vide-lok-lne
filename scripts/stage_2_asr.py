#!/usr/bin/env python3
"""
Fáza 2: Slovenský ASR prepis (Whisper Large-v3 SK)
Využíva model NaiveNeuron/whisper-large-v3-sk s podporou ROCm akcelerácie.
Generuje presné časové značky na úrovni slov a segmentov.
"""

import argparse
import os
import sys
import json
import torch


def _asr_result_to_whisper_format(segments):
    """Normalises backend output into the `{"chunks": [...]}` shape used below.

    Both backends report (start, end, text) triples per segment; the transformers
    path additionally reports per-word timestamps, which we keep when present so
    the metadata editor still has word-level timing for Slovak.
    """
    chunks = []
    for seg in segments:
        # `faster-whisper` yields dataclasses (`Segment`, `Word`), not dicts, so
        # calling `.get()` on them raised AttributeError and killed the stage on
        # the very first segment. Normalise both shapes here.
        if not isinstance(seg, dict):
            seg = {
                "text": getattr(seg, "text", None),
                "start": getattr(seg, "start", None),
                "end": getattr(seg, "end", None),
                "words": getattr(seg, "words", None),
            }
        text = (seg.get("text") or "").strip()
        if not text:
            continue
        start = float(seg.get("start") or 0.0)
        end = float(seg.get("end") or (start + 0.5))
        chunk = {"text": text, "timestamp": (start, end)}
        words = seg.get("words")
        if words:
            normalized = []
            for w in words:
                if not isinstance(w, dict):
                    w = {
                        "word": getattr(w, "word", None) or getattr(w, "text", None),
                        "start": getattr(w, "start", None),
                        "end": getattr(w, "end", None),
                    }
                word_text = (w.get("word") or w.get("text") or "").strip()
                if not word_text:
                    continue
                normalized.append(
                    {
                        "word": word_text,
                        "start": float(w.get("start") if w.get("start") is not None else start),
                        "end": float(w.get("end") if w.get("end") is not None else end),
                    }
                )
            if normalized:
                chunk["words"] = normalized
        chunks.append(chunk)
    return {"chunks": chunks}


def _run_faster_whisper(audio_path, model_to_load, target_device, local_model_path, model_id):
    """Runs the CTranslate2 `faster-whisper` backend."""
    try:
        from faster_whisper import WhisperModel
    except ImportError as e:
        raise RuntimeError(
            "ASR engine 'faster_whisper' vyžaduje balík 'faster-whisper'. "
            "Nainštalujte ho vo venv (`pip install faster-whisper`) "
            "alebo v Settings prepnite engine na Whisper-SK."
        ) from e

    # CTranslate2 needs its own converted weights; only reuse the local folder
    # when it actually contains a CTranslate2 model, otherwise resolve from the
    # HuggingFace id at the correct size ("large-v3" -> "large-v3").
    compute_type = "float16" if target_device.startswith("cuda") else "int8"
    size_hint = None
    if os.path.isdir(local_model_path) and os.path.exists(
        os.path.join(local_model_path, "model.bin")
    ):
        size_hint = local_model_path
    else:
        size_hint = "large-v3"

    print(f"[ASR] Načítavam faster-whisper model ({size_hint}, {compute_type})...")
    try:
        model = WhisperModel(
            size_hint, device=target_device.split(":")[0], compute_type=compute_type
        )
    except Exception:
        # CPU ROCm builds can reject the GPU device; retry on CPU rather than
        # failing the whole stage.
        print("[ASR] GPU inicializácia zlyhala, skúšam CPU fallback...", file=sys.stderr)
        model = WhisperModel(size_hint, device="cpu", compute_type="int8")

    segments, _info = model.transcribe(
        audio_path,
        language="sk",
        task="transcribe",
        word_timestamps=True,
        vad_filter=True,
    )
    return _asr_result_to_whisper_format(list(segments))


def _run_transformers_whisper(audio_path, model_to_load, target_device):
    """Runs the `transformers` Whisper pipeline backend (default)."""
    from transformers import AutoModelForSpeechSeq2Seq, AutoProcessor, pipeline

    print(f"[ASR] Načítavam HuggingFace pipeline z: {model_to_load}...")

    torch_dtype = torch.float16 if target_device.startswith("cuda") else torch.float32
    model = AutoModelForSpeechSeq2Seq.from_pretrained(
        model_to_load,
        torch_dtype=torch_dtype,
        low_cpu_mem_usage=True,
        use_safetensors=True,
    ).to(target_device)

    processor = AutoProcessor.from_pretrained(model_to_load)

    pipe = pipeline(
        "automatic-speech-recognition",
        model=model,
        tokenizer=processor.tokenizer,
        feature_extractor=processor.feature_extractor,
        max_new_tokens=440,
        chunk_length_s=30,
        batch_size=8,
        return_timestamps="word",
        torch_dtype=torch_dtype,
        device=target_device,
    )

    print(f"[ASR] Spúšťam inferenciu slovenskej reči na zvuku: {audio_path}")
    raw = pipe(audio_path, generate_kwargs={"language": "slovak", "task": "transcribe"})

    # The word-level pipeline returns chunks carrying `timestamp` only; the
    # sentence-level grouping below needs a stable (start, end) per chunk.
    return {"chunks": raw.get("chunks", [])}


# Whisper emits punctuation as a separate token ("Dobrý", "deň", ",", "vitaj"),
# so naively joining tokens with a space yields "Dobrý deň , vitaj". Reattach
# punctuation to the preceding token to keep the transcript readable and to give
# the MT model a well-formed sentence.
_NO_SPACE_BEFORE = set(",.!?;:%…)]}»”\"'’")
_NO_SPACE_AFTER = set("([{«“\"'")


def _join_tokens(tokens):
    out = ""
    for tok in tokens:
        if not out:
            out = tok
        elif tok and tok[0] in _NO_SPACE_BEFORE:
            out += tok
        elif out and out[-1] in _NO_SPACE_AFTER:
            out += tok
        else:
            out += " " + tok
    return out.strip()


def _group_chunks_into_utterances(chunks, max_tokens=12):
    """Groups ASR chunks into utterance-sized segments.

    A chunk may be a whole sentence (faster-whisper backend) or a single word
    (transformers word-timestamp backend), so both are handled: we accumulate
    until sentence-final punctuation or the token budget is reached, and record
    word-level timing whenever the backend provided it.
    """
    utterances = []
    current_words = []
    current_tokens = []
    utt_start = None
    utt_idx = 1

    def flush(end_time, confidence):
        nonlocal current_words, current_tokens, utt_start, utt_idx
        text = _join_tokens(current_tokens)
        if not text:
            current_words = []
            current_tokens = []
            utt_start = None
            return
        start = utt_start if utt_start is not None else 0.0
        end = max(end_time, start + 0.2)
        utterances.append({
            "id": f"utt_{utt_idx:03d}",
            "start_time": round(start, 2),
            "end_time": round(end, 2),
            "duration": round(end - start, 2),
            "speaker_id": "SPEAKER_00",
            "slovak_text": text,
            "chinese_text": "",
            "target_audio_file": f"audio_segments/utt_{utt_idx:03d}.wav",
            "speed_factor": 1.0,
            "is_edited": False,
            "confidence": confidence,
            "words": current_words,
        })
        utt_idx += 1
        current_words = []
        current_tokens = []
        utt_start = None

    for chunk in chunks:
        text = (chunk.get("text") or "").strip()
        if not text:
            continue
        ts = chunk.get("timestamp") or (0.0, 0.0)
        start = float(ts[0]) if ts[0] is not None else 0.0
        end = float(ts[1]) if ts[1] is not None else (start + 0.5)

        if utt_start is None:
            # Use the real first-word start instead of 0.0, otherwise every
            # utterance inherited a spurious leading gap and the dub drifted
            # earlier than the original speech.
            utt_start = start

        # Prefer backend-provided per-word timing when available. The
        # transformers word-timestamp pipeline does not populate `words`, so in
        # that case the chunk text itself is the unit to record.
        word_entries = [w for w in (chunk.get("words") or []) if (w.get("word") or "").strip()]
        if word_entries:
            for w in word_entries:
                wtext = w["word"].strip()
                current_words.append({
                    "word": wtext,
                    "start": round(float(w.get("start") or start), 2),
                    "end": round(float(w.get("end") or end), 2),
                    "score": 0.98,
                })
                current_tokens.append(wtext)
        else:
            current_words.append({
                "word": text,
                "start": round(start, 2),
                "end": round(end, 2),
                "score": 0.98,
            })
            current_tokens.append(text)

        if text.endswith((".", "?", "!")) or len(current_tokens) >= max_tokens:
            flush(end, 0.98)

    if current_tokens:
        last_end = current_words[-1]["end"] if current_words else (utt_start or 0.0) + 2.0
        flush(float(last_end), 0.97)

    return utterances


def run_asr(input_video: str, workspace: str, engine: str, device_type: str, model_id: str, simulate: bool = False):
    print(f"=== Fáza 2: Slovenský ASR prepis ({model_id}) ===")
    
    if simulate:
        print("[ASR] Spúšťam explicitný simulačný režim pre testovanie...")
        utterances = [
            {
                "id": "utt_001",
                "start_time": 0.5,
                "end_time": 3.8,
                "duration": 3.3,
                "speaker_id": "SPEAKER_00",
                "slovak_text": "Dobrý deň, vítam vás pri prezentácii nášho nového produktu.",
                "chinese_text": "",
                "target_audio_file": "audio_segments/utt_001.wav",
                "speed_factor": 1.0,
                "is_edited": False,
                "confidence": 0.98,
                "words": []
            },
            {
                "id": "utt_002",
                "start_time": 4.2,
                "end_time": 9.0,
                "duration": 4.8,
                "speaker_id": "SPEAKER_00",
                "slovak_text": "Tento systém využíva pokročilú umelú inteligenciu a beží kompletne lokálne na vašom hardvéri.",
                "chinese_text": "",
                "target_audio_file": "audio_segments/utt_002.wav",
                "speed_factor": 1.0,
                "is_edited": False,
                "confidence": 0.96,
                "words": []
            },
            {
                "id": "utt_003",
                "start_time": 9.6,
                "end_time": 14.8,
                "duration": 5.2,
                "speaker_id": "SPEAKER_00",
                "slovak_text": "Vďaka optimalizácii pre grafické karty AMD Radeon dosahuje vysoký výkon bez odosielania dát na cloud.",
                "chinese_text": "",
                "target_audio_file": "audio_segments/utt_003.wav",
                "speed_factor": 1.0,
                "is_edited": False,
                "confidence": 0.97,
                "words": []
            }
        ]
        raw_meta_path = os.path.join(workspace, "raw_asr_metadata.json")
        with open(raw_meta_path, "w", encoding="utf-8") as f:
            json.dump({
                "video_source": input_video,
                "sample_rate": 16000,
                "source_language": "slk_Latn",
                "utterances": utterances
            }, f, ensure_ascii=False, indent=2)
        print("[PROGRESS:100.0%]")
        print(f"=== Fáza 2: Slovenský ASR úspešne dokončený (simulácia: {len(utterances)} segmentov) ===")
        return

    audio_path = os.path.join(workspace, "audio", "extracted_audio_16k.wav")
    
    if not os.path.exists(audio_path):
        print(f"Zvukový súbor {audio_path} nebol nájdený. Spúšťam núdzovú extrakciu...", file=sys.stderr)
        os.makedirs(os.path.dirname(audio_path), exist_ok=True)
        import subprocess
        subprocess.run(["ffmpeg", "-y", "-i", input_video, "-vn", "-ar", "16000", "-ac", "1", audio_path], check=True)

    target_device = "cuda:0" if (device_type == "rocm" and torch.cuda.is_available()) else "cpu"
    print(f"[ASR] Inicializujem model na zariadení: {target_device} (Torch HIP: {getattr(torch.version, 'hip', 'N/A')})")
    print("[PROGRESS:10.0%]")

    # Check for local workspace model path first
    local_model_path = os.path.join(workspace, "models", "asr", "whisper-large-v3-sk")
    model_to_load = local_model_path if os.path.isdir(local_model_path) else model_id

    # `faster_whisper` is a CTranslate2 runtime with its own model format, so it
    # cannot reuse the transformers `AutoModelForSpeechSeq2Seq` path below. The
    # engine argument used to be accepted and then ignored entirely, which made
    # the Settings dropdown a no-op.
    if engine == "faster_whisper":
        result = _run_faster_whisper(
            audio_path, model_to_load, target_device, local_model_path, model_id
        )
    else:
        result = _run_transformers_whisper(audio_path, model_to_load, target_device)

    print("[PROGRESS:85.0%]")

    utterances = _group_chunks_into_utterances(result.get("chunks", []))

    if not utterances:
        raise RuntimeError(
            "ASR nevrátilo žiadne segmenty. Skontrolujte, či vstupné video obsahuje "
            "rozpoznateľnú reč a či zvolený ASR model podporuje slovenčinu."
        )

    # Save intermediate JSON
    raw_meta_path = os.path.join(workspace, "raw_asr_metadata.json")
    with open(raw_meta_path, "w", encoding="utf-8") as f:
        json.dump({
            "video_source": input_video,
            "sample_rate": 16000,
            "source_language": "slk_Latn",
            "utterances": utterances
        }, f, ensure_ascii=False, indent=2)

    print("[PROGRESS:100.0%]")
    print(f"=== Fáza 2: Slovenský ASR úspešne dokončený ({len(utterances)} segmentov) ===")

def main():
    parser = argparse.ArgumentParser(description="Stage 2: Slovak ASR")
    parser.add_argument("--input", required=True)
    parser.add_argument("--workspace", required=True)
    parser.add_argument("--engine", default="whisper_sk")
    parser.add_argument("--device", default="rocm")
    parser.add_argument("--model", default="NaiveNeuron/whisper-large-v3-sk")
    parser.add_argument("--simulate", action="store_true", default=False)
    args = parser.parse_args()

    try:
        run_asr(args.input, args.workspace, args.engine, args.device, args.model, args.simulate)
    except Exception as e:
        print(f"CHYBA vo Fáze 2 (ASR): {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
