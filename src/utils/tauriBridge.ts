import { AppConfig } from '../types/config';
import { PipelineExecutionState, FullPipelineResourceBudget, LiveSystemMetrics, ProcessLogLine } from '../types/pipeline';
import { SystemDiagnosticsReport, ModelManifestItem } from '../types/wizard';
import { UtteranceMetadataDocument, UtteranceItem } from '../types/metadata';

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
    __TAURI__?: unknown;
  }
}

export const isTauriEnvironment = (): boolean => {
  return typeof window !== 'undefined' && (!!window.__TAURI_INTERNALS__ || !!window.__TAURI__);
};

// Default in-memory config for web dev / fallback
let mockConfig: AppConfig = {
  wsl_distro: 'Ubuntu-24.04',
  venv_path: '~/.dubbing_env',
  workspace_dir: '~/ai_dubbing_workspace',
  models_cache_dir: '~/ai_dubbing_workspace/models',
  asr_engine: 'whisper_sk',
  asr_device: 'gpu_rocm',
  whisper_sk_model_id: 'NaiveNeuron/whisper-large-v3-sk',
  mt_model_id: 'facebook/nllb-200-distilled-600M',
  source_lang: 'slk_Latn',
  target_lang: 'zho_Hans',
  tts_engine: 'piper',
  tts_voice: 'zh_CN-huayan-medium',
  tts_speed_factor: 1.0,
  lipsync_engine: 'latentsync_1_5',
  lipsync_batch_size: 8,
  lipsync_fallback_on_oom: true,
  rocm_sdpa_fallback: true,
  target_resolution: 'original',
  ducking_level_db: -14.0,
  auto_pause_for_review: true,
  simulate_mode: false,
};

let mockMetadata: UtteranceMetadataDocument = {
  video_source: "prezentacia_produktu.mp4",
  total_duration: 14.8,
  sample_rate: 24000,
  source_language: "slk_Latn",
  target_language: "zho_Hans",
  generated_at_iso: new Date().toISOString(),
  is_verified_by_user: false,
  utterances: [
    {
      id: "utt_001",
      start_time: 0.5,
      end_time: 3.8,
      duration: 3.3,
      speaker_id: "SPEAKER_00",
      slovak_text: "Dobrý deň, vítam vás pri prezentácii nášho nového produktu.",
      chinese_text: "您好，欢迎来到我们新产品的展示会。",
      target_audio_file: "audio_segments/utt_001.wav",
      speed_factor: 1.0,
      is_edited: false,
      confidence: 0.98,
      words: [
        { word: "Dobrý", start: 0.5, end: 0.8 },
        { word: "deň,", start: 0.8, end: 1.1 },
        { word: "vítam", start: 1.2, end: 1.5 },
        { word: "vás", start: 1.5, end: 1.7 },
        { word: "pri", start: 1.8, end: 2.0 },
        { word: "prezentácii", start: 2.0, end: 2.7 },
        { word: "nového", start: 2.8, end: 3.2 },
        { word: "produktu.", start: 3.2, end: 3.8 }
      ]
    },
    {
      id: "utt_002",
      start_time: 4.2,
      end_time: 9.0,
      duration: 4.8,
      speaker_id: "SPEAKER_00",
      slovak_text: "Tento systém využíva pokročilú umelú inteligenciu a beží kompletne lokálne na vašom hardvéri.",
      chinese_text: "该系统利用先进的人工智能，并完全在您的本地硬件上运行。",
      target_audio_file: "audio_segments/utt_002.wav",
      speed_factor: 1.05,
      is_edited: false,
      confidence: 0.96,
      words: []
    },
    {
      id: "utt_003",
      start_time: 9.6,
      end_time: 14.8,
      duration: 5.2,
      speaker_id: "SPEAKER_00",
      slovak_text: "Vďaka optimalizácii pre grafické karty AMD Radeon dosahuje vysoký výkon bez odosielania dát na cloud.",
      chinese_text: "由于针对AMD Radeon显卡进行了优化，无需将数据发送到云端即可实现高性能。",
      target_audio_file: "audio_segments/utt_003.wav",
      speed_factor: 0.98,
      is_edited: false,
      confidence: 0.97,
      words: []
    }
  ]
};

let mockPipelineState: PipelineExecutionState = {
  is_running: false,
  is_paused_for_review: false,
  current_stage_index: 0,
  stages: [
    { id: 'demux', name: '1. Extrakcia & Demuxing', description: 'Izolácia zvukovej stopy a normalizácia (FFmpeg)', status: 'idle', progress_percent: 0, started_at_ms: null, completed_at_ms: null, estimated_vram_gb: 0, estimated_ram_gb: 0.5, engine_badge: 'FFmpeg CLI', is_gpu_accelerated: false, error_message: null, user_suggestion: null },
    { id: 'asr', name: '2. Slovenský ASR (Prepis)', description: 'Slovenský prepis s presnými časovými značkami slov (Whisper-SK)', status: 'idle', progress_percent: 0, started_at_ms: null, completed_at_ms: null, estimated_vram_gb: 5.5, estimated_ram_gb: 4.5, engine_badge: 'Whisper-SK (ROCm)', is_gpu_accelerated: true, error_message: null, user_suggestion: null },
    { id: 'translate', name: '3. Preklad SK → ZH', description: 'Neurónový preklad slk_Latn do zho_Hans (NLLB-200)', status: 'idle', progress_percent: 0, started_at_ms: null, completed_at_ms: null, estimated_vram_gb: 2.5, estimated_ram_gb: 2.0, engine_badge: 'NLLB-200 (PyTorch)', is_gpu_accelerated: true, error_message: null, user_suggestion: null },
    { id: 'review', name: '4. Kontrola Metadát & Prekladu', description: 'Interaktívna kontrola utterance_metadata pred syntézou reči', status: 'idle', progress_percent: 0, started_at_ms: null, completed_at_ms: null, estimated_vram_gb: 0, estimated_ram_gb: 0.2, engine_badge: 'Interaktívny Editor', is_gpu_accelerated: false, error_message: null, user_suggestion: null },
    { id: 'tts', name: '5. Čínska Syntéza Reči (TTS)', description: 'Generovanie čínskeho audia so zarovnaním dĺžky segmentov', status: 'idle', progress_percent: 0, started_at_ms: null, completed_at_ms: null, estimated_vram_gb: 0.5, estimated_ram_gb: 0.8, engine_badge: 'Piper (MIT)', is_gpu_accelerated: false, error_message: null, user_suggestion: null },
    { id: 'lipsync', name: '6. Lip-Sync Synchronizácia', description: 'Rozanimovanie pier tváre podľa reči (LatentSync 1.5 / MuseTalk fallback)', status: 'idle', progress_percent: 0, started_at_ms: null, completed_at_ms: null, estimated_vram_gb: 7.5, estimated_ram_gb: 6.0, engine_badge: 'LatentSync 1.5 (SDPA)', is_gpu_accelerated: true, error_message: null, user_suggestion: null },
    { id: 'mux', name: '7. Záverečný Muxing', description: 'Zmiešanie hudby s dabingom a vpečenie titulkov (FFmpeg)', status: 'idle', progress_percent: 0, started_at_ms: null, completed_at_ms: null, estimated_vram_gb: 0, estimated_ram_gb: 1.0, engine_badge: 'FFmpeg Mux', is_gpu_accelerated: false, error_message: null, user_suggestion: null },
  ],
  input_video_path_win: "C:\\AI_Dubbing\\Videos\\vstupna_prezentacia.mp4",
  input_video_path_wsl: "/mnt/c/AI_Dubbing/Videos/vstupna_prezentacia.mp4",
  output_video_path_win: "C:\\AI_Dubbing\\Videos\\vstupna_prezentacia_dubbed_zh.mp4",
  output_video_path_wsl: "/mnt/c/AI_Dubbing/Videos/vstupna_prezentacia_dubbed_zh.mp4",
  metadata_json_path_win: "C:\\AI_Dubbing\\Videos\\vstupna_prezentacia_utterance_metadata.json",
  metadata_json_path_wsl: "/mnt/c/AI_Dubbing/Videos/vstupna_prezentacia_utterance_metadata.json",
  error_summary: null,
  active_lipsync_engine: "LatentSync 1.5",
};

// Event listeners registry
type EventCallback = (payload: any) => void;
const listeners: { [event: string]: EventCallback[] } = {};

export function addTauriListener(event: string, callback: EventCallback): () => void {
  let tauriUnlisten: (() => void) | null = null;
  let isCancelled = false;

  if (isTauriEnvironment()) {
    import('@tauri-apps/api/event')
      .then(({ listen }) => {
        if (isCancelled) return;
        return listen(event, (e: { payload: any }) => {
          callback(e.payload);
        });
      })
      .then((unlistenFn) => {
        if (!unlistenFn) return;
        if (isCancelled) {
          unlistenFn();
        } else {
          tauriUnlisten = unlistenFn;
        }
      })
      .catch((err) => {
        console.warn(`[Tauri Event Listen Error on ${event}]`, err);
      });
  }

  if (!listeners[event]) {
    listeners[event] = [];
  }
  listeners[event].push(callback);
  return () => {
    isCancelled = true;
    if (tauriUnlisten) {
      tauriUnlisten();
    }
    listeners[event] = (listeners[event] || []).filter(cb => cb !== callback);
  };
}

export function emitMockEvent(event: string, payload: any) {
  if (listeners[event]) {
    listeners[event].forEach(cb => cb(payload));
  }
}

export async function invokeCommand<T>(command: string, args: Record<string, any> = {}): Promise<T> {
  if (isTauriEnvironment()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<T>(command, args);
    } catch (err) {
      console.warn(`[Tauri Invoke Error on ${command}]`, err);
      // Fall through to mock
    }
  }

  // Web / Simulation fallbacks
  switch (command) {
    case 'get_config':
      return { ...mockConfig } as unknown as T;

    case 'save_config':
      mockConfig = { ...args.new_config };
      return undefined as unknown as T;

    case 'reset_config_to_default':
      return { ...mockConfig } as unknown as T;

    case 'get_pipeline_state':
      return { ...mockPipelineState } as unknown as T;

    case 'set_pipeline_video': {
      const p = args.video_path || "C:\\Videos\\sample.mp4";
      mockPipelineState.input_video_path_win = p;
      mockPipelineState.input_video_path_wsl = `/mnt/c/${p.replace(/^[A-Z]:\\/, '').replace(/\\/g, '/')}`;
      mockPipelineState.output_video_path_win = p.replace(/\.mp4$/, '_dubbed_zh.mp4');
      mockPipelineState.output_video_path_wsl = mockPipelineState.input_video_path_wsl.replace(/\.mp4$/, '_dubbed_zh.mp4');
      return { ...mockPipelineState } as unknown as T;
    }

    case 'get_resource_budget': {
      const budget: FullPipelineResourceBudget = {
        hardware_profile: "AMD Ryzen 5 5600 (16GB RAM) + Radeon RX 7700 XT (12GB VRAM)",
        total_gpu_vram_mb: 12288,
        total_system_ram_mb: 16384,
        peak_vram_mb: 7680,
        peak_ram_mb: 6144,
        is_overall_safe: true,
        stages: [
          { stage_id: 'demux', stage_name: 'Extrakcia & Demuxing', estimated_ram_mb: 512, estimated_vram_mb: 0, is_gpu_active: false, max_supported_vram_mb: 12288, max_supported_ram_mb: 16384, is_safe: true, warning_message: null, recommendation: null },
          { stage_id: 'asr', stage_name: 'Slovenský ASR (Whisper-SK)', estimated_ram_mb: 4200, estimated_vram_mb: 5600, is_gpu_active: true, max_supported_vram_mb: 12288, max_supported_ram_mb: 16384, is_safe: true, warning_message: null, recommendation: null },
          { stage_id: 'translate', stage_name: 'Preklad SK → ZH (NLLB-200)', estimated_ram_mb: 2048, estimated_vram_mb: 2560, is_gpu_active: true, max_supported_vram_mb: 12288, max_supported_ram_mb: 16384, is_safe: true, warning_message: null, recommendation: null },
          { stage_id: 'review', stage_name: 'Kontrola Metadát', estimated_ram_mb: 256, estimated_vram_mb: 0, is_gpu_active: false, max_supported_vram_mb: 12288, max_supported_ram_mb: 16384, is_safe: true, warning_message: null, recommendation: null },
          { stage_id: 'tts', stage_name: 'Syntéza Reči (Piper TTS)', estimated_ram_mb: 800, estimated_vram_mb: 256, is_gpu_active: false, max_supported_vram_mb: 12288, max_supported_ram_mb: 16384, is_safe: true, warning_message: null, recommendation: "Piper TTS je komerčne bezpečný (MIT) a ultra-ľahký na pamäť." },
          { stage_id: 'lipsync', stage_name: 'Lip-Sync (LatentSync 1.5)', estimated_ram_mb: 6144, estimated_vram_mb: 7680, is_gpu_active: true, max_supported_vram_mb: 12288, max_supported_ram_mb: 16384, is_safe: true, warning_message: null, recommendation: "LatentSync 1.5 spotrebuje ~7.5 GB VRAM. Pri OOM sa aktivuje záložný MuseTalk." },
          { stage_id: 'mux', stage_name: 'Muxing & Audio Ducking', estimated_ram_mb: 1024, estimated_vram_mb: 0, is_gpu_active: false, max_supported_vram_mb: 12288, max_supported_ram_mb: 16384, is_safe: true, warning_message: null, recommendation: null },
        ]
      };
      return budget as unknown as T;
    }

    case 'get_live_system_metrics': {
      const metrics: LiveSystemMetrics = {
        host_ram_used_mb: 8420,
        host_ram_total_mb: 16384,
        host_ram_percent: 51.4,
        cpu_usage_percent: 24.5,
        gpu_vram_used_mb: 4120,
        gpu_vram_total_mb: 12288,
        gpu_vram_percent: 33.5,
        gpu_name: "AMD Radeon RX 7700 XT (12 GB)",
        is_rocm_ready: true,
        timestamp_ms: Date.now(),
      };
      return metrics as unknown as T;
    }

    case 'load_utterance_metadata':
    case 'get_demo_utterance_metadata':
      return { ...mockMetadata } as unknown as T;

    case 'save_utterance_metadata':
      if (args.document) {
        mockMetadata = { ...args.document };
      }
      return undefined as unknown as T;

    case 'run_system_diagnostics': {
      const diag: SystemDiagnosticsReport = {
        all_ok: true,
        readiness_percentage: 100,
        is_reboot_pending: false,
        timestamp_ms: Date.now(),
        items: [
          { id: 'wsl2_installed', title: 'WSL2 Virtualizačná platforma', description: 'Windows Subsystem for Linux v2', category: 'wsl', is_installed: true, version_detected: 'WSL 2.2.4', is_critical: true, error_message: null, fix_hint: null },
          { id: 'distro_ubuntu_24_04', title: 'Distribúcia Ubuntu-24.04', description: 'Linuxové prostredie Ubuntu 24.04 LTS', category: 'wsl', is_installed: true, version_detected: 'Ubuntu 24.04.1 LTS', is_critical: true, error_message: null, fix_hint: null },
          { id: 'pkg_ffmpeg', title: 'FFmpeg Media Framework', description: 'Extrakcia audia, strih a muxing', category: 'system', is_installed: true, version_detected: 'FFmpeg 6.1.1', is_critical: true, error_message: null, fix_hint: null },
          { id: 'pkg_git', title: 'Git CLI & Nástroje', description: 'Klonovanie AI repozitárov', category: 'system', is_installed: true, version_detected: 'git 2.43.0', is_critical: true, error_message: null, fix_hint: null },
          { id: 'python_venv', title: 'Python Virtuálne Prostredie (~/.dubbing_env)', description: 'Izolované Python 3.12 prostredie', category: 'python', is_installed: true, version_detected: 'Python 3.12.3', is_critical: true, error_message: null, fix_hint: null },
          { id: 'pytorch_rocm', title: 'PyTorch s AMD ROCm podporou', description: 'GPU akcelerácia pre RX 7700 XT', category: 'python', is_installed: true, version_detected: 'PyTorch 2.5.0+rocm6.2 (HIP: 6.2.4)', is_critical: true, error_message: null, fix_hint: null },
          { id: 'repo_latentsync', title: 'LatentSync 1.5 Repozitár & SDPA Patch', description: 'Primárny UNet lip-sync model (~7.5 GB VRAM)', category: 'repos', is_installed: true, version_detected: 'LatentSync v1.5', is_critical: true, error_message: null, fix_hint: null },
          { id: 'repo_musetalk', title: 'MuseTalk Repozitár (Fallback)', description: 'Záložný lip-sync model (~4.5 GB VRAM)', category: 'repos', is_installed: true, version_detected: 'MuseTalk 0.1', is_critical: false, error_message: null, fix_hint: null },
          { id: 'model_whisper-large-v3-sk', title: 'Whisper Large-v3 SK Model', description: 'NaiveNeuron/whisper-large-v3-sk (3.1 GB)', category: 'models', is_installed: true, version_detected: 'Nainštalovaný v HF Cache', is_critical: true, error_message: null, fix_hint: null },
          { id: 'model_nllb-200-distilled-600m', title: 'NLLB-200 Distilled 600M Prekladač', description: 'facebook/nllb-200-distilled-600M (1.2 GB)', category: 'models', is_installed: true, version_detected: 'Nainštalovaný', is_critical: true, error_message: null, fix_hint: null },
          { id: 'model_piper-zh-huayan', title: 'Piper TTS Chinese (Huayan)', description: 'zh_CN-huayan-medium.onnx (MIT Licencia)', category: 'models', is_installed: true, version_detected: 'Prítomný', is_critical: true, error_message: null, fix_hint: null },
          { id: 'model_latentsync-1-5', title: 'LatentSync 1.5 Checkpoint', description: 'latentsync_unet.pt (3.8 GB)', category: 'models', is_installed: true, version_detected: 'Prítomný', is_critical: true, error_message: null, fix_hint: null },
        ]
      };
      return diag as unknown as T;
    }

    case 'get_models_manifest': {
      const manifest: ModelManifestItem[] = [
        { id: "whisper-large-v3-sk", name: "Whisper Large-v3 SK Fine-tune", category: "asr", description: "Slovenský ASR model od NaiveNeuron s presným časovaním slov a interpunkciou.", license: "Apache 2.0 (Komerčne bezpečné)", is_commercial_safe: true, approximate_size_mb: 3100, local_relative_path: "models/asr/whisper-large-v3-sk", download_urls: ["https://huggingface.co/NaiveNeuron/whisper-large-v3-sk"], is_required_for_mvp: true },
        { id: "nllb-200-distilled-600m", name: "NLLB-200 Distilled 600M", category: "translation", description: "Vysokorýchlostný neurónový prekladač slk_Latn -> zho_Hans s nízkou spotrebou VRAM (~2 GB).", license: "CC-BY-NC-4.0 / Research", is_commercial_safe: false, approximate_size_mb: 1200, local_relative_path: "models/mt/nllb-200-distilled-600M", download_urls: ["https://huggingface.co/facebook/nllb-200-distilled-600M"], is_required_for_mvp: true },
        { id: "piper-zh-huayan", name: "Piper TTS — Chinese (Huayan Medium)", category: "tts", description: "Ultra-rýchly syntetizátor čínskej reči pre CPU aj ROCm. Vhodný pre komerčné nasadenie.", license: "MIT (Komerčne bezpečné)", is_commercial_safe: true, approximate_size_mb: 65, local_relative_path: "models/tts/piper/zh_CN-huayan-medium.onnx", download_urls: ["https://huggingface.co/rhasspy/piper-voices"], is_required_for_mvp: true },
        { id: "kokoro-v019", name: "Kokoro TTS (v0.19 Multilingual)", category: "tts", description: "Moderný 82M neurónový TTS model s vysokou kvalitou intonácie.", license: "Apache 2.0 (Komerčne bezpečné)", is_commercial_safe: true, approximate_size_mb: 340, local_relative_path: "models/tts/kokoro/kokoro-v0_19.onnx", download_urls: ["https://huggingface.co/hexgrad/Kokoro-82M"], is_required_for_mvp: false },
        { id: "coqui-xtts-v2", name: "Coqui XTTS-v2 (Klonovanie hlasu)", category: "tts", description: "Viacjazyčný TTS model s klonovaním hlasu. UPOZORNENIE: CPML nekomerčná licencia!", license: "CPML (Len nekomerčné / testovacie)", is_commercial_safe: false, approximate_size_mb: 3200, local_relative_path: "models/tts/coqui-xtts-v2", download_urls: ["https://huggingface.co/coqui/XTTS-v2"], is_required_for_mvp: false },
        { id: "latentsync-1-5", name: "LatentSync 1.5 Checkpoint", category: "lipsync", description: "UNet lip-sync checkpoint pre LatentSync v1.5 (~7.5 GB VRAM). Optimalizované pre ROCm SDPA.", license: "Apache 2.0", is_commercial_safe: true, approximate_size_mb: 3800, local_relative_path: "models/lipsync/latentsync/latentsync_unet.pt", download_urls: ["https://huggingface.co/ByteDance/LatentSync"], is_required_for_mvp: true },
        { id: "musetalk-weights", name: "MuseTalk Checkpoints & DWPose", category: "lipsync", description: "Odľahčený lip-sync model s nízkou spotrebou (~4.5 GB VRAM) — ideálny fallback pri OOM.", license: "MIT", is_commercial_safe: true, approximate_size_mb: 2200, local_relative_path: "models/lipsync/musetalk/musetalk.json", download_urls: ["https://huggingface.co/TMElyralab/MuseTalk"], is_required_for_mvp: true },
      ];
      return manifest as unknown as T;
    }

    default:
      return {} as unknown as T;
  }
}
