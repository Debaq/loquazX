import { convertFileSrc } from "@tauri-apps/api/core";

interface Props {
  videoPath: string | null;
  hasProject: boolean;
  onImport: () => void;
}

function VideoPreview({ videoPath, hasProject, onImport }: Props) {
  if (videoPath) {
    return (
      <div className="preview">
        <video
          className="preview__video"
          src={convertFileSrc(videoPath)}
          controls
        />
      </div>
    );
  }

  return (
    <div className="preview">
      <div className="preview__placeholder">
        <div className="preview__icon">▶</div>
        <div className="preview__hint">
          {hasProject ? "Sin video importado" : "Sin proyecto"}
        </div>
        {hasProject ? (
          <button type="button" onClick={onImport}>
            Importar video…
          </button>
        ) : (
          <div className="preview__sub">Crea o abre un proyecto para empezar</div>
        )}
      </div>
    </div>
  );
}

export default VideoPreview;
