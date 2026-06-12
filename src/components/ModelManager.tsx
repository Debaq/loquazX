import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ask, open, message } from "@tauri-apps/plugin-dialog";
import type { ModelInfo, VoiceInfo, EdgeVoice, DownloadProgress } from "../types";
import { LANGUAGES } from "../languages";

interface Props {
  selectedLevel: string;
  onSelectLevel: (nivel: string) => void;
  onClose: () => void;
}

type Tab = "transcripcion" | "voces" | "traduccion" | "edge";

const MUESTRA_POR_DEFECTO = "Hola, esta es una prueba de voz para el doblaje.";

function porcentaje(p: DownloadProgress): number {
  if (p.total === 0) return 0;
  return Math.min(100, Math.round((p.descargado / p.total) * 100));
}

function ModelManager({ selectedLevel, onSelectLevel, onClose }: Props) {
  const [tab, setTab] = useState<Tab>("transcripcion");
  const [modelos, setModelos] = useState<ModelInfo[]>([]);
  const [voces, setVoces] = useState<VoiceInfo[]>([]);
  // Motor de traducción local (NLLB/ONNX); misma forma que ModelInfo.
  const [motores, setMotores] = useState<ModelInfo[]>([]);
  // El progreso se indexa por id (nivel de whisper o id de voz); ambos eventos
  // comparten la forma `DownloadProgress` con el campo `nivel`.
  const [progreso, setProgreso] = useState<Record<string, DownloadProgress>>({});
  const [ocupado, setOcupado] = useState<string | null>(null);

  // edge-tts (online): voces para audicionar y elegir.
  const [edgeVoces, setEdgeVoces] = useState<EdgeVoice[] | null>(null);
  const [edgeCargando, setEdgeCargando] = useState(false);
  const [edgeIdioma, setEdgeIdioma] = useState("es");
  const [edgeTexto, setEdgeTexto] = useState(MUESTRA_POR_DEFECTO);
  const [edgeProbando, setEdgeProbando] = useState<string | null>(null);
  const [audioSrc, setAudioSrc] = useState<string | null>(null);

  async function refrescar() {
    try {
      setModelos(await invoke<ModelInfo[]>("listar_modelos"));
      setVoces(await invoke<VoiceInfo[]>("listar_voces"));
      setMotores(await invoke<ModelInfo[]>("listar_motores_traduccion"));
    } catch (e) {
      await message(String(e), { title: "Modelos y voces", kind: "error" });
    }
  }

  useEffect(() => {
    refrescar();
    // El backend emite el avance por id; lo reflejamos sin importar la pestaña.
    const seguir = (evento: string) =>
      listen<DownloadProgress>(evento, (e) => {
        setProgreso((prev) => ({ ...prev, [e.payload.nivel]: e.payload }));
      });
    const subs = [
      seguir("modelo:progreso"),
      seguir("voz:progreso"),
      seguir("modelo-traduccion:progreso"),
    ];
    return () => {
      subs.forEach((s) => s.then((fn) => fn()));
    };
  }, []);

  function limpiarProgreso(id: string) {
    setProgreso((prev) => {
      const copia = { ...prev };
      delete copia[id];
      return copia;
    });
  }

  async function descargarModelo(nivel: string) {
    setOcupado(nivel);
    try {
      await invoke("descargar_modelo", { nivel });
      onSelectLevel(nivel);
      await refrescar();
    } catch (e) {
      await message(String(e), { title: "Descargar modelo", kind: "error" });
    } finally {
      setOcupado(null);
      limpiarProgreso(nivel);
    }
  }

  async function importarModelo(nivel: string) {
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

  async function eliminarModelo(nivel: string) {
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

  async function descargarTraduccion(id: string) {
    setOcupado(id);
    try {
      await invoke("descargar_motor_traduccion");
      await refrescar();
    } catch (e) {
      await message(String(e), { title: "Descargar modelo de traducción", kind: "error" });
    } finally {
      setOcupado(null);
      limpiarProgreso(id);
    }
  }

  async function descargarVoz(voz: string) {
    setOcupado(voz);
    try {
      await invoke("descargar_voz", { voz });
      await refrescar();
    } catch (e) {
      await message(String(e), { title: "Descargar voz", kind: "error" });
    } finally {
      setOcupado(null);
      limpiarProgreso(voz);
    }
  }

  async function eliminarVoz(voz: string) {
    const seguro = await ask(`¿Borrar la voz «${voz}» del disco?`, {
      title: "Borrar voz",
      kind: "warning",
    });
    if (!seguro) return;
    setOcupado(voz);
    try {
      await invoke("eliminar_voz", { voz });
      await refrescar();
    } catch (e) {
      await message(String(e), { title: "Borrar voz", kind: "error" });
    } finally {
      setOcupado(null);
    }
  }

  // Las voces edge-tts se listan por red; se cargan al abrir la pestaña, no antes.
  useEffect(() => {
    if (tab !== "edge" || edgeVoces !== null || edgeCargando) return;
    setEdgeCargando(true);
    invoke<EdgeVoice[]>("listar_voces_edge")
      .then(setEdgeVoces)
      .catch(async (e) => {
        await message(String(e), { title: "Voces edge-tts", kind: "error" });
        setEdgeVoces([]);
      })
      .finally(() => setEdgeCargando(false));
  }, [tab, edgeVoces, edgeCargando]);

  async function probarEdge(shortName: string) {
    setEdgeProbando(shortName);
    setAudioSrc(null);
    try {
      const url = await invoke<string>("probar_voz_edge", {
        voz: shortName,
        texto: edgeTexto.trim() || MUESTRA_POR_DEFECTO,
      });
      setAudioSrc(url);
    } catch (e) {
      await message(String(e), { title: "Probar voz edge-tts", kind: "error" });
    } finally {
      setEdgeProbando(null);
    }
  }

  function barraProgreso(id: string) {
    const enCurso = ocupado === id;
    const p = progreso[id];
    if (!enCurso || !p) return null;
    return (
      <div className="modelos__progress">
        <div className="modelos__bar" style={{ width: `${porcentaje(p)}%` }} />
        <span className="modelos__pct">{porcentaje(p)}%</span>
      </div>
    );
  }

  return (
    <div className="modal" onClick={onClose}>
      <div className="modal__panel" onClick={(e) => e.stopPropagation()}>
        <div className="modal__header">
          <h2>Modelos y voces</h2>
          <button type="button" onClick={onClose}>
            Cerrar
          </button>
        </div>

        <div className="modal__tabs">
          <button
            type="button"
            className={tab === "transcripcion" ? "is-active" : ""}
            onClick={() => setTab("transcripcion")}
          >
            Transcripción
          </button>
          <button
            type="button"
            className={tab === "voces" ? "is-active" : ""}
            onClick={() => setTab("voces")}
          >
            Voces Piper (local)
          </button>
          <button
            type="button"
            className={tab === "traduccion" ? "is-active" : ""}
            onClick={() => setTab("traduccion")}
          >
            Traducción
          </button>
          <button
            type="button"
            className={tab === "edge" ? "is-active" : ""}
            onClick={() => setTab("edge")}
          >
            Voces edge-tts (online)
          </button>
        </div>

        {tab === "transcripcion" && (
          <>
            <p className="modal__hint">
              Descarga un modelo whisper y queda guardado para usarlo siempre. A
              mayor nivel, mejor calidad y mayor peso.
            </p>
            <ul className="modelos">
              {modelos.map((m) => {
                const enCurso = ocupado === m.id;
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
                    {barraProgreso(m.id)}
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
                            onClick={() => eliminarModelo(m.id)}
                          >
                            Borrar
                          </button>
                        </>
                      ) : (
                        <>
                          <button
                            type="button"
                            disabled={ocupado !== null}
                            onClick={() => descargarModelo(m.id)}
                          >
                            {enCurso ? "Descargando…" : "Descargar"}
                          </button>
                          <button
                            type="button"
                            disabled={ocupado !== null}
                            onClick={() => importarModelo(m.id)}
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
          </>
        )}

        {tab === "voces" && (
          <>
            <p className="modal__hint">
              Voces locales Piper para el doblaje (sin red). edge-tts es online y no
              requiere descarga. Descarga la voz del idioma de salida que necesites.
            </p>
            <ul className="modelos">
              {voces.map((v) => {
                const enCurso = ocupado === v.id;
                return (
                  <li key={v.id} className="modelos__item">
                    <div className="modelos__info">
                      <span className="modelos__label">
                        {v.label}
                        <span className="modelos__badge">{v.language}</span>
                        {v.downloaded && <span className="modelos__badge">descargada</span>}
                      </span>
                      <span className="modelos__size">≈ {v.approx_size_mb} MB</span>
                    </div>
                    {barraProgreso(v.id)}
                    <div className="modelos__actions">
                      {v.downloaded ? (
                        <button
                          type="button"
                          disabled={ocupado !== null}
                          onClick={() => eliminarVoz(v.id)}
                        >
                          Borrar
                        </button>
                      ) : (
                        <button
                          type="button"
                          disabled={ocupado !== null}
                          onClick={() => descargarVoz(v.id)}
                        >
                          {enCurso ? "Descargando…" : "Descargar"}
                        </button>
                      )}
                    </div>
                  </li>
                );
              })}
            </ul>
          </>
        )}

        {tab === "traduccion" && (
          <>
            <p className="modal__hint">
              Modelo de traducción local NLLB-200 (NMT, 200 idiomas) sobre ONNX, sin
              red. Rápido en CPU. Alternativa al export/import a un LLM externo.
            </p>
            <ul className="modelos">
              {motores.map((m) => {
                const enCurso = ocupado === m.id;
                return (
                  <li key={m.id} className="modelos__item">
                    <div className="modelos__info">
                      <span className="modelos__label">
                        {m.label}
                        {m.downloaded && <span className="modelos__badge">descargado</span>}
                      </span>
                      <span className="modelos__size">≈ {m.approx_size_mb} MB</span>
                    </div>
                    {barraProgreso(m.id)}
                    <div className="modelos__actions">
                      {m.downloaded ? (
                        <button type="button" disabled>
                          Descargado
                        </button>
                      ) : (
                        <button
                          type="button"
                          disabled={ocupado !== null}
                          onClick={() => descargarTraduccion(m.id)}
                        >
                          {enCurso ? "Descargando…" : "Descargar"}
                        </button>
                      )}
                    </div>
                  </li>
                );
              })}
            </ul>
          </>
        )}

        {tab === "edge" && (
          <>
            <p className="modal__hint">
              Voces online de Microsoft (edge-tts). Audiciona y elige la mejor. Ojo:
              el texto de prueba se envía a Microsoft por red (opt-in, no es la opción
              por defecto). No requiere descarga.
            </p>
            <div className="edge__controls">
              <label className="edge__field">
                Idioma
                <select
                  value={edgeIdioma}
                  onChange={(e) => setEdgeIdioma(e.target.value)}
                >
                  {LANGUAGES.map((l) => (
                    <option key={l.code} value={l.code}>
                      {l.label}
                    </option>
                  ))}
                </select>
              </label>
              <label className="edge__field edge__field--grow">
                Texto de prueba
                <input
                  type="text"
                  value={edgeTexto}
                  onChange={(e) => setEdgeTexto(e.target.value)}
                  placeholder={MUESTRA_POR_DEFECTO}
                />
              </label>
            </div>
            {audioSrc && (
              <audio className="edge__audio" src={audioSrc} controls autoPlay />
            )}
            {edgeCargando && <div className="modal__hint">Cargando voces…</div>}
            {edgeVoces !== null && !edgeCargando && (
              <ul className="modelos">
                {edgeVoces
                  .filter((v) => v.language === edgeIdioma)
                  .map((v) => {
                    const probando = edgeProbando === v.short_name;
                    return (
                      <li key={v.short_name} className="modelos__item">
                        <div className="modelos__info">
                          <span className="modelos__label">
                            {v.short_name}
                            {v.gender && (
                              <span className="modelos__badge">{v.gender}</span>
                            )}
                          </span>
                          <span className="modelos__size">{v.locale}</span>
                        </div>
                        <div className="modelos__actions">
                          <button
                            type="button"
                            disabled={edgeProbando !== null}
                            onClick={() => probarEdge(v.short_name)}
                          >
                            {probando ? "Generando…" : "Probar"}
                          </button>
                        </div>
                      </li>
                    );
                  })}
                {edgeVoces.filter((v) => v.language === edgeIdioma).length === 0 && (
                  <div className="modal__hint">
                    Sin voces edge-tts para este idioma.
                  </div>
                )}
              </ul>
            )}
          </>
        )}
      </div>
    </div>
  );
}

export default ModelManager;
