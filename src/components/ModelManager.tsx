import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ask, open, message } from "@tauri-apps/plugin-dialog";
import type { ModelInfo, DownloadProgress } from "../types";

interface Props {
  selectedLevel: string;
  onSelectLevel: (nivel: string) => void;
  onClose: () => void;
}

function porcentaje(p: DownloadProgress): number {
  if (p.total === 0) return 0;
  return Math.min(100, Math.round((p.descargado / p.total) * 100));
}

function ModelManager({ selectedLevel, onSelectLevel, onClose }: Props) {
  const [modelos, setModelos] = useState<ModelInfo[]>([]);
  const [progreso, setProgreso] = useState<Record<string, DownloadProgress>>({});
  const [ocupado, setOcupado] = useState<string | null>(null);

  async function refrescar() {
    try {
      setModelos(await invoke<ModelInfo[]>("listar_modelos"));
    } catch (e) {
      await message(String(e), { title: "Modelos", kind: "error" });
    }
  }

  useEffect(() => {
    refrescar();
    // ADR-007: el backend emite el avance de la descarga; lo reflejamos por nivel.
    const desuscribir = listen<DownloadProgress>("modelo:progreso", (evento) => {
      setProgreso((prev) => ({ ...prev, [evento.payload.nivel]: evento.payload }));
    });
    return () => {
      desuscribir.then((fn) => fn());
    };
  }, []);

  async function descargar(nivel: string) {
    setOcupado(nivel);
    try {
      await invoke("descargar_modelo", { nivel });
      onSelectLevel(nivel);
      await refrescar();
    } catch (e) {
      await message(String(e), { title: "Descargar modelo", kind: "error" });
    } finally {
      setOcupado(null);
      setProgreso((prev) => {
        const copia = { ...prev };
        delete copia[nivel];
        return copia;
      });
    }
  }

  async function importar(nivel: string) {
    const archivo = await open({
      title: "Importar modelo GGML",
      filters: [{ name: "Modelo GGML", extensions: ["bin"] }],
    });
    if (!archivo) return;
    setOcupado(nivel);
    try {
      await invoke("importar_modelo", { nivel, archivo });
      onSelectLevel(nivel);
      await refrescar();
    } catch (e) {
      await message(String(e), { title: "Importar modelo", kind: "error" });
    } finally {
      setOcupado(null);
    }
  }

  async function eliminar(nivel: string) {
    const seguro = await ask(`¿Borrar el modelo «${nivel}» del disco?`, {
      title: "Borrar modelo",
      kind: "warning",
    });
    if (!seguro) return;
    setOcupado(nivel);
    try {
      await invoke("eliminar_modelo", { nivel });
      await refrescar();
    } catch (e) {
      await message(String(e), { title: "Borrar modelo", kind: "error" });
    } finally {
      setOcupado(null);
    }
  }

  return (
    <div className="modal" onClick={onClose}>
      <div className="modal__panel" onClick={(e) => e.stopPropagation()}>
        <div className="modal__header">
          <h2>Modelo de transcripción</h2>
          <button type="button" onClick={onClose}>
            Cerrar
          </button>
        </div>
        <p className="modal__hint">
          Descarga un modelo whisper y queda guardado para usarlo siempre. A mayor
          nivel, mejor calidad y mayor peso.
        </p>
        <ul className="modelos">
          {modelos.map((m) => {
            const enCurso = ocupado === m.id;
            const p = progreso[m.id];
            const activo = selectedLevel === m.id;
            return (
              <li key={m.id} className={`modelos__item ${activo ? "is-active" : ""}`}>
                <div className="modelos__info">
                  <span className="modelos__label">
                    {m.label}
                    {m.downloaded && <span className="modelos__badge">descargado</span>}
                    {activo && <span className="modelos__badge">en uso</span>}
                  </span>
                  <span className="modelos__size">≈ {m.approx_size_mb} MB</span>
                </div>
                {enCurso && p && (
                  <div className="modelos__progress">
                    <div
                      className="modelos__bar"
                      style={{ width: `${porcentaje(p)}%` }}
                    />
                    <span className="modelos__pct">{porcentaje(p)}%</span>
                  </div>
                )}
                <div className="modelos__actions">
                  {m.downloaded ? (
                    <>
                      <button
                        type="button"
                        disabled={enCurso || activo}
                        onClick={() => onSelectLevel(m.id)}
                      >
                        {activo ? "En uso" : "Usar"}
                      </button>
                      <button
                        type="button"
                        disabled={ocupado !== null}
                        onClick={() => eliminar(m.id)}
                      >
                        Borrar
                      </button>
                    </>
                  ) : (
                    <>
                      <button
                        type="button"
                        disabled={ocupado !== null}
                        onClick={() => descargar(m.id)}
                      >
                        {enCurso ? "Descargando…" : "Descargar"}
                      </button>
                      <button
                        type="button"
                        disabled={ocupado !== null}
                        onClick={() => importar(m.id)}
                      >
                        Importar .bin
                      </button>
                    </>
                  )}
                </div>
              </li>
            );
          })}
        </ul>
      </div>
    </div>
  );
}

export default ModelManager;
