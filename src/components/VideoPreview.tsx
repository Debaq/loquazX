import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Props {
  videoPath: string | null;
  hasProject: boolean;
  onImport: () => void;
}

function VideoPreview({ videoPath, hasProject, onImport }: Props) {
  const [src, setSrc] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // ADR-005: el video se sirve por HTTP local; WebKitGTK no reproduce
  // media a través del protocolo asset.
  useEffect(() => {
    setSrc(null);
    setError(null);
    if (!videoPath) return;
    invoke<string>("url_media", { path: videoPath })
      .then(setSrc)
      .catch((e) => setError(String(e)));
  }, [videoPath]);

  if (videoPath) {
    return (
      <div className="preview">
        {src && (
          <video
            key={src}
            className="preview__video"
            src={src}
            controls
            onError={(e) => {
              const code = e.currentTarget.error?.code;
              setError(`No se pudo reproducir el video (MediaError ${code ?? "?"}).`);
            }}
          />
        )}
        {error && <div className="preview__hint">{error}</div>}
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
