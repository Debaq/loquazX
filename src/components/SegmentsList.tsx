import type { Segment } from "../types";

interface Props {
  segments: Segment[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

function formatoTiempo(segundos: number): string {
  const m = Math.floor(segundos / 60);
  const s = (segundos % 60).toFixed(1);
  return `${m}:${s.padStart(4, "0")}`;
}

function SegmentsList({ segments, selectedId, onSelect }: Props) {
  return (
    <div className="segments">
      <div className="segments__header">Segmentos ({segments.length})</div>
      <ul className="segments__list">
        {segments.map((s, i) => (
          <li
            key={s.id}
            className={`segments__item ${s.id === selectedId ? "is-selected" : ""}`}
            onClick={() => onSelect(s.id)}
          >
            <div className="segments__meta">
              <span className="segments__index">#{i + 1}</span>
              <span className="segments__time">
                {formatoTiempo(s.start)} → {formatoTiempo(s.end)}
              </span>
            </div>
            <div className="segments__text">{s.source}</div>
          </li>
        ))}
      </ul>
    </div>
  );
}

export default SegmentsList;
