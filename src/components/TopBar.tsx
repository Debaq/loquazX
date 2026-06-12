interface Props {
  projectName: string;
  canSave: boolean;
  canExtractAudio: boolean;
  extractingAudio: boolean;
  hasAudio: boolean;
  transcribing: boolean;
  hasSegments: boolean;
  onNew: () => void;
  onOpen: () => void;
  onSave: () => void;
  onExtractAudio: () => void;
  onTranscribe: () => void;
  onExportTranslation: () => void;
  onImportTranslation: () => void;
}

function TopBar({
  projectName,
  canSave,
  canExtractAudio,
  extractingAudio,
  hasAudio,
  transcribing,
  hasSegments,
  onNew,
  onOpen,
  onSave,
  onExtractAudio,
  onTranscribe,
  onExportTranslation,
  onImportTranslation,
}: Props) {
  return (
    <header className="topbar">
      <div className="topbar__title">loquazX — {projectName}</div>
      <div className="topbar__actions">
        <button type="button" onClick={onNew}>Nuevo</button>
        <button type="button" onClick={onOpen}>Abrir</button>
        <button type="button" onClick={onSave} disabled={!canSave}>Guardar</button>
        <button
          type="button"
          onClick={onExtractAudio}
          disabled={!canExtractAudio || extractingAudio}
        >
          {extractingAudio
            ? "Extrayendo audio…"
            : hasAudio
              ? "Reextraer audio"
              : "Extraer audio"}
        </button>
        <button
          type="button"
          onClick={onTranscribe}
          disabled={!hasAudio || transcribing || extractingAudio}
        >
          {transcribing
            ? "Transcribiendo…"
            : hasSegments
              ? "Retranscribir"
              : "Transcribir"}
        </button>
        <button
          type="button"
          onClick={onExportTranslation}
          disabled={!hasSegments || transcribing}
        >
          Exportar para traducir
        </button>
        <button
          type="button"
          onClick={onImportTranslation}
          disabled={!hasSegments || transcribing}
        >
          Importar traducción
        </button>
      </div>
    </header>
  );
}

export default TopBar;
