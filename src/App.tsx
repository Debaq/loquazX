import { useState } from "react";
import "./App.css";
import TopBar from "./components/TopBar";
import SegmentsList from "./components/SegmentsList";
import VideoPreview from "./components/VideoPreview";
import EditPanel from "./components/EditPanel";
import Transport from "./components/Transport";
import type { Segment } from "./types";

const segmentosDemo: Segment[] = [
  { id: "s1", start: 0, end: 3.2, source: "Hola, bienvenidos al video.", translation: "" },
  { id: "s2", start: 3.2, end: 7.8, source: "Hoy hablaremos de loquazX.", translation: "" },
  { id: "s3", start: 7.8, end: 12.0, source: "Una herramienta de subtitulado.", translation: "" },
];

function App() {
  const [segments, setSegments] = useState<Segment[]>(segmentosDemo);
  const [selectedId, setSelectedId] = useState<string | null>(segmentosDemo[0]?.id ?? null);
  const [projectName] = useState<string>("Proyecto sin título");

  const selected = segments.find((s) => s.id === selectedId) ?? null;

  function actualizarSegmento(id: string, cambios: Partial<Segment>) {
    setSegments((prev) => prev.map((s) => (s.id === id ? { ...s, ...cambios } : s)));
  }

  return (
    <div className="app">
      <TopBar projectName={projectName} />
      <div className="app__body">
        <aside className="app__left">
          <SegmentsList
            segments={segments}
            selectedId={selectedId}
            onSelect={setSelectedId}
          />
        </aside>
        <main className="app__center">
          <VideoPreview />
        </main>
        <aside className="app__right">
          <EditPanel segment={selected} onChange={actualizarSegmento} />
        </aside>
      </div>
      <footer className="app__footer">
        <Transport />
      </footer>
    </div>
  );
}

export default App;
