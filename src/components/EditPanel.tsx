import type { Segment } from "../types";

interface Props {
  segment: Segment | null;
  onChange: (id: string, cambios: Partial<Segment>) => void;
}

function EditPanel({ segment, onChange }: Props) {
  if (!segment) {
    return (
      <div className="edit edit--empty">
        Selecciona un segmento para editar.
      </div>
    );
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
          <select disabled defaultValue="edge-tts">
            <option value="edge-tts">edge-tts</option>
            <option value="xtts">XTTS-v2</option>
          </select>
          <button type="button" disabled>Generar audio</button>
        </div>
      </div>

      <div className="edit__runs">
        <div className="edit__field-title">Versiones</div>
        <div className="edit__hint">Sin generaciones aún.</div>
      </div>
    </div>
  );
}

export default EditPanel;
