interface Props {
  projectName: string;
  canSave: boolean;
  onNew: () => void;
  onOpen: () => void;
  onSave: () => void;
}

function TopBar({ projectName, canSave, onNew, onOpen, onSave }: Props) {
  return (
    <header className="topbar">
      <div className="topbar__title">loquazX — {projectName}</div>
      <div className="topbar__actions">
        <button type="button" onClick={onNew}>Nuevo</button>
        <button type="button" onClick={onOpen}>Abrir</button>
        <button type="button" onClick={onSave} disabled={!canSave}>Guardar</button>
      </div>
    </header>
  );
}

export default TopBar;
