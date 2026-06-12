import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Segment, Waveform } from "../types";
import { LANGUAGES } from "../languages";

const ZOOM_MIN = 1;
const ZOOM_MAX = 60;

interface Props {
  video: HTMLVideoElement | null;
  segments: Segment[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  audioPath: string | null;
  outputLanguages: string[];
  /** Ruta absoluta del proyecto, para resolver los WAV de doblaje. */
  projectPath: string | null;
  /** Ids de segmentos que ya tienen doblaje generado (ADR-009). */
  dubs: string[];
  /** Cambia con cada generación para recargar las ondas del doblaje. */
  dubVersion: number;
  canDub: boolean;
  dubbing: boolean;
  dubProgress: { done: number; total: number } | null;
  onGenerateDub: () => void;
}

function nombreIdioma(code: string): string {
  return LANGUAGES.find((l) => l.code === code)?.label ?? code.toUpperCase();
}

const BUCKETS = 1000;

function formatTime(seconds: number): string {
  const s = Number.isFinite(seconds) ? seconds : 0;
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  const tenths = Math.floor((s % 1) * 10);
  return `${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}.${tenths}`;
}

function Timeline({
  video,
  segments,
  selectedId,
  onSelect,
  audioPath,
  outputLanguages,
  projectPath,
  dubs,
  dubVersion,
  canDub,
  dubbing,
  dubProgress,
  onGenerateDub,
}: Props) {
  const [time, setTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [waveform, setWaveform] = useState<Waveform | null>(null);
  const [zoom, setZoom] = useState(1);
  const scrollRef = useRef<HTMLDivElement>(null);
  // Ondas y URLs locales del audio de doblaje por segmento (ADR-009).
  const [dubWaves, setDubWaves] = useState<Record<string, Waveform>>({});
  const [dubUrls, setDubUrls] = useState<Record<string, string>>({});
  // Canal que se escucha: el audio original del video o un idioma de doblaje.
  const [channel, setChannel] = useState<string>("original");
  const dubAudioRef = useRef<HTMLAudioElement>(null);

  // Re-suscribe cada vez que cambia el elemento (nuevo video => nuevo <video>).
  useEffect(() => {
    if (!video) {
      setTime(0);
      setDuration(0);
      setPlaying(false);
      return;
    }
    const onTime = () => setTime(video.currentTime);
    const onMeta = () => setDuration(Number.isFinite(video.duration) ? video.duration : 0);
    const onPlay = () => setPlaying(true);
    const onPause = () => setPlaying(false);

    video.addEventListener("timeupdate", onTime);
    video.addEventListener("loadedmetadata", onMeta);
    video.addEventListener("durationchange", onMeta);
    video.addEventListener("play", onPlay);
    video.addEventListener("pause", onPause);
    onMeta();
    onTime();
    setPlaying(!video.paused);

    return () => {
      video.removeEventListener("timeupdate", onTime);
      video.removeEventListener("loadedmetadata", onMeta);
      video.removeEventListener("durationchange", onMeta);
      video.removeEventListener("play", onPlay);
      video.removeEventListener("pause", onPause);
    };
  }, [video]);

  // La onda se recalcula solo al cambiar el audio del proyecto.
  useEffect(() => {
    if (!audioPath) {
      setWaveform(null);
      return;
    }
    let vigente = true;
    invoke<Waveform>("forma_onda", { path: audioPath, buckets: BUCKETS })
      .then((w) => {
        if (vigente) setWaveform(w);
      })
      .catch(() => {
        if (vigente) setWaveform(null);
      });
    return () => {
      vigente = false;
    };
  }, [audioPath]);

  // Carga la onda y la URL local de cada segmento doblado. Se rehace al cambiar
  // el conjunto de doblajes o tras una (re)generación (`dubVersion`).
  useEffect(() => {
    if (!projectPath || dubs.length === 0) {
      setDubWaves({});
      setDubUrls({});
      return;
    }
    let vigente = true;
    const waves: Record<string, Waveform> = {};
    const urls: Record<string, string> = {};
    Promise.all(
      dubs.map(async (id) => {
        const wav = `${projectPath}/runs/dub/${id}.wav`;
        try {
          const [w, url] = await Promise.all([
            invoke<Waveform>("forma_onda", { path: wav, buckets: BUCKETS }),
            invoke<string>("url_media", { path: wav }),
          ]);
          waves[id] = w;
          urls[id] = url;
        } catch {
          /* un doblaje ausente o ilegible simplemente no se dibuja. */
        }
      }),
    ).then(() => {
      if (vigente) {
        setDubWaves(waves);
        setDubUrls(urls);
      }
    });
    return () => {
      vigente = false;
    };
  }, [projectPath, dubs, dubVersion]);

  // Reproducción del canal de doblaje sincronizada con el video: cuando el canal
  // activo es un idioma, se mutea el video y se reproduce el WAV del segmento en
  // curso desfasado a su inicio; en «original» se restaura el audio del video.
  useEffect(() => {
    const a = dubAudioRef.current;
    if (!video || !a) return;
    if (channel === "original") {
      video.muted = false;
      if (!a.paused) a.pause();
      return;
    }
    video.muted = true;
    const seg = segments.find(
      (s) => time >= s.start && time < s.end && dubUrls[s.id],
    );
    if (!seg) {
      if (!a.paused) a.pause();
      return;
    }
    if (a.dataset.seg !== seg.id) {
      a.dataset.seg = seg.id;
      a.src = dubUrls[seg.id];
    }
    const offset = time - seg.start;
    if (Math.abs(a.currentTime - offset) > 0.25) a.currentTime = offset;
    if (playing && a.paused) a.play().catch(() => {});
    if (!playing && !a.paused) a.pause();
  }, [time, channel, playing, segments, dubUrls, video]);

  // Al salir del canal de doblaje (o desmontar), restaura el audio del video.
  useEffect(() => {
    return () => {
      if (video) video.muted = false;
    };
  }, [video]);

  // Escala común a todas las pistas: lo más largo entre video, audio y el
  // último segmento, para que todo quede alineado verticalmente.
  const scale = useMemo(() => {
    const ultimoFin = segments.reduce((max, s) => Math.max(max, s.end), 0);
    return Math.max(duration, waveform?.duration ?? 0, ultimoFin);
  }, [duration, waveform, segments]);

  const disabled = !video;
  const progreso = scale ? (time / scale) * 100 : 0;

  // Cambia el zoom manteniendo fijo el punto bajo el cursor (o el centro).
  function aplicarZoom(nuevo: number, anchorClientX?: number) {
    const z = Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, nuevo));
    const el = scrollRef.current;
    if (!el) {
      setZoom(z);
      return;
    }
    const rect = el.getBoundingClientRect();
    const cursorX = anchorClientX != null ? anchorClientX - rect.left : el.clientWidth / 2;
    const anchoViejo = el.clientWidth * zoom;
    const fraccion = anchoViejo ? (el.scrollLeft + cursorX) / anchoViejo : 0;
    setZoom(z);
    // El ancho real se actualiza tras el render; reajustamos el scroll después.
    requestAnimationFrame(() => {
      const anchoNuevo = el.clientWidth * z;
      el.scrollLeft = fraccion * anchoNuevo - cursorX;
    });
  }

  // Listener nativo no-passive: el onWheel de React es passive y no permite
  // preventDefault, lo que dejaría a Ctrl+rueda haciendo zoom del webview.
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const rueda = (e: WheelEvent) => {
      if (!e.ctrlKey) return; // Ctrl+rueda = zoom; rueda sola = scroll normal.
      e.preventDefault();
      aplicarZoom(zoom * (e.deltaY < 0 ? 1.2 : 1 / 1.2), e.clientX);
    };
    el.addEventListener("wheel", rueda, { passive: false });
    return () => el.removeEventListener("wheel", rueda);
  });

  // Con zoom, el cursor sigue al video durante la reproducción si se sale del
  // área visible.
  useEffect(() => {
    if (!playing || zoom <= 1) return;
    const el = scrollRef.current;
    if (!el || !scale) return;
    const x = (time / scale) * el.clientWidth * zoom;
    if (x < el.scrollLeft || x > el.scrollLeft + el.clientWidth) {
      el.scrollLeft = x - el.clientWidth / 2;
    }
  }, [time, playing, zoom, scale]);

  function alternarReproduccion() {
    if (!video) return;
    if (video.paused) video.play();
    else video.pause();
  }

  function saltar(delta: number) {
    if (!video) return;
    const limite = scale || video.duration || 0;
    video.currentTime = Math.max(0, Math.min(limite, video.currentTime + delta));
  }

  // Busca en el video a partir de la posición horizontal del clic.
  function buscarEnPosicion(e: React.MouseEvent<HTMLDivElement>) {
    if (!video || !scale) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const fraccion = (e.clientX - rect.left) / rect.width;
    video.currentTime = Math.max(0, Math.min(1, fraccion)) * scale;
  }

  // Path de la onda completa (fallback sin segmentos). El SVG escala con
  // `preserveAspectRatio: none`; el path centra la onda.
  const ondaPath = useMemo(() => {
    if (!waveform || waveform.peaks.length === 0) return "";
    return waveform.peaks
      .map((p, i) => `M${i} ${50 - p * 50}L${i} ${50 + p * 50}`)
      .join("");
  }, [waveform]);

  // Path de una onda completa (la del WAV de doblaje cubre todo el segmento).
  function ondaDeWave(w: Waveform | undefined): { len: number; d: string } {
    if (!w || w.peaks.length === 0) return { len: 1, d: "" };
    const d = w.peaks
      .map((p, i) => `M${i} ${50 - p * 50}L${i} ${50 + p * 50}`)
      .join("");
    return { len: w.peaks.length, d };
  }

  // Recorta la onda al rango de un segmento para dibujarla dentro de su bloque.
  function ondaSegmento(s: Segment): { len: number; d: string } {
    if (!waveform || waveform.peaks.length === 0) return { len: 1, d: "" };
    const dur = waveform.duration || scale;
    if (dur <= 0) return { len: 1, d: "" };
    const n = waveform.peaks.length;
    let i0 = Math.floor((s.start / dur) * n);
    let i1 = Math.ceil((s.end / dur) * n);
    i0 = Math.max(0, Math.min(i0, n - 1));
    i1 = Math.max(i0 + 1, Math.min(i1, n));
    const slice = waveform.peaks.slice(i0, i1);
    const d = slice.map((p, i) => `M${i} ${50 - p * 50}L${i} ${50 + p * 50}`).join("");
    return { len: slice.length || 1, d };
  }

  return (
    <div className="timeline">
      <div className="timeline__controls">
        <button type="button" disabled={disabled} onClick={() => saltar(-10)} title="Retroceder 10 s">
          ⏮
        </button>
        <button type="button" disabled={disabled} onClick={alternarReproduccion}>
          {playing ? "⏸" : "▶"}
        </button>
        <button type="button" disabled={disabled} onClick={() => saltar(10)} title="Avanzar 10 s">
          ⏭
        </button>
        <span className="timeline__time">
          {formatTime(time)} / {formatTime(scale)}
        </span>
        <div className="timeline__zoom">
          <button
            type="button"
            disabled={zoom <= ZOOM_MIN}
            onClick={() => aplicarZoom(zoom / 1.5)}
            title="Alejar"
          >
            −
          </button>
          <span className="timeline__zoom-level" title="Ctrl + rueda para hacer zoom">
            {Math.round(zoom * 100)}%
          </span>
          <button
            type="button"
            disabled={zoom >= ZOOM_MAX}
            onClick={() => aplicarZoom(zoom * 1.5)}
            title="Acercar"
          >
            +
          </button>
          <button
            type="button"
            disabled={zoom === 1}
            onClick={() => aplicarZoom(1)}
            title="Restablecer zoom"
          >
            ⤢
          </button>
        </div>
        <div className="timeline__channel" title="Audio que se escucha al reproducir">
          <span>🎧</span>
          <select value={channel} onChange={(e) => setChannel(e.target.value)}>
            <option value="original">Original</option>
            {outputLanguages.map((code) => (
              <option key={code} value={code} disabled={dubs.length === 0}>
                Doblaje · {nombreIdioma(code)}
              </option>
            ))}
          </select>
        </div>
      </div>
      <audio ref={dubAudioRef} style={{ display: "none" }} />

      <div className="timeline__scroll" ref={scrollRef}>
        <div
          className="timeline__tracks"
          style={{ width: `${zoom * 100}%` }}
          onClick={buscarEnPosicion}
        >
        {/* Cursor de reproducción común a todas las pistas. */}
        <div className="timeline__playhead" style={{ left: `${progreso}%` }} />

        {/* Pista 1: barra de tiempo con los segmentos. */}
        <div className="timeline__track timeline__track--ruler">
          <div className="timeline__label">Segmentos</div>
          <div className="timeline__lane">
            <div className="timeline__progress" style={{ width: `${progreso}%` }} />
            {scale > 0 &&
              segments.map((s) => {
                const left = (s.start / scale) * 100;
                const width = Math.max(((s.end - s.start) / scale) * 100, 0.4);
                return (
                  <div
                    key={s.id}
                    className={`timeline__segment ${s.id === selectedId ? "is-selected" : ""}`}
                    style={{ left: `${left}%`, width: `${width}%` }}
                    title={`${formatTime(s.start)} → ${formatTime(s.end)}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      onSelect(s.id);
                    }}
                  />
                );
              })}
            {scale > 0 &&
              segments.map((s) => (
                <div
                  key={`l-${s.id}`}
                  className="timeline__marker"
                  style={{ left: `${(s.start / scale) * 100}%` }}
                />
              ))}
          </div>
        </div>

        {/* Pista 2: audio extraído, en bloques por segmento. */}
        <div className="timeline__track timeline__track--audio">
          <div className="timeline__label">Audio</div>
          <div className="timeline__lane timeline__lane--wave">
            {!audioPath && <span className="timeline__empty">Sin audio extraído</span>}
            {audioPath && !waveform && <span className="timeline__empty">Calculando onda…</span>}
            {/* Sin segmentos aún: onda continua como referencia. */}
            {waveform && segments.length === 0 && ondaPath && (
              <svg
                className="timeline__wave"
                viewBox={`0 0 ${BUCKETS} 100`}
                preserveAspectRatio="none"
              >
                <path d={ondaPath} />
              </svg>
            )}
            {waveform &&
              scale > 0 &&
              segments.map((s) => {
                const left = (s.start / scale) * 100;
                const width = Math.max(((s.end - s.start) / scale) * 100, 0.4);
                const { len, d } = ondaSegmento(s);
                return (
                  <div
                    key={s.id}
                    className={`timeline__clip ${s.id === selectedId ? "is-selected" : ""}`}
                    style={{ left: `${left}%`, width: `${width}%` }}
                    title={`${formatTime(s.start)} → ${formatTime(s.end)}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      onSelect(s.id);
                    }}
                  >
                    {d && (
                      <svg
                        className="timeline__wave"
                        viewBox={`0 0 ${len} 100`}
                        preserveAspectRatio="none"
                      >
                        <path d={d} />
                      </svg>
                    )}
                  </div>
                );
              })}
          </div>
        </div>

        {/* Pista(s) 3: doblaje generado, una por idioma de salida (hoy solo el
            idioma destino del proyecto; ADR-009). Cada bloque marca un segmento
            ya doblado en `runs/dub/`. */}
        {outputLanguages.map((code) => (
          <div className="timeline__track" key={code}>
            <div
              className={`timeline__label ${channel === code ? "is-active" : ""}`}
            >
              Doblaje · {nombreIdioma(code)}
            </div>
            <button
              type="button"
              className="timeline__dub-btn"
              onClick={onGenerateDub}
              disabled={!canDub || dubbing}
            >
              {dubbing
                ? dubProgress
                  ? `Doblando ${dubProgress.done}/${dubProgress.total}…`
                  : "Doblando…"
                : "Generar todas"}
            </button>
            <div
              className={`timeline__lane timeline__lane--wave ${
                channel === code ? "is-active" : ""
              }`}
            >
              {dubs.length === 0 && (
                <span className="timeline__empty">Sin doblaje generado</span>
              )}
              {scale > 0 &&
                segments
                  .filter((s) => dubs.includes(s.id))
                  .map((s) => {
                    const left = (s.start / scale) * 100;
                    const width = Math.max(((s.end - s.start) / scale) * 100, 0.4);
                    const { len, d } = ondaDeWave(dubWaves[s.id]);
                    return (
                      <div
                        key={s.id}
                        className={`timeline__clip timeline__clip--dub ${
                          s.id === selectedId ? "is-selected" : ""
                        }`}
                        style={{ left: `${left}%`, width: `${width}%` }}
                        title={`Doblaje ${formatTime(s.start)} → ${formatTime(s.end)}`}
                        onClick={(e) => {
                          e.stopPropagation();
                          onSelect(s.id);
                        }}
                      >
                        {d && (
                          <svg
                            className="timeline__wave"
                            viewBox={`0 0 ${len} 100`}
                            preserveAspectRatio="none"
                          >
                            <path d={d} />
                          </svg>
                        )}
                      </div>
                    );
                  })}
            </div>
          </div>
        ))}
      </div>
      </div>
    </div>
  );
}

export default Timeline;
