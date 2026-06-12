import { useEffect, useRef, useState } from "react";
import type { DubEngine, EdgeVoice, Segment, VoiceInfo } from "../types";

interface Props {
  segment: Segment | null;
  onChange: (id: string, cambios: Partial<Segment>) => void;
  /** Motor y voz elegidos (compartidos con la generación masiva). */
  engine: DubEngine;
  voice: string;
  onChangeEngine: (engine: DubEngine) => void;
  onChangeVoice: (voice: string) => void;
  /** Voces Piper descargadas para el idioma de salida. */
  piperVoices: VoiceInfo[];
  /** Voces edge-tts del idioma de salida (se cargan bajo demanda, con red). */
  edgeVoices: EdgeVoice[];
  loadingEdgeVoices: boolean;
  onLoadEdgeVoices: () => void;
  /** `true` si el segmento ya tiene doblaje generado. */
  hasDub: boolean;
  /** URL del doblaje ya en disco para este segmento, si lo hay. */
  existingDubUrl: string | null;
  /** Genera (o regenera) el doblaje del segmento y devuelve la URL para oírlo. */
  onGenerateSegment: () => Promise<string | null>;
}

function EditPanel({
  segment,
  onChange,
  engine,
  voice,
  onChangeEngine,
  onChangeVoice,
  piperVoices,
  edgeVoices,
  loadingEdgeVoices,
  onLoadEdgeVoices,
  hasDub,
  existingDubUrl,
  onGenerateSegment,
}: Props) {
  const [generating, setGenerating] = useState(false);
  const [dubUrl, setDubUrl] = useState<string | null>(null);
  const audioRef = useRef<HTMLAudioElement>(null);

  // Al cambiar de segmento, olvida el audio recién generado del anterior.
  useEffect(() => {
    setDubUrl(null);
  }, [segment?.id]);

  if (!segment) {
    return (
      <div className="edit edit--empty">
        Selecciona un segmento para editar.
      </div>
    );
  }

  const voces = engine === "piper" ? piperVoices : edgeVoices;
  const sinVoces = voces.length === 0;
  const sinTraduccion = segment.translation.trim().length === 0;

  async function generar() {
    setGenerating(true);
    try {
      const url = await onGenerateSegment();
      setDubUrl(url);
      // Espera a que el <audio> tome la nueva fuente antes de reproducir.
      requestAnimationFrame(() => audioRef.current?.play().catch(() => {}));
    } finally {
      setGenerating(false);
    }
  }

  return (
    <div className="edit">
      <div className="edit__header">Edición de segmento</div>

      <label className="edit__field">
        <span>Texto origen</span>
        <textarea
          value={segment.source}
          onChange={(e) => onChange(segment.id, { source: e.target.value })}
          rows={3}
        />
      </label>

      <label className="edit__field">
        <span>Traducción</span>
        <textarea
          value={segment.translation}
          onChange={(e) => onChange(segment.id, { translation: e.target.value })}
          rows={3}
          placeholder="Ingresa la traducción…"
        />
      </label>

      <div className="edit__voice">
        <div className="edit__field-title">Voz</div>
        <div className="edit__row">
          <select
            value={engine}
            onChange={(e) => {
              const motor = e.target.value as DubEngine;
              onChangeEngine(motor);
              if (motor === "edge-tts" && edgeVoices.length === 0) onLoadEdgeVoices();
            }}
          >
            <option value="piper">Piper — local, sin red</option>
            <option value="edge-tts">edge-tts — online (Microsoft)</option>
            <option value="xtts" disabled>
              XTTS-v2 — clonar voz (próximamente)
            </option>
          </select>
        </div>

        <div className="edit__row">
          <select
            value={voice}
            onChange={(e) => onChangeVoice(e.target.value)}
            disabled={sinVoces}
          >
            {sinVoces && (
              <option value="">
                {engine === "piper"
                  ? "Sin voces Piper descargadas"
                  : loadingEdgeVoices
                    ? "Cargando voces…"
                    : "Sin voces para este idioma"}
              </option>
            )}
            {engine === "piper" &&
              piperVoices.map((v) => (
                <option key={v.id} value={v.id}>
                  {v.label}
                </option>
              ))}
            {engine === "edge-tts" &&
              edgeVoices.map((v) => (
                <option key={v.short_name} value={v.short_name}>
                  {v.friendly_name || v.short_name}
                  {v.gender ? ` · ${v.gender}` : ""}
                </option>
              ))}
          </select>
          <button
            type="button"
            onClick={generar}
            disabled={generating || sinVoces || !voice || sinTraduccion}
          >
            {generating ? "Generando…" : hasDub ? "Regenerar" : "Generar audio"}
          </button>
        </div>

        {sinTraduccion && (
          <div className="edit__hint">Traduce el segmento antes de doblarlo.</div>
        )}
        {engine === "piper" && sinVoces && !sinTraduccion && (
          <div className="edit__hint">
            Descarga una voz Piper del idioma de salida desde «Modelos y voces».
          </div>
        )}
        <div className="edit__hint">
          Piper: privado, no sale de tu equipo (recomendado). edge-tts: más voces
          pero envía el texto a Microsoft. XTTS-v2 (clonación) llegará después.
        </div>

        {(dubUrl ?? existingDubUrl) && (
          <audio
            ref={audioRef}
            className="edit__audio"
            controls
            src={(dubUrl ?? existingDubUrl) ?? undefined}
          />
        )}
      </div>
    </div>
  );
}

export default EditPanel;
