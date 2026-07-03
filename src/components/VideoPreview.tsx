import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Segment } from "../types";

interface Props {
  videoPath: string | null;
  hasProject: boolean;
  videoRef: (el: HTMLVideoElement | null) => void;
  /** Ruta absoluta del proyecto; necesaria para resolver las páginas del PDF (ADR-010). */
  projectPath: string | null;
  /** PDF de fondo importado (modo presentación, ADR-010). */
  slidesPath: string | null;
  /** Conteo de páginas del PDF; controla los límites del campo de slide. */
  slidesPageCount: number | null;
  /** Segmentos del proyecto; el preview muestra la página del segmento seleccionado. */
  segments: Segment[];
  /** Id del segmento activo. */
  selectedId: string | null;
}

/** Devuelve la página del PDF que debe mostrarse para el segmento seleccionado
 * según la lista de segmentos. Si el segmento activo no tiene `slide`, mantiene
 * la última página vista (al inicio del video se asume la 1). Coincide con la
 * convención del backend al renderizar (ADR-010). */
function paginaActiva(segments: Segment[], selectedId: string | null): number {
  let page = 1;
  for (const s of segments) {
    if (s.slide != null) page = s.slide;
    if (s.id === selectedId) break;
  }
  return page;
}

function VideoPreview({
  videoPath,
  hasProject,
  videoRef,
  projectPath,
  slidesPath,
  slidesPageCount,
  segments,
  selectedId,
}: Props) {
  const [src, setSrc] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Modo presentación: la "diapositiva" activa según el segmento seleccionado.
  const [slideSrc, setSlideSrc] = useState<string | null>(null);
  const [slideError, setSlideError] = useState<string | null>(null);

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

  // Modo presentación: imagen de la página activa, servida por el `MediaServer`
  // local (ADR-005). Se recarga cuando cambia la página o el proyecto.
  const page = paginaActiva(segments, selectedId);
  useEffect(() => {
    setSlideSrc(null);
    setSlideError(null);
    if (videoPath || !slidesPath || !projectPath || !slidesPageCount) return;
    if (page < 1 || page > slidesPageCount) {
      setSlideError(`Página ${page} fuera de rango (1–${slidesPageCount}).`);
      return;
    }
    const pngPath = `${projectPath}/slides/pages/page-${page}.png`;
    invoke<string>("url_media", { path: pngPath })
      .then(setSlideSrc)
      .catch((e) => setSlideError(String(e)));
  }, [videoPath, slidesPath, projectPath, slidesPageCount, page]);

  if (videoPath) {
    return (
      <div className="preview">
        {src && (
          <video
            key={src}
            ref={videoRef}
            className="preview__video"
            src={src}
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

  if (slidesPath && projectPath) {
    return (
      <div className="preview">
        {slideSrc ? (
          <img
            key={slideSrc}
            className="preview__slide"
            src={slideSrc}
            alt={`Diapositiva ${page}`}
            onError={() => setSlideError(`No se pudo cargar la página ${page}.`)}
          />
        ) : (
          <div className="preview__placeholder">
            <div className="preview__icon">▢</div>
            <div className="preview__hint">
              {slideError ?? `Cargando página ${page}…`}
            </div>
          </div>
        )}
        {slideError && slideSrc && (
          <div className="preview__hint">{slideError}</div>
        )}
        {slidesPageCount != null && (
          <div className="preview__slide-meta">
            Página {page} de {slidesPageCount}
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="preview">
      <div className="preview__placeholder">
        <div className="preview__icon">▶</div>
        <div className="preview__hint">
          {hasProject ? "Sin video ni presentación importados" : "Sin proyecto"}
        </div>
        <div className="preview__sub">
          {hasProject
            ? "Importa un video o un PDF desde la barra superior"
            : "Crea o abre un proyecto para empezar"}
        </div>
      </div>
    </div>
  );
}

export default VideoPreview;
