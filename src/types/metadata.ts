export interface WordItem {
  word: string;
  start: number;
  end: number;
  score?: number;
}

export interface UtteranceItem {
  id: string;
  start_time: number;
  end_time: number;
  duration: number;
  speaker_id: string;
  slovak_text: string;
  chinese_text: string;
  target_audio_file?: string;
  speed_factor: number;
  is_edited: boolean;
  confidence?: number;
  words: WordItem[];
}

export interface UtteranceMetadataDocument {
  video_source: string;
  total_duration: number;
  sample_rate: number;
  source_language: string;
  target_language: string;
  utterances: UtteranceItem[];
  generated_at_iso: string;
  is_verified_by_user: boolean;
}
