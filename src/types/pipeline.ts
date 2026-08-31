export type StageId = 'demux' | 'asr' | 'translate' | 'review' | 'tts' | 'lipsync' | 'mux';

export type StageStatus = 'idle' | 'running' | 'review_paused' | 'completed' | 'failed' | 'skipped';

export interface PipelineStageInfo {
  id: StageId;
  name: string;
  description: string;
  status: StageStatus;
  progress_percent: number;
  started_at_ms: number | null;
  completed_at_ms: number | null;
  estimated_vram_gb: number;
  estimated_ram_gb: number;
  engine_badge: string;
  is_gpu_accelerated: boolean;
  error_message: string | null;
  user_suggestion: string | null;
}

export interface PipelineExecutionState {
  is_running: boolean;
  is_paused_for_review: boolean;
  current_stage_index: number;
  stages: PipelineStageInfo[];
  input_video_path_win: string;
  input_video_path_wsl: string;
  output_video_path_win: string | null;
  output_video_path_wsl: string | null;
  metadata_json_path_win: string | null;
  metadata_json_path_wsl: string | null;
  error_summary: string | null;
  active_lipsync_engine: string;
}

export interface StageResourceEstimate {
  stage_id: StageId;
  stage_name: string;
  estimated_ram_mb: number;
  estimated_vram_mb: number;
  is_gpu_active: boolean;
  max_supported_vram_mb: number;
  max_supported_ram_mb: number;
  is_safe: boolean;
  warning_message: string | null;
  recommendation: string | null;
}

export interface FullPipelineResourceBudget {
  stages: StageResourceEstimate[];
  peak_vram_mb: number;
  peak_ram_mb: number;
  total_system_ram_mb: number;
  total_gpu_vram_mb: number;
  is_overall_safe: boolean;
  hardware_profile: string;
}

export interface LiveSystemMetrics {
  host_ram_used_mb: number;
  host_ram_total_mb: number;
  host_ram_percent: number;
  cpu_usage_percent: number;
  gpu_vram_used_mb: number;
  gpu_vram_total_mb: number;
  gpu_vram_percent: number;
  gpu_name: string;
  is_rocm_ready: boolean;
  timestamp_ms: number;
}

export interface ProcessLogLine {
  stream: string;
  message: string;
  timestamp_ms: number;
  is_progress: boolean;
  progress_percent: number | null;
  step_tag: string | null;
}
