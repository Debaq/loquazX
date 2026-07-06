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
  Music,
  FilePlus2,
  Clapperboard,
  Wand2,
  Sparkles,
  RotateCcw,
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
  /** Importa un video o un PDF como fuente del proyecto (ADR-002 / ADR-010). */
  onImportVideo: () => void;
  onExtractAudio: () => void;
  onTranscribe: () => void;
  onExportTranslation: () => void;
  onImportTranslation: () => void;
  onTranslateLocal: () => void;
  onOpenModels: () => void;
  /** Importa un audio arbitrario cuando no hay video (ADR-010). */
  onImportAudioPresentation: () => void;
  /** Importa segmentos desde un JSON externo (ADR-010). */
  onImportSegmentsJson: () => void;
  /** Renderiza el video de presentación (ADR-010). */
  onExportPresentation: () => void;
  /** `true` cuando el proyecto tiene un PDF, al menos un segmento traducido
   * y una voz configurada; el render auto-doblará los pendientes. */
  canExportPresentation: boolean;
  /** Cantidad de segmentos traducidos que aún no tienen WAV; el botón
   * «Exportar video» los doblará en el mismo paso si hay alguno. */
  segmentsToDubCount: number;
  /** `true` mientras se renderiza el video de presentación. */
  renderingPresentation: boolean;
  /** Avance del render de presentación, si está corriendo. */
  renderProgress?: { etapa: number; total: number } | null;
  /** Planifica los segmentos a 2s secuenciales para doblar en orden (ADR-010). */
  onEliminarTiempos: () => void;
  /** Aplica manualmente las duraciones reales de los audios a los `start`/`end`. */
  onAplicarTiempos: () => void;
  /** Restaura los `start`/`end` originales desde el backup. */
  onRestaurarTiempos: () => void;
  /** `true` si existe el backup `timings.original.json` en disco. */
  hasTimingsBackup: boolean;
  /** `true` si el proyecto está en modo placeholder (después de «Eliminar tiempos»). */
  inPlaceholder: boolean;
  /** `true` mientras se ejecuta la planificación o restauración de tiempos. */
  timingsWorking: boolean;
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
  onImportAudioPresentation,
  onImportSegmentsJson,
  onExportPresentation,
  canExportPresentation,
  segmentsToDubCount,
  renderingPresentation,
  renderProgress,
  onEliminarTiempos,
  onAplicarTiempos,
  onRestaurarTiempos,
  hasTimingsBackup,
  inPlaceholder,
  timingsWorking,
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
          title="2. Importar video o PDF"
          aria-label="Paso 2: Importar video o PDF"
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
        {/* ADR-010: herramientas del modo presentación (audio + segmentos). */}
        <button
          type="button"
          className="topbar__btn"
          onClick={onImportAudioPresentation}
          disabled={!canSave || !!hasAudio}
          title={
            hasAudio
              ? "El proyecto ya tiene audio. Reimporta la fuente para reemplazarlo."
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
          onClick={onEliminarTiempos}
          disabled={!canExportPresentation || timingsWorking || renderingPresentation}
          title={
            timingsWorking
              ? "Planificando tiempos…"
              : "Eliminar tiempos de los segmentos (los pone a 2s cada uno para doblar en orden natural)"
          }
          aria-label="Eliminar tiempos"
        >
          {timingsWorking ? (
            <Loader size={ICON_SIZE} className="topbar__spin" />
          ) : (
            <Wand2 size={ICON_SIZE} />
          )}
          <span>Eliminar tiempos</span>
        </button>
        {inPlaceholder && (
          <button
            type="button"
            className="topbar__btn topbar__btn--label"
            onClick={onAplicarTiempos}
            disabled={timingsWorking || renderingPresentation}
            title="Aplicar las duraciones reales de los audios a los start/end de los segmentos"
            aria-label="Aplicar tiempos reales"
          >
            {timingsWorking ? (
              <Loader size={ICON_SIZE} className="topbar__spin" />
            ) : (
              <Sparkles size={ICON_SIZE} />
            )}
            <span>Aplicar tiempos</span>
          </button>
        )}
        {hasTimingsBackup && (
          <button
            type="button"
            className="topbar__btn"
            onClick={onRestaurarTiempos}
            disabled={timingsWorking || renderingPresentation}
            title="Restaurar los tiempos originales (antes de «Eliminar tiempos»)"
            aria-label="Restaurar tiempos"
          >
            <RotateCcw size={ICON_SIZE} />
          </button>
        )}
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
              : segmentsToDubCount > 0
                ? `Exportar video (doblará ${segmentsToDubCount} segmento${segmentsToDubCount === 1 ? "" : "s"} primero)`
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
