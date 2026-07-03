export interface Segment {
  id: string;
  start: number;
  end: number;
  source: string;
  translation: string;
  /** Número de página del PDF que se muestra durante `[start, end)` (ADR-010). */
  slide?: number | null;
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

/** PDF de fondo del modo presentación (ADR-010). */
export interface Presentation {
  file: string;
  page_count: number;
  imported_at: number;
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
  slides?: Presentation;
}

export interface Project {
  path: string;
  manifest: ProjectManifest;
  segments: Segment[];
  video_path: string | null;
  audio_path: string | null;
  /** Ids de segmentos que ya tienen audio de doblaje generado (ADR-009). */
  dubs: string[];
  /** Ruta absoluta del PDF de fondo del modo presentación (ADR-010), si hay uno. */
  slides_path: string | null;
  /** Conteo de páginas del PDF de fondo, si hay uno. */
  slides_page_count: number | null;
}

/** Motor de síntesis de voz para el doblaje (ADR-009). */
export type DubEngine = "piper" | "edge-tts";

export interface DubSettings {
  engine: DubEngine;
  voice: string;
}

export interface DubReport {
  generated: number;
  skipped: number;
}

export interface DubResult {
  project: Project;
  report: DubReport;
}

export interface ModelInfo {
  id: string;
  label: string;
  approx_size_mb: number;
  downloaded: boolean;
  path: string | null;
}

export interface VoiceInfo {
  id: string;
  label: string;
  language: string;
  approx_size_mb: number;
  downloaded: boolean;
  path: string | null;
}

export interface DownloadProgress {
  nivel: string;
  descargado: number;
  total: number;
}

export interface EdgeVoice {
  short_name: string;
  locale: string;
  language: string;
  gender: string;
  friendly_name: string;
}

export interface Waveform {
  peaks: number[];
  duration: number;
}

export interface ExportResult {
  request_file: string;
  prompt_file: string;
  segment_count: number;
}

export interface MergeReport {
  translated: number;
  missing: number;
  unknown: number;
}

export interface ImportResult {
  project: Project;
  report: MergeReport;
}

/** Resultado del render de presentación (ADR-010). */
export interface RenderReport {
  output: string;
  duration_secs: number;
}
