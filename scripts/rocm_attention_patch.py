#!/usr/bin/env python3
"""
ROCm Attention Patch for LatentSync & Diffusers
Replaces CUDA-specific xFormers / FlashAttention with PyTorch native Scaled Dot Product Attention (SDPA).
Ensures stable inference on AMD Radeon RX 7000 series (RDNA3 / ROCm 6.2+).
"""

import math
import torch
import torch.nn.functional as F

def apply_rocm_sdpa_patch():
    """
    Patches diffusers and latentsync attention processors to use PyTorch native SDPA.
    Native SDPA supports FlashAttention and memory efficient attention on ROCm out-of-the-box.
    """
    print("[ROCm Patch] Aplikujem optimalizáciu natívnej SDPA pre AMD GPU architektúru...")
    
    # 1. Disable xformers if imported to prevent HIP kernel crash
    try:
        import sys
        if 'xformers' in sys.modules:
            del sys.modules['xformers']
    except Exception:
        pass

    # 2. Patch PyTorch backend flags for ROCm
    if hasattr(torch.backends, "cuda") and hasattr(torch.backends.cuda, "enable_flash_sdp"):
        torch.backends.cuda.enable_flash_sdp(True)
        torch.backends.cuda.enable_mem_efficient_sdp(True)
        torch.backends.cuda.enable_math_sdp(True)
        print("[ROCm Patch] PyTorch SDPA backendy (Flash/MemEfficient/Math) úspešne aktivované.")

    # 3. Patch diffusers AttentionProcessor if available
    try:
        from diffusers.models.attention_processor import AttnProcessor2_0
        import diffusers.models.attention_processor as attn_module
        
        # Replace default xFormers processor with native AttnProcessor2_0 (SDPA)
        if hasattr(attn_module, "XFormersAttnProcessor"):
            attn_module.XFormersAttnProcessor = AttnProcessor2_0
            print("[ROCm Patch] XFormersAttnProcessor nahradený za PyTorch 2.0 AttnProcessor2_0 (SDPA).")
    except ImportError:
        pass

    return True

if __name__ == "__main__":
    apply_rocm_sdpa_patch()
    print("[ROCm Patch] Test inicializácie dokončený.")
