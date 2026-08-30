export interface DependencyCheckItem {
  id: string;
  title: string;
  description: string;
  category: 'wsl' | 'system' | 'python' | 'repos' | 'models' | string;
  is_installed: boolean;
  version_detected: string | null;
  is_critical: boolean;
  error_message: string | null;
  fix_hint: string | null;
}

export interface SystemDiagnosticsReport {
  all_ok: boolean;
  readiness_percentage: number;
  is_reboot_pending: boolean;
  items: DependencyCheckItem[];
  timestamp_ms: number;
}

export interface ModelManifestItem {
  id: string;
  name: string;
  category: 'asr' | 'translation' | 'tts' | 'lipsync' | 'auxiliary';
  description: string;
  license: string;
  is_commercial_safe: boolean;
  approximate_size_mb: number;
  local_relative_path: string;
  download_urls: string[];
  expected_sha256?: string;
  is_required_for_mvp: boolean;
}
