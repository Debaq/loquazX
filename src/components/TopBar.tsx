interface Props {
  projectName: string;
  canSave: boolean;
  canExtractAudio: boolean;
  extractingAudio: boolean;
  hasAudio: boolean;
  onNew: () => void;
  onOpen: () => void;
  onSave: () => void;
  onExtractAudio: () => void;
}

function TopBar({
  projectName,
  canSave,
  canExtractAudio,
  extractingAudio,
  hasAudio,
  onNew,
  onOpen,
  onSave,
  onExtractAudio,
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
      </div>
    </header>
  );
}

export default TopBar;
