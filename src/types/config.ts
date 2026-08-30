export type AsrEngine = 'whisper_sk' | 'faster_whisper';
export type AsrDevice = 'gpu_rocm' | 'cpu';
export type TtsEngine = 'piper' | 'kokoro' | 'coqui_xtts';
export type LipsyncEngine = 'latentsync_1_5' | 'musetalk';

export interface AppConfig {
  wsl_distro: string;
  venv_path: string;
  workspace_dir: string;
  models_cache_dir: string;

  // ASR
  asr_engine: AsrEngine;
  asr_device: AsrDevice;
  whisper_sk_model_id: string;

  // MT
  mt_model_id: string;
  source_lang: string;
  target_lang: string;

  // TTS
  tts_engine: TtsEngine;
  tts_voice: string;
  tts_speed_factor: number;

  // Lip-sync
  lipsync_engine: LipsyncEngine;
  lipsync_batch_size: number;
  lipsync_fallback_on_oom: boolean;
  rocm_sdpa_fallback: boolean;

  // Audio/Video
  target_resolution: string;
  ducking_level_db: number;
  auto_pause_for_review: boolean;

  // Sim
  simulate_mode: boolean;
}
