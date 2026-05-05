interface Props {
  projectName: string;
}

function TopBar({ projectName }: Props) {
  return (
    <header className="topbar">
      <div className="topbar__title">loquazX — {projectName}</div>
      <div className="topbar__actions">
        <button type="button" disabled>Nuevo</button>
        <button type="button" disabled>Abrir</button>
        <button type="button" disabled>Guardar</button>
      </div>
    </header>
  );
}

export default TopBar;
