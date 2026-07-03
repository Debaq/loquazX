import {
  FilePlus,
  FolderOpen,
  Save,
  Film,
  AudioLines,
  Captions,
  FileUp,
  FileDown,
  Languages,
  Cpu,
  Loader,
  FileText,
  Music,
  FilePlus2,
  Clapperboard,
} from "lucide-react";
import { LANGUAGES } from "../languages";

interface Props {
  projectName: string;
  canSave: boolean;
  canImportVideo: boolean;
  canExtractAudio: boolean;
  extractingAudio: boolean;
  hasAudio: boolean;
  transcribing: boolean;
  hasSegments: boolean;
  translating: boolean;
  translateProgress: { done: number; total: number } | null;
  modelLevel: string;
  sourceLanguage: string;
  targetLanguage: string;
  onChangeLanguages: (origen: string, destino: string) => void;
  onNew: () => void;
  onOpen: () => void;
  onSave: () => void;
  onImportVideo: () => void;
  onExtractAudio: () => void;
  onTranscribe: () => void;
  onExportTranslation: () => void;
  onImportTranslation: () => void;
  onTranslateLocal: () => void;
  onOpenModels: () => void;
  /** Importa un PDF de fondo para el modo presentación (ADR-010). */
  onImportPdf: () => void;
  /** Importa un audio arbitrario cuando no hay video (ADR-010). */
  onImportAudioPresentation: () => void;
  /** Importa segmentos desde un JSON externo (ADR-010). */
  onImportSegmentsJson: () => void;
  /** Renderiza el video de presentación (ADR-010). */
  onExportPresentation: () => void;
  /** `true` cuando el proyecto tiene un PDF y al menos un segmento doblado. */
  canExportPresentation: boolean;
  /** `true` mientras se renderiza el video de presentación. */
  renderingPresentation: boolean;
  /** Avance del render de presentación, si está corriendo. */
  renderProgress?: { etapa: number; total: number } | null;
}

const ICON_SIZE = 18;

function TopBar({
  projectName,
  canSave,
  canImportVideo,
  canExtractAudio,
  extractingAudio,
  hasAudio,
  transcribing,
  hasSegments,
  translating,
  translateProgress,
  modelLevel,
  sourceLanguage,
  targetLanguage,
  onChangeLanguages,
  onNew,
  onOpen,
  onSave,
  onImportVideo,
  onExtractAudio,
  onTranscribe,
  onExportTranslation,
  onImportTranslation,
  onTranslateLocal,
  onOpenModels,
  onImportPdf,
  onImportAudioPresentation,
  onImportSegmentsJson,
  onExportPresentation,
  canExportPresentation,
  renderingPresentation,
  renderProgress,
}: Props) {
  return (
    <header className="topbar">
      <div className="topbar__title">loquazX — {projectName}</div>
      <div className="topbar__actions">
        <button
          type="button"
          className="topbar__btn topbar__btn--step"
          data-step="1"
          onClick={onNew}
          title="1. Nuevo proyecto"
          aria-label="Paso 1: Nuevo proyecto"
        >
          <FilePlus size={ICON_SIZE} />
        </button>
        <button
          type="button"
          className="topbar__btn topbar__btn--step"
          data-step="1"
          onClick={onOpen}
          title="1. Abrir proyecto"
          aria-label="Paso 1: Abrir proyecto"
        >
          <FolderOpen size={ICON_SIZE} />
        </button>
        <button
          type="button"
          className="topbar__btn"
          onClick={onSave}
          disabled={!canSave}
          title="Guardar proyecto"
          aria-label="Guardar proyecto"
        >
          <Save size={ICON_SIZE} />
        </button>
        <button
          type="button"
          className="topbar__btn topbar__btn--step"
          data-step="2"
          onClick={onImportVideo}
          disabled={!canImportVideo}
          title="2. Importar video"
          aria-label="Paso 2: Importar video"
        >
          <Film size={ICON_SIZE} />
        </button>
        <button
          type="button"
          className="topbar__btn topbar__btn--step"
          data-step="3"
          onClick={onExtractAudio}
          disabled={!canExtractAudio || extractingAudio}
          title={
            extractingAudio
              ? "3. Extrayendo audio…"
              : hasAudio
                ? "3. Reextraer audio"
                : "3. Extraer audio"
          }
          aria-label="Paso 3: Extraer audio"
        >
          {extractingAudio ? (
            <Loader size={ICON_SIZE} className="topbar__spin" />
          ) : (
            <AudioLines size={ICON_SIZE} />
          )}
        </button>
        <label
          className="topbar__lang topbar__lang--step"
          data-step="4"
          title="Paso 4: Idioma de origen"
        >
          Origen
          <select
            value={sourceLanguage}
            onChange={(e) => onChangeLanguages(e.target.value, targetLanguage)}
            disabled={transcribing}
            title="4. Idioma hablado en el audio (lo usa whisper al transcribir)"
          >
            {LANGUAGES.map((l) => (
              <option key={l.code} value={l.code}>
                {l.label}
              </option>
            ))}
          </select>
        </label>
        <button
          type="button"
          className="topbar__btn topbar__btn--step"
          data-step="5"
          onClick={onTranscribe}
          disabled={!hasAudio || transcribing || extractingAudio}
          title={
            transcribing
              ? "5. Transcribiendo…"
              : hasSegments
                ? "5. Retranscribir"
                : "5. Transcribir"
          }
          aria-label="Paso 5: Transcribir"
        >
          {transcribing ? (
            <Loader size={ICON_SIZE} className="topbar__spin" />
          ) : (
            <Captions size={ICON_SIZE} />
          )}
        </button>
        <label
          className="topbar__lang topbar__lang--step"
          data-step="6"
          title="Paso 6: Idioma de destino (al que se traduce)"
        >
          Destino
          <select
            value={targetLanguage}
            onChange={(e) => onChangeLanguages(sourceLanguage, e.target.value)}
            disabled={transcribing || translating}
            title="6. Idioma al que se traduce (lo usan «Exportar para traducir» y «Traducir con IA local»)"
          >
            {LANGUAGES.map((l) => (
              <option key={l.code} value={l.code}>
                {l.label}
              </option>
            ))}
          </select>
        </label>
        <button
          type="button"
          className="topbar__btn topbar__btn--step"
          data-step="6"
          onClick={onExportTranslation}
          disabled={!hasSegments || transcribing}
          title="6. Exportar para traducir"
          aria-label="Paso 6: Exportar para traducir"
        >
          <FileUp size={ICON_SIZE} />
        </button>
        <button
          type="button"
          className="topbar__btn topbar__btn--step"
          data-step="6"
          onClick={onImportTranslation}
          disabled={!hasSegments || transcribing}
          title="6. Importar traducción"
          aria-label="Paso 6: Importar traducción"
        >
          <FileDown size={ICON_SIZE} />
        </button>
        <button
          type="button"
          className="topbar__btn topbar__btn--step"
          data-step="6"
          onClick={onTranslateLocal}
          disabled={!hasSegments || transcribing || translating}
          title={
            translating
              ? translateProgress
                ? `6. Traduciendo… (${translateProgress.done}/${translateProgress.total})`
                : "6. Traduciendo…"
              : "6. Traducir con IA local (sin red)"
          }
          aria-label="Paso 6: Traducir con IA local"
        >
          {translating ? (
            <Loader size={ICON_SIZE} className="topbar__spin" />
          ) : (
            <Languages size={ICON_SIZE} />
          )}
        </button>
        <button
          type="button"
          className="topbar__btn topbar__btn--label"
          onClick={onOpenModels}
          disabled={transcribing}
          title={`Modelo: ${modelLevel}`}
          aria-label={`Modelo: ${modelLevel}`}
        >
          <Cpu size={ICON_SIZE} />
          <span>{modelLevel}</span>
        </button>
        {/* ADR-010: herramientas del modo presentación (PDF + segmentos). */}
        <button
          type="button"
          className="topbar__btn"
          onClick={onImportPdf}
          disabled={!canSave}
          title="Importar PDF de fondo (presentación)"
          aria-label="Importar PDF de fondo"
        >
          <FileText size={ICON_SIZE} />
        </button>
        <button
          type="button"
          className="topbar__btn"
          onClick={onImportAudioPresentation}
          disabled={!canSave || !!hasAudio}
          title={
            hasAudio
              ? "El proyecto ya tiene audio. Reimporta el video o PDF para reemplazarlo."
              : "Importar audio arbitrario para el modo presentación"
          }
          aria-label="Importar audio"
        >
          <Music size={ICON_SIZE} />
        </button>
        <button
          type="button"
          className="topbar__btn"
          onClick={onImportSegmentsJson}
          disabled={!canSave}
          title="Importar segmentos desde un JSON"
          aria-label="Importar segmentos JSON"
        >
          <FilePlus2 size={ICON_SIZE} />
        </button>
        <button
          type="button"
          className="topbar__btn topbar__btn--label"
          onClick={onExportPresentation}
          disabled={!canExportPresentation || renderingPresentation}
          title={
            renderingPresentation
              ? renderProgress
                ? `Renderizando presentación… (${renderProgress.etapa}/${renderProgress.total})`
                : "Renderizando presentación…"
              : "Exportar video de presentación"
          }
          aria-label="Exportar video de presentación"
        >
          {renderingPresentation ? (
            <Loader size={ICON_SIZE} className="topbar__spin" />
          ) : (
            <Clapperboard size={ICON_SIZE} />
          )}
          <span>Exportar video</span>
        </button>
      </div>
    </header>
  );
}

export default TopBar;
