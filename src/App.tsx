import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ask, open, save, message } from "@tauri-apps/plugin-dialog";
import "./App.css";
import TopBar from "./components/TopBar";
import SegmentsList from "./components/SegmentsList";
import VideoPreview from "./components/VideoPreview";
import EditPanel from "./components/EditPanel";
import Timeline from "./components/Timeline";
import ModelManager from "./components/ModelManager";
import type {
  Project,
  Segment,
  ModelInfo,
  ExportResult,
  ImportResult,
  VoiceInfo,
  EdgeVoice,
  DubEngine,
  DubResult,
  RenderReport,
  RecalibrationReport,
} from "./types";

const NIVEL_POR_DEFECTO = "base";
const ORIGEN_POR_DEFECTO = "es";
const DESTINO_POR_DEFECTO = "en";

function App() {
  const [project, setProject] = useState<Project | null>(null);
  // El <video> vive en VideoPreview; lo exponemos vía callback ref para que
  // Transport pueda controlarlo. Usar estado (no useRef) hace que Transport
  // re-suscriba sus listeners cuando el elemento se re-monta al cambiar de video.
  const [videoEl, setVideoEl] = useState<HTMLVideoElement | null>(null);
  const [segments, setSegments] = useState<Segment[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [extractingAudio, setExtractingAudio] = useState(false);
  const [transcribing, setTranscribing] = useState(false);
  // ADR-008: traducción con el motor local NLLB. `translateProgress` lleva el
  // avance segmento a segmento que emite el backend por `traduccion:progreso`.
  const [translating, setTranslating] = useState(false);
  const [translateProgress, setTranslateProgress] = useState<{
    done: number;
    total: number;
  } | null>(null);
  const [showModels, setShowModels] = useState(false);
  // ADR-007: nivel de modelo elegido; persiste entre sesiones.
  const [modelLevel, setModelLevel] = useState(
    () => localStorage.getItem("loquazx.whisperLevel") ?? NIVEL_POR_DEFECTO,
  );
  // Idioma de origen (el que usa whisper al transcribir) y destino. Persisten
  // como valores por defecto para el próximo proyecto; un proyecto abierto manda
  // sobre ellos al cargarse.
  const [sourceLanguage, setSourceLanguage] = useState(
    () => localStorage.getItem("loquazx.sourceLanguage") ?? ORIGEN_POR_DEFECTO,
  );
  const [targetLanguage, setTargetLanguage] = useState(
    () => localStorage.getItem("loquazx.targetLanguage") ?? DESTINO_POR_DEFECTO,
  );
  // ADR-009: doblaje. Voces Piper descargadas del idioma de salida y voces
  // edge-tts (cargadas bajo demanda, con red). El motor y la voz se comparten
  // entre la generación por segmento (EditPanel) y la masiva (Timeline).
  const [piperVoices, setPiperVoices] = useState<VoiceInfo[]>([]);
  const [edgeVoices, setEdgeVoices] = useState<EdgeVoice[]>([]);
  const [loadingEdgeVoices, setLoadingEdgeVoices] = useState(false);
  const [dubEngine, setDubEngine] = useState<DubEngine>(
    () => (localStorage.getItem("loquazx.dubEngine") as DubEngine) ?? "piper",
  );
  const [dubVoice, setDubVoice] = useState("");
  const [dubbing, setDubbing] = useState(false);
  const [dubProgress, setDubProgress] = useState<{ done: number; total: number } | null>(null);
  const [selectedDubUrl, setSelectedDubUrl] = useState<string | null>(null);
  // Cambia con cada generación (masiva o por segmento) para que la Timeline
  // recargue las ondas del doblaje, incluso al regenerar un segmento ya doblado.
  const [dubVersion, setDubVersion] = useState(0);
  // ADR-010: estado del render de la presentación (PDF + audio doblado).
  const [renderingPresentation, setRenderingPresentation] = useState(false);
  const [renderProgress, setRenderProgress] = useState<{ etapa: number; total: number } | null>(null);
  // ADR-010: planificación y restauración de tiempos (modo presentación).
  const [hasTimingsBackup, setHasTimingsBackup] = useState(false);
  const [inPlaceholder, setInPlaceholder] = useState(false);
  const [timingsWorking, setTimingsWorking] = useState(false);

  // Desactiva el zoom del webview (Ctrl+rueda, pellizco, Ctrl +/-/0): la app
  // no debe escalar como página web; el único zoom es el de la línea de tiempo.
  useEffect(() => {
    const sinZoomRueda = (e: WheelEvent) => {
      if (e.ctrlKey) e.preventDefault();
    };
    const sinZoomTecla = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && ["+", "-", "=", "0"].includes(e.key)) {
        e.preventDefault();
      }
    };
    const sinGesto = (e: Event) => e.preventDefault();
    window.addEventListener("wheel", sinZoomRueda, { passive: false, capture: true });
    window.addEventListener("keydown", sinZoomTecla);
    window.addEventListener("gesturestart", sinGesto);
    window.addEventListener("gesturechange", sinGesto);
    return () => {
      window.removeEventListener("wheel", sinZoomRueda, { capture: true });
      window.removeEventListener("keydown", sinZoomTecla);
      window.removeEventListener("gesturestart", sinGesto);
      window.removeEventListener("gesturechange", sinGesto);
    };
  }, []);

  function elegirNivel(nivel: string) {
    setModelLevel(nivel);
    localStorage.setItem("loquazx.whisperLevel", nivel);
  }

  // Voces Piper descargadas para el idioma de salida: se refrescan al cambiar de
  // idioma destino (y al volver del gestor de modelos, que puede haber bajado una).
  useEffect(() => {
    invoke<VoiceInfo[]>("listar_voces")
      .then((vs) =>
        setPiperVoices(vs.filter((v) => v.downloaded && v.language === targetLanguage)),
      )
      .catch(() => setPiperVoices([]));
  }, [targetLanguage, showModels]);

  // Al cambiar el motor o las voces disponibles, asegura que la voz elegida sea
  // válida (o queda vacía si no hay voces para el idioma).
  useEffect(() => {
    const ids =
      dubEngine === "piper"
        ? piperVoices.map((v) => v.id)
        : edgeVoices.map((v) => v.short_name);
    setDubVoice((actual) => (ids.includes(actual) ? actual : ids[0] ?? ""));
  }, [dubEngine, piperVoices, edgeVoices]);

  // URL local del doblaje ya generado del segmento seleccionado, para oírlo sin
  // regenerar (ruta determinista en `runs/dub/`).
  useEffect(() => {
    setSelectedDubUrl(null);
    if (!project || !selectedId || !project.dubs.includes(selectedId)) return;
    const wav = `${project.path}/runs/dub/${selectedId}.wav`;
    invoke<string>("url_media", { path: wav })
      .then(setSelectedDubUrl)
      .catch(() => setSelectedDubUrl(null));
  }, [project, selectedId]);

  async function cargarVocesEdge() {
    setLoadingEdgeVoices(true);
    try {
      const vs = await invoke<EdgeVoice[]>("listar_voces_edge");
      setEdgeVoices(vs.filter((v) => v.language === targetLanguage));
    } catch (e) {
      await message(String(e), { title: "Voces edge-tts", kind: "error" });
    } finally {
      setLoadingEdgeVoices(false);
    }
  }

  function elegirMotorDoblaje(motor: DubEngine) {
    setDubEngine(motor);
    localStorage.setItem("loquazx.dubEngine", motor);
  }

  // Genera el doblaje del segmento seleccionado y devuelve su URL para oírlo.
  // Guarda primero los segmentos: la síntesis lee la traducción desde disco.
  async function generarDoblajeSegmento(): Promise<string | null> {
    if (!project || !selectedId || !dubVoice) return null;
    try {
      await guardarProyecto();
      const url = await invoke<string>("generar_doblaje_segmento", {
        path: project.path,
        segmento: selectedId,
        ajustes: { engine: dubEngine, voice: dubVoice },
      });
      setProject((p) =>
        p
          ? { ...p, dubs: p.dubs.includes(selectedId) ? p.dubs : [...p.dubs, selectedId] }
          : p,
      );
      setDubVersion((v) => v + 1);
      return url;
    } catch (e) {
      await message(String(e), { title: "Generar doblaje", kind: "error" });
      return null;
    }
  }

  // Dobla todos los segmentos traducidos del proyecto con el motor/voz elegidos.
  async function generarDoblaje() {
    if (!project || !dubVoice) return;
    setDubbing(true);
    setDubProgress({ done: 0, total: segments.length });
    const desuscribir = await listen<{ generados: number; total: number }>(
      "doblaje:progreso",
      (e) => setDubProgress({ done: e.payload.generados, total: e.payload.total }),
    );
    try {
      await guardarProyecto();
      const resultado = await invoke<DubResult>("generar_doblaje", {
        path: project.path,
        ajustes: { engine: dubEngine, voice: dubVoice },
      });
      cargarProyecto(resultado.project);
      setDubVersion((v) => v + 1);
      await message(
        `Doblados: ${resultado.report.generated}\nSin traducción: ${resultado.report.skipped}`,
        { title: "Generar doblaje", kind: "info" },
      );
    } catch (e) {
      await message(String(e), { title: "Generar doblaje", kind: "error" });
    } finally {
      desuscribir();
      setDubbing(false);
      setDubProgress(null);
    }
  }

  const selected = segments.find((s) => s.id === selectedId) ?? null;

  // Seleccionar un segmento también mueve el cursor del video a su inicio,
  // conectando la lista y la línea de tiempo con la reproducción.
  function seleccionarSegmento(id: string) {
    setSelectedId(id);
    const segmento = segments.find((s) => s.id === id);
    if (segmento && videoEl) videoEl.currentTime = segmento.start;
  }

  function cargarProyecto(proyecto: Project) {
    setProject(proyecto);
    setSegments(proyecto.segments);
    setSelectedId(proyecto.segments[0]?.id ?? null);
    // Los selectores reflejan los idiomas del proyecto abierto.
    setSourceLanguage(proyecto.manifest.source_language);
    setTargetLanguage(proyecto.manifest.target_language);
    // El botón «Restaurar» aparece sólo si hay backup de timings.
    void invoke<boolean>("tiene_backup_timings", { path: proyecto.path })
      .then(setHasTimingsBackup)
      .catch(() => setHasTimingsBackup(false));
    // Lee el modo de planificación directamente de `segments.json` para
    // mostrar el botón «Aplicar tiempos» sólo cuando aplica.
    void invoke<{
      segments: Segment[];
      timing_mode: string | null;
    }>("leer_segments_con_timing", { path: proyecto.path })
      .then((data) => setInPlaceholder(data.timing_mode === "placeholder"))
      .catch(() => setInPlaceholder(false));
  }

  // Cambia los idiomas: si hay proyecto, persiste en su manifiesto (el de origen
  // lo usa whisper al transcribir); además quedan como valores por defecto.
  async function cambiarIdiomas(origen: string, destino: string) {
    setSourceLanguage(origen);
    setTargetLanguage(destino);
    localStorage.setItem("loquazx.sourceLanguage", origen);
    localStorage.setItem("loquazx.targetLanguage", destino);
    if (!project) return;
    try {
      const proyecto = await invoke<Project>("cambiar_idiomas", {
        path: project.path,
        idiomaOrigen: origen,
        idiomaDestino: destino,
      });
      setProject(proyecto);
    } catch (e) {
      await message(String(e), { title: "Cambiar idiomas", kind: "error" });
    }
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
        idiomaOrigen: sourceLanguage,
        idiomaDestino: targetLanguage,
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
      title: "Importar video o PDF",
      filters: [
        { name: "Video", extensions: ["mp4", "mkv", "webm", "mov", "avi"] },
        { name: "PDF", extensions: ["pdf"] },
      ],
    });
    if (!ruta) return;
    try {
      const proyecto = await importarFuente(ruta);
      setProject(proyecto);
    } catch (e) {
      await message(String(e), { title: "Importar", kind: "error" });
    }
  }

  // Despacha al backend correcto según la extensión del archivo seleccionado
  // (ADR-002 para video, ADR-010 para PDF). Devuelve el `Project` actualizado.
  async function importarFuente(ruta: string): Promise<Project> {
    if (!project) throw new Error("No hay proyecto abierto.");
    const extension = ruta.split(".").pop()?.toLowerCase() ?? "";
    if (extension === "pdf") {
      return importarPdf(ruta);
    }
    return importarVideoSeleccionado(ruta);
  }

  async function importarVideoSeleccionado(ruta: string): Promise<Project> {
    if (!project) throw new Error("No hay proyecto abierto.");
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
    return await invoke<Project>("importar_video", {
      path: project.path,
      video: ruta,
      copiar,
    });
  }

  async function importarPdf(ruta: string): Promise<Project> {
    if (!project) throw new Error("No hay proyecto abierto.");
    const pageCount = await invoke<number>("conteo_paginas_pdf", { pdf: ruta });
    const continuar = await ask(
      `El PDF tiene ${pageCount} páginas. Se copiará al proyecto.\n\n¿Continuar?`,
      { title: "Importar PDF", kind: "info" },
    );
    if (!continuar) {
      throw new Error("Importación cancelada.");
    }
    return await invoke<Project>("importar_pdf", {
      path: project.path,
      pdf: ruta,
    });
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

  async function transcribir() {
    if (!project?.audio_path) return;
    // ADR-004: la transcripción reemplaza los segmentos existentes.
    if (segments.length > 0) {
      const continuar = await ask(
        "Transcribir reemplazará todos los segmentos actuales.\n\n¿Continuar?",
        { title: "Transcribir", kind: "warning" },
      );
      if (!continuar) return;
    }
    // ADR-007: el modelo se descarga y se guarda; si el nivel elegido no está
    // disponible, abrimos el gestor en vez de fallar al transcribir.
    const modelos = await invoke<ModelInfo[]>("listar_modelos");
    const disponible = modelos.find((m) => m.id === modelLevel)?.downloaded ?? false;
    if (!disponible) {
      await message(
        `El modelo «${modelLevel}» no está descargado. Descárgalo en «Modelo».`,
        { title: "Transcribir", kind: "warning" },
      );
      setShowModels(true);
      return;
    }
    setTranscribing(true);
    try {
      const proyecto = await invoke<Project>("transcribir", {
        path: project.path,
        nivel: modelLevel,
      });
      cargarProyecto(proyecto);
    } catch (e) {
      await message(String(e), { title: "Transcribir", kind: "error" });
    } finally {
      setTranscribing(false);
    }
  }

  async function exportarTraduccion() {
    if (!project) return;
    // ADR-006: la exportación parte de segments.json en disco; persiste lo editado.
    try {
      await invoke("guardar_segmentos", {
        path: project.path,
        segmentos: segments,
      });
      const resultado = await invoke<ExportResult>("exportar_traduccion", {
        path: project.path,
      });
      await message(
        `Se exportaron ${resultado.segment_count} segmentos.\n\n` +
          `Solicitud: ${resultado.request_file}\n` +
          `Prompt: ${resultado.prompt_file}\n\n` +
          "Pega el prompt y el JSON en un LLM e importa el JSON de respuesta.",
        { title: "Exportar para traducir", kind: "info" },
      );
    } catch (e) {
      await message(String(e), { title: "Exportar para traducir", kind: "error" });
    }
  }

  async function importarTraduccion() {
    if (!project) return;
    const ruta = await open({
      title: "Importar traducción (JSON de respuesta)",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!ruta) return;
    // El merge parte de segments.json en disco; persiste lo editado antes.
    try {
      await invoke("guardar_segmentos", {
        path: project.path,
        segmentos: segments,
      });
      const resultado = await invoke<ImportResult>("importar_traduccion", {
        path: project.path,
        respuesta: ruta,
      });
      cargarProyecto(resultado.project);
      const { translated, missing, unknown } = resultado.report;
      await message(
        `Traducidos: ${translated}\nSin traducción: ${missing}\n` +
          `Ids ignorados (no existen en el proyecto): ${unknown}`,
        { title: "Importar traducción", kind: "info" },
      );
    } catch (e) {
      await message(String(e), { title: "Importar traducción", kind: "error" });
    }
  }

  async function traducirLocal() {
    if (!project) return;
    // ADR-008: traduce sin red con el modelo NLLB. Si no está descargado, abre el
    // gestor en vez de fallar, igual que el flujo de transcripción con whisper.
    const motores = await invoke<ModelInfo[]>("listar_motores_traduccion");
    const listo = motores[0]?.downloaded ?? false;
    if (!listo) {
      await message(
        "El modelo de traducción no está descargado. Descárgalo en «Modelos y voces».",
        { title: "Traducir con IA local", kind: "warning" },
      );
      setShowModels(true);
      return;
    }
    // El motor parte de segments.json en disco; persiste lo editado antes.
    try {
      await invoke("guardar_segmentos", {
        path: project.path,
        segmentos: segments,
      });
    } catch (e) {
      await message(String(e), { title: "Traducir con IA local", kind: "error" });
      return;
    }
    setTranslating(true);
    setTranslateProgress({ done: 0, total: segments.length });
    const desuscribir = await listen<{ traducidos: number; total: number }>(
      "traduccion:progreso",
      (e) =>
        setTranslateProgress({ done: e.payload.traducidos, total: e.payload.total }),
    );
    try {
      const resultado = await invoke<ImportResult>("traducir_local", {
        path: project.path,
      });
      cargarProyecto(resultado.project);
      const { translated, missing } = resultado.report;
      await message(
        `Traducidos: ${translated}\nSin traducción: ${missing}`,
        { title: "Traducir con IA local", kind: "info" },
      );
    } catch (e) {
      await message(String(e), { title: "Traducir con IA local", kind: "error" });
    } finally {
      desuscribir();
      setTranslating(false);
      setTranslateProgress(null);
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

  // ADR-010: importa un audio arbitrario cuando el proyecto no tiene video.
  async function importarAudioPresentacion() {
    if (!project) return;
    if (project.audio_path) {
      await message(
        "El proyecto ya tiene audio extraído. Reimporta el video o el PDF para reemplazarlo.",
        { title: "Importar audio", kind: "warning" },
      );
      return;
    }
    const ruta = await open({
      title: "Importar audio",
      filters: [
        { name: "Audio", extensions: ["wav", "mp3", "m4a", "ogg", "flac"] },
      ],
    });
    if (!ruta) return;
    try {
      const proyecto = await invoke<Project>("importar_audio_presentacion", {
        path: project.path,
        audio: ruta,
      });
      setProject(proyecto);
    } catch (e) {
      await message(String(e), { title: "Importar audio", kind: "error" });
    }
  }

  // ADR-010: importa segmentos desde un JSON externo. Sobrescribe los actuales
  // previa confirmación.
  async function importarSegmentosJson() {
    if (!project) return;
    const ruta = await open({
      title: "Importar segmentos (JSON)",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!ruta) return;
    if (segments.length > 0) {
      const continuar = await ask(
        "Esto reemplazará todos los segmentos actuales. ¿Continuar?",
        { title: "Importar segmentos", kind: "warning" },
      );
      if (!continuar) return;
    }
    try {
      const proyecto = await invoke<Project>("importar_segmentos_json", {
        path: project.path,
        json: ruta,
      });
      cargarProyecto(proyecto);
    } catch (e) {
      await message(String(e), { title: "Importar segmentos", kind: "error" });
    }
  }

  // ADR-010: renderiza el video de presentación y deja el mp4 en `exports/`.
  // Auto-dobla los segmentos traducidos que aún no tengan WAV usando el
  // motor y la voz configurados, así el usuario no tiene que pasar por
  // «Generar todas» en la Timeline antes de exportar.
  async function renderizarPresentacion() {
    if (!project) return;
    setRenderingPresentation(true);
    setRenderProgress({ etapa: 0, total: 2 });
    const desuscribir = await listen<{ etapa: number; total: number }>(
      "presentacion:progreso",
      (e) => setRenderProgress({ etapa: e.payload.etapa, total: e.payload.total }),
    );
    try {
      await guardarProyecto();
      const reporte = await invoke<RenderReport>("renderizar_presentacion", {
        path: project.path,
        ajustes: { engine: dubEngine, voice: dubVoice },
      });
      await message(
        `Video generado en ${reporte.output}\nDuración: ${reporte.duration_secs.toFixed(1)} s`,
        { title: "Exportar video", kind: "info" },
      );
    } catch (e) {
      await message(String(e), { title: "Exportar video", kind: "error" });
    } finally {
      desuscribir();
      setRenderingPresentation(false);
      setRenderProgress(null);
    }
  }

  // ADR-010: aplica manualmente las duraciones reales de los WAV a los
// `start`/`end` de los segmentos. Es un atajo al auto-trigger que ya
// dispara `generate_dub` al terminar; aquí lo exponemos como botón
// explícito por si el usuario regenera audios por su cuenta y quiere
// forzar la recalibración.
async function aplicarTiempos() {
    if (!project) return;
    setTimingsWorking(true);
    try {
      const reporte = await invoke<RecalibrationReport>("aplicar_tiempos_reales", {
        path: project.path,
      });
      const proyecto = await invoke<Project>("abrir_proyecto", { path: project.path });
      cargarProyecto(proyecto);
      await message(
        `Tiempos aplicados: ${reporte.recalibrated} segmento(s) recalibrado(s), ${reporte.kept} silencio(s).`,
        { title: "Aplicar tiempos", kind: "info" },
      );
    } catch (e) {
      await message(String(e), { title: "Aplicar tiempos", kind: "error" });
    } finally {
      setTimingsWorking(false);
    }
  }

  // ADR-010: planificación de tiempos. Pone cada segmento a 2 s secuencial
  // para que el doblaje posterior se haga a velocidad natural y los tiempos
  // reales se apliquen al terminar. Crea un backup único la primera vez;
  // confirma antes si el proyecto ya tiene un cálculo previo o si el
  // usuario invirtió tiempo en los `start`/`end` actuales.
  async function eliminarTiempos() {
    if (!project) return;
    if (project.slides_path == null) {
      await message(
        "Importa un PDF antes de planificar los tiempos.",
        { title: "Eliminar tiempos", kind: "warning" },
      );
      return;
    }
    if (segments.length === 0) {
      await message(
        "No hay segmentos para planificar.",
        { title: "Eliminar tiempos", kind: "warning" },
      );
      return;
    }
    const aviso = hasTimingsBackup
      ? "Esto reemplazará los tiempos actuales por slots de 2 s. Ya hay un backup, así que «Restaurar» seguirá llevando al estado previo. ¿Continuar?"
      : "Esto reemplazará los tiempos actuales por slots de 2 s para que los audios se doblen en orden natural. Se creará un backup para poder restaurar después. ¿Continuar?";
    const continuar = await ask(aviso, {
      title: "Eliminar tiempos",
      kind: "warning",
    });
    if (!continuar) return;
    setTimingsWorking(true);
    try {
      await guardarProyecto();
      const proyecto = await invoke<Project>("planificar_tiempos_presentacion", {
        path: project.path,
        duracionSeg: 2.0,
      });
      cargarProyecto(proyecto);
      await message(
        `Tiempos planificados a 2 s por segmento (${segments.length} en total). ` +
          "Genera los audios (botón «Generar todas» o por segmento desde el panel). " +
          "Al terminar, las duraciones reales se aplican automáticamente, " +
          "o puedes pulsar «Aplicar tiempos» para forzarlo.",
        { title: "Eliminar tiempos", kind: "info" },
      );
    } catch (e) {
      await message(String(e), { title: "Eliminar tiempos", kind: "error" });
    } finally {
      setTimingsWorking(false);
    }
  }

  // ADR-010: restauración de tiempos. Revierte `segments.json` al estado
  // guardado en `timings.original.json` y borra el backup. Falla con un
  // mensaje claro si el backup ya no existe (botón deshabilitado en ese
  // caso, pero el usuario podría haber borrado el archivo a mano).
  async function restaurarTiempos() {
    if (!project) return;
    const continuar = await ask(
      "Esto restaura los `start`/`end` originales de los segmentos y borra el backup. ¿Continuar?",
      { title: "Restaurar tiempos", kind: "warning" },
    );
    if (!continuar) return;
    setTimingsWorking(true);
    try {
      const proyecto = await invoke<Project>("restaurar_timings_originales", {
        path: project.path,
      });
      cargarProyecto(proyecto);
      await message(
        "Tiempos originales restaurados.",
        { title: "Restaurar tiempos", kind: "info" },
      );
    } catch (e) {
      await message(String(e), { title: "Restaurar tiempos", kind: "error" });
      // Si falló por backup perdido, refrescamos el flag para ocultar el botón.
      void invoke<boolean>("tiene_backup_timings", { path: project.path })
        .then(setHasTimingsBackup)
        .catch(() => setHasTimingsBackup(false));
    } finally {
      setTimingsWorking(false);
    }
  }

  // ADR-010: regenera las imágenes del PDF a partir del PDF persistido.
  // Útil cuando la auto-recuperación del `open` no aplicó (PDF perdido,
  // error al importar) y el usuario no quiere reimportar todavía.
  async function regenerarImagenesPdf() {
    if (!project) return;
    try {
      const proyecto = await invoke<Project>("regenerar_imagenes_pdf", {
        path: project.path,
      });
      setProject(proyecto);
    } catch (e) {
      await message(String(e), { title: "Regenerar imágenes", kind: "error" });
    }
  }

  return (
    <div className="app">
      <TopBar
        projectName={project?.manifest.name ?? "Sin proyecto"}
        canSave={project !== null}
        canImportVideo={project !== null}
        canExtractAudio={project?.video_path != null}
        extractingAudio={extractingAudio}
        hasAudio={project?.audio_path != null}
        transcribing={transcribing}
        hasSegments={segments.length > 0}
        translating={translating}
        translateProgress={translateProgress}
        modelLevel={modelLevel}
        sourceLanguage={sourceLanguage}
        targetLanguage={targetLanguage}
        onChangeLanguages={cambiarIdiomas}
        onNew={nuevoProyecto}
        onOpen={abrirProyecto}
        onSave={guardarProyecto}
        onImportVideo={importarVideo}
        onExtractAudio={extraerAudio}
        onTranscribe={transcribir}
        onExportTranslation={exportarTraduccion}
        onImportTranslation={importarTraduccion}
        onTranslateLocal={traducirLocal}
        onOpenModels={() => setShowModels(true)}
        onImportAudioPresentation={importarAudioPresentacion}
        onImportSegmentsJson={importarSegmentosJson}
        onExportPresentation={renderizarPresentacion}
        canExportPresentation={
          project?.slides_path != null &&
          segments.some(
            (s) =>
              s.translation.trim().length > 0 ||
              s.source.trim().length > 0,
          ) &&
          dubVoice !== ""
        }
        segmentsToDubCount={
          segments.filter(
            (s) =>
              (s.translation.trim().length > 0 ||
                s.source.trim().length > 0) &&
              !(project?.dubs.includes(s.id) ?? false),
          ).length
        }
        renderingPresentation={renderingPresentation}
        renderProgress={renderProgress}
        onEliminarTiempos={eliminarTiempos}
        onAplicarTiempos={aplicarTiempos}
        onRestaurarTiempos={restaurarTiempos}
        hasTimingsBackup={hasTimingsBackup}
        inPlaceholder={inPlaceholder}
        timingsWorking={timingsWorking}
      />
      <div className="app__body">
        <aside className="app__left">
          <SegmentsList
            segments={segments}
            selectedId={selectedId}
            onSelect={seleccionarSegmento}
          />
        </aside>
        <main className="app__center">
          <VideoPreview
            videoPath={project?.video_path ?? null}
            hasProject={project !== null}
            videoRef={setVideoEl}
            projectPath={project?.path ?? null}
            slidesPath={project?.slides_path ?? null}
            slidesPageCount={project?.slides_page_count ?? null}
            segments={segments}
            selectedId={selectedId}
            onRegenerarSlides={regenerarImagenesPdf}
          />
        </main>
        <aside className="app__right">
          <EditPanel
            segment={selected}
            onChange={actualizarSegmento}
            engine={dubEngine}
            voice={dubVoice}
            onChangeEngine={elegirMotorDoblaje}
            onChangeVoice={setDubVoice}
            piperVoices={piperVoices}
            edgeVoices={edgeVoices}
            loadingEdgeVoices={loadingEdgeVoices}
            onLoadEdgeVoices={cargarVocesEdge}
            hasDub={selectedId != null && (project?.dubs.includes(selectedId) ?? false)}
            existingDubUrl={selectedDubUrl}
            onGenerateSegment={generarDoblajeSegmento}
            slidesPageCount={project?.slides_page_count ?? null}
            targetLanguage={targetLanguage}
          />
        </aside>
      </div>
      <footer className="app__footer">
        <Timeline
          video={videoEl}
          segments={segments}
          selectedId={selectedId}
          onSelect={seleccionarSegmento}
          audioPath={project?.audio_path ?? null}
          outputLanguages={[targetLanguage]}
          projectPath={project?.path ?? null}
          dubs={project?.dubs ?? []}
          dubVersion={dubVersion}
          canDub={
            segments.some(
              (s) =>
                s.translation.trim().length > 0 ||
                s.source.trim().length > 0,
            ) && dubVoice !== ""
          }
          dubbing={dubbing}
          dubProgress={dubProgress}
          onGenerateDub={generarDoblaje}
        />
      </footer>
      {showModels && (
        <ModelManager
          selectedLevel={modelLevel}
          onSelectLevel={elegirNivel}
          onClose={() => setShowModels(false)}
        />
      )}
    </div>
  );
}

export default App;
