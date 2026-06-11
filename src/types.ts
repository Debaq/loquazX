export interface Segment {
  id: string;
  start: number;
  end: number;
  source: string;
  translation: string;
}

export interface ProjectManifest {
  id: string;
  format_version: number;
  name: string;
  source_language: string;
  target_language: string;
  created_at: number;
}

export interface Project {
  path: string;
  manifest: ProjectManifest;
  segments: Segment[];
}
