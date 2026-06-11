import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ask, open, save, message } from "@tauri-apps/plugin-dialog";
import "./App.css";
import TopBar from "./components/TopBar";
import SegmentsList from "./components/SegmentsList";
import VideoPreview from "./components/VideoPreview";
import EditPanel from "./components/EditPanel";
import Transport from "./components/Transport";
import type { Project, Segment } from "./types";

function App() {
  const [project, setProject] = useState<Project | null>(null);
  const [segments, setSegments] = useState<Segment[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [extractingAudio, setExtractingAudio] = useState(false);

  const selected = segments.find((s) => s.id === selectedId) ?? null;

  function cargarProyecto(proyecto: Project) {
    setProject(proyecto);
    setSegments(proyecto.segments);
    setSelectedId(proyecto.segments[0]?.id ?? null);
  }

  async function nuevoProyecto() {
    const ruta = await save({
      title: "Nuevo proyecto",
      defaultPath: "proyecto.lqzx",
    });
    if (!ruta) return;
    const nombre =
      ruta.split(/[\\/]/).pop()?.replace(/\.lqzx$/, "") ?? "Proyecto";
    try {
      const proyecto = await invoke<Project>("crear_proyecto", {
        path: ruta,
        nombre,
        // Idiomas por defecto hasta que exista selector en la UI.
        idiomaOrigen: "es",
        idiomaDestino: "en",
      });
      cargarProyecto(proyecto);
    } catch (e) {
      await message(String(e), { title: "Nuevo proyecto", kind: "error" });
    }
  }

  async function abrirProyecto() {
    const ruta = await open({ title: "Abrir proyecto", directory: true });
    if (!ruta) return;
    try {
      const proyecto = await invoke<Project>("abrir_proyecto", { path: ruta });
      cargarProyecto(proyecto);
    } catch (e) {
      await message(String(e), { title: "Abrir proyecto", kind: "error" });
    }
  }

  async function importarVideo() {
    if (!project) return;
    const ruta = await open({
      title: "Importar video",
      filters: [
        { name: "Video", extensions: ["mp4", "mkv", "webm", "mov", "avi"] },
      ],
    });
    if (!ruta) return;
    // ADR-002: el video se copia o se referencia según preferencia del usuario.
    const copiar = await ask(
      "¿Copiar el video dentro del proyecto?\n\nCopiar: el proyecto queda autocontenido.\nReferenciar: se usa la ruta original sin duplicar el archivo.",
      {
        title: "Importar video",
        kind: "info",
        okLabel: "Copiar al proyecto",
        cancelLabel: "Solo referenciar",
      },
    );
    try {
      const proyecto = await invoke<Project>("importar_video", {
        path: project.path,
        video: ruta,
        copiar,
      });
      // Solo se actualiza el proyecto: los segmentos locales sin guardar se conservan.
      setProject(proyecto);
    } catch (e) {
      await message(String(e), { title: "Importar video", kind: "error" });
    }
  }

  async function extraerAudio() {
    if (!project?.video_path) return;
    setExtractingAudio(true);
    try {
      const proyecto = await invoke<Project>("extraer_audio", {
        path: project.path,
      });
      // Solo se actualiza el proyecto: los segmentos locales sin guardar se conservan.
      setProject(proyecto);
    } catch (e) {
      await message(String(e), { title: "Extraer audio", kind: "error" });
    } finally {
      setExtractingAudio(false);
    }
  }

  async function guardarProyecto() {
    if (!project) return;
    try {
      await invoke("guardar_segmentos", {
        path: project.path,
        segmentos: segments,
      });
    } catch (e) {
      await message(String(e), { title: "Guardar", kind: "error" });
    }
  }

  function actualizarSegmento(id: string, cambios: Partial<Segment>) {
    setSegments((prev) => prev.map((s) => (s.id === id ? { ...s, ...cambios } : s)));
  }

  return (
    <div className="app">
      <TopBar
        projectName={project?.manifest.name ?? "Sin proyecto"}
        canSave={project !== null}
        canExtractAudio={project?.video_path != null}
        extractingAudio={extractingAudio}
        hasAudio={project?.audio_path != null}
        onNew={nuevoProyecto}
        onOpen={abrirProyecto}
        onSave={guardarProyecto}
        onExtractAudio={extraerAudio}
      />
      <div className="app__body">
        <aside className="app__left">
          <SegmentsList
            segments={segments}
            selectedId={selectedId}
            onSelect={setSelectedId}
          />
        </aside>
        <main className="app__center">
          <VideoPreview
            videoPath={project?.video_path ?? null}
            hasProject={project !== null}
            onImport={importarVideo}
          />
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
