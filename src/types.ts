export interface Segment {
  id: string;
  start: number;
  end: number;
  source: string;
  translation: string;
}

export interface SourceVideo {
  file: string;
  mode: "copy" | "reference";
  original_path: string;
}

export interface ExtractedAudio {
  file: string;
  extracted_at: number;
}

export interface ProjectManifest {
  id: string;
  format_version: number;
  name: string;
  source_language: string;
  target_language: string;
  created_at: number;
  source?: SourceVideo;
  audio?: ExtractedAudio;
}

export interface Project {
  path: string;
  manifest: ProjectManifest;
  segments: Segment[];
  video_path: string | null;
  audio_path: string | null;
}

export interface ModelInfo {
  id: string;
  label: string;
  approx_size_mb: number;
  downloaded: boolean;
  path: string | null;
}

export interface DownloadProgress {
  nivel: string;
  descargado: number;
  total: number;
}
