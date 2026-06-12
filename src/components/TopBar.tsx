interface Props {
  projectName: string;
  canSave: boolean;
  canExtractAudio: boolean;
  extractingAudio: boolean;
  hasAudio: boolean;
  transcribing: boolean;
  hasSegments: boolean;
  modelLevel: string;
  onNew: () => void;
  onOpen: () => void;
  onSave: () => void;
  onExtractAudio: () => void;
  onTranscribe: () => void;
  onOpenModels: () => void;
}

function TopBar({
  projectName,
  canSave,
  canExtractAudio,
  extractingAudio,
  hasAudio,
  transcribing,
  hasSegments,
  modelLevel,
  onNew,
  onOpen,
  onSave,
  onExtractAudio,
  onTranscribe,
  onOpenModels,
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
        <button type="button" onClick={onOpenModels} disabled={transcribing}>
          Modelo: {modelLevel}
        </button>
      </div>
    </header>
  );
}

export default TopBar;
