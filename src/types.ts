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

export interface ProjectManifest {
  id: string;
  format_version: number;
  name: string;
  source_language: string;
  target_language: string;
  created_at: number;
  source?: SourceVideo;
}

export interface Project {
  path: string;
  manifest: ProjectManifest;
  segments: Segment[];
  video_path: string | null;
}
