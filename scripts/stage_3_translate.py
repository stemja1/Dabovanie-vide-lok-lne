#!/usr/bin/env python3
"""
Fáza 3: Preklad zo slovenčiny do čínštiny (NLLB-200)
Prekladá slk_Latn -> zho_Hans pomocou facebook/nllb-200-distilled-600M.
Vytvára kompletný utterance_metadata.json pripravený na kontrolu v GUI.
"""

import argparse
import os
import sys
import json
import torch

def run_translation(workspace: str, meta_path: str, model_id: str, src_lang: str, tgt_lang: str, simulate: bool = False):
    print(f"=== Fáza 3: Preklad SK → ZH ({model_id}) ===")
    print("[PROGRESS:10.0%]")

    raw_meta_path = os.path.join(workspace, "raw_asr_metadata.json")
    if not os.path.exists(raw_meta_path):
        print(f"Chýba súbor {raw_meta_path}", file=sys.stderr)
        sys.exit(1)

    with open(raw_meta_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    utterances = data.get("utterances", [])
    total = len(utterances)

    if simulate:
        print("[Preklad] Spúšťam explicitný simulačný režim pre preklad...")
        sk_zh_demo = {
            "Dobrý deň, vítam vás pri prezentácii nášho nového produktu.": "您好，欢迎来到我们新产品的展示会。",
            "Tento systém využíva pokročilú umelú inteligenciu a beží kompletne lokálne na vašom hardvéri.": "该系统利用先进的人工智能，并完全在您的本地硬件上运行。",
            "Vďaka optimalizácii pre grafické karty AMD Radeon dosahuje vysoký výkon bez odosielania dát na cloud.": "由于针对AMD Radeon显卡进行了优化，无需将数据发送到云端即可实现高性能。"
        }
        for i, utt in enumerate(utterances):
            sk_text = utt.get("slovak_text", "")
            utt["chinese_text"] = sk_zh_demo.get(sk_text, "本地AI视频配音：从斯洛伐克语到中文。")
    elif total > 0:
        device = "cuda:0" if torch.cuda.is_available() else "cpu"
        print(f"[Preklad] Inicializujem NLLB model na zariadení: {device}")

        local_model_path = os.path.join(workspace, "models", "mt", "nllb-200-distilled-600M")
        model_to_load = local_model_path if os.path.isdir(local_model_path) else model_id

        from transformers import AutoModelForSeq2SeqLM, AutoTokenizer
        print(f"[Preklad] Načítavam HuggingFace tokenizer a model z: {model_to_load}...")
        tokenizer = AutoTokenizer.from_pretrained(model_to_load, src_lang=src_lang)
        model = AutoModelForSeq2SeqLM.from_pretrained(model_to_load).to(device)
        model.eval()

        for i, utt in enumerate(utterances):
            sk_text = utt.get("slovak_text", "").strip()
            if not sk_text:
                utt["chinese_text"] = ""
                continue

            inputs = tokenizer(sk_text, return_tensors="pt").to(device)
            forced_bos_token_id = tokenizer.lang_code_to_id.get(tgt_lang)
            
            with torch.no_grad():
                gen_kwargs = {"max_length": 128}
                if forced_bos_token_id is not None:
                    gen_kwargs["forced_bos_token_id"] = forced_bos_token_id
                translated_tokens = model.generate(**inputs, **gen_kwargs)

            zh_text = tokenizer.batch_decode(translated_tokens, skip_special_tokens=True)[0]
            utt["chinese_text"] = zh_text
            
            pct = 20.0 + (float(i + 1) / float(total)) * 75.0
            print(f"[PROGRESS:{pct:.1f}%]")
            print(f"[{i+1}/{total}] SK: {sk_text} -> ZH: {zh_text}")

    # Write final utterance_metadata.json
    final_doc = {
        "video_source": data.get("video_source", "input.mp4"),
        "total_duration": sum(u.get("duration", 0.0) for u in utterances),
        "sample_rate": 24000,
        "source_language": src_lang,
        "target_language": tgt_lang,
        "utterances": utterances,
        "generated_at_iso": "2026-09-06T12:00:00Z",
        "is_verified_by_user": False
    }

    target_save_path = meta_path if meta_path else os.path.join(workspace, "utterance_metadata.json")
    with open(target_save_path, "w", encoding="utf-8") as f:
        json.dump(final_doc, f, ensure_ascii=False, indent=2)

    print("[PROGRESS:100.0%]")
    print(f"=== Fáza 3: Preklad úspešne dokončený. Metadáta uložené v: {target_save_path} ===")

def main():
    parser = argparse.ArgumentParser(description="Stage 3: Slovak to Chinese Translation (NLLB-200)")
    parser.add_argument("--workspace", required=True)
    parser.add_argument("--meta", required=True)
    parser.add_argument("--model", default="facebook/nllb-200-distilled-600M")
    parser.add_argument("--src", default="slk_Latn")
    parser.add_argument("--tgt", default="zho_Hans")
    parser.add_argument("--simulate", action="store_true", default=False)
    args = parser.parse_args()

    try:
        run_translation(args.workspace, args.meta, args.model, args.src, args.tgt, args.simulate)
    except Exception as e:
        print(f"CHYBA vo Fáze 3 (Preklad): {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
