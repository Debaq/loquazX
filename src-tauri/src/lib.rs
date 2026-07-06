mod audio;
mod download;
mod media_server;
mod models;
mod presentacion;
mod project;
mod transcribe;
mod translate_engine;
mod translation;
mod tts;
mod tts_edge;
mod voices;

use project::{ExportResult, ImportResult, Project, RecalibrationReport, Segment};
use std::path::PathBuf;
use tauri::{Emitter, Manager};

// ADR-007: los modelos viven en el directorio de datos de la app, bajo `models/`.
fn models_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| format!("No se pudo resolver el directorio de datos: {e}"))?
        .join("models"))
}

#[tauri::command]
fn crear_proyecto(
    path: String,
    nombre: String,
    idioma_origen: String,
    idioma_destino: String,
) -> Result<Project, String> {
    project::create(
        &PathBuf::from(path),
        &nombre,
        &idioma_origen,
        &idioma_destino,
    )
}

#[tauri::command]
async fn abrir_proyecto(path: String) -> Result<Project, String> {
    // Asíncrono: la auto-recuperación de páginas del PDF (ADR-010) puede
    // tardar varios segundos en PDFs grandes y no debe congelar la UI.
    tauri::async_runtime::spawn_blocking(move || project::open(&PathBuf::from(path)))
        .await
        .map_err(|e| format!("La apertura del proyecto se interrumpió: {e}"))?
}

#[tauri::command]
fn guardar_segmentos(path: String, segmentos: Vec<Segment>) -> Result<(), String> {
    project::save_segments(&PathBuf::from(path), segmentos)
}

#[tauri::command]
fn importar_video(path: String, video: String, copiar: bool) -> Result<Project, String> {
    project::import_video(&PathBuf::from(path), &PathBuf::from(video), copiar)
}

// Cambia los idiomas del proyecto; el de origen lo usa whisper al transcribir.
#[tauri::command]
fn cambiar_idiomas(
    path: String,
    idioma_origen: String,
    idioma_destino: String,
) -> Result<Project, String> {
    project::set_languages(&PathBuf::from(path), &idioma_origen, &idioma_destino)
}

// Asíncrono: ffmpeg puede tardar y no debe congelar el hilo principal.
#[tauri::command]
async fn extraer_audio(path: String) -> Result<Project, String> {
    tauri::async_runtime::spawn_blocking(move || project::extract_audio(&PathBuf::from(path)))
        .await
        .map_err(|e| format!("La extracción de audio se interrumpió: {e}"))?
}

// Calcula la envolvente de amplitud del audio extraído para la pista de onda
// de la línea de tiempo. Asíncrono: leer el WAV completo puede tardar en clips
// largos y no debe congelar el hilo principal.
#[tauri::command]
async fn forma_onda(path: String, buckets: usize) -> Result<audio::Waveform, String> {
    tauri::async_runtime::spawn_blocking(move || {
        audio::waveform(std::path::Path::new(&path), buckets)
    })
    .await
    .map_err(|e| format!("El cálculo de la onda se interrumpió: {e}"))?
}

// ADR-005: el media se sirve por HTTP local; WebKitGTK no enruta los elementos
// de media por el protocolo asset.
#[tauri::command]
fn url_media(
    server: tauri::State<media_server::MediaServer>,
    path: String,
) -> Result<String, String> {
    let p = std::path::Path::new(&path);
    eprintln!("[loquazX] url_media: {p:?}");
    let result = server.url_for(p);
    if let Err(ref e) = result {
        eprintln!("[loquazX] url_media falló: {e}");
    }
    result
}

/// Devuelve la URL local de una página rasterizada del PDF (ADR-010). Arma el
/// path internamente a partir del proyecto y el número de página para que el
/// frontend no tenga que componerlo a mano (antes fallaba con "no existe el
/// fichero" porque el path construido en JS no coincidía con el real).
#[tauri::command]
fn url_slide(
    server: tauri::State<media_server::MediaServer>,
    project_path: String,
    page: u32,
) -> Result<String, String> {
    let png = std::path::Path::new(&project_path)
        .join("slides")
        .join("pages")
        .join(format!("page-{page}.png"));
    eprintln!("[loquazX] url_slide: {png:?}");
    let result = server.url_for(&png);
    if let Err(ref e) = result {
        eprintln!("[loquazX] url_slide falló: {e}");
    }
    result
}

// ADR-007: lista los niveles de modelo y su estado de descarga.
#[tauri::command]
fn listar_modelos(app: tauri::AppHandle) -> Result<Vec<models::ModelInfo>, String> {
    Ok(models::list(&models_dir(&app)?))
}

#[derive(Clone, serde::Serialize)]
struct ProgresoDescarga {
    nivel: String,
    descargado: u64,
    total: u64,
}

// Asíncrono: la descarga puede pesar varios GB; emite `modelo:progreso` a la UI.
#[tauri::command]
async fn descargar_modelo(
    app: tauri::AppHandle,
    window: tauri::Window,
    nivel: String,
) -> Result<models::ModelInfo, String> {
    let dir = models_dir(&app)?;
    let nivel_evento = nivel.clone();
    tauri::async_runtime::spawn_blocking(move || {
        models::download(&dir, &nivel, |descargado, total| {
            let _ = window.emit(
                "modelo:progreso",
                ProgresoDescarga {
                    nivel: nivel_evento.clone(),
                    descargado,
                    total,
                },
            );
        })
    })
    .await
    .map_err(|e| format!("La descarga se interrumpió: {e}"))?
}

// ADR-007: fallback sin red, copia un `.bin` propio al almacén.
#[tauri::command]
fn importar_modelo(
    app: tauri::AppHandle,
    nivel: String,
    archivo: String,
) -> Result<models::ModelInfo, String> {
    models::import(&models_dir(&app)?, &nivel, &PathBuf::from(archivo))
}

#[tauri::command]
fn eliminar_modelo(app: tauri::AppHandle, nivel: String) -> Result<(), String> {
    models::delete(&models_dir(&app)?, &nivel)
}

// ADR-009: lista las voces Piper y su estado de descarga. Las voces viven en el
// mismo almacén `models/` que whisper, con nombres propios (`<id>.onnx`).
#[tauri::command]
fn listar_voces(app: tauri::AppHandle) -> Result<Vec<voices::VoiceInfo>, String> {
    Ok(voices::list(&models_dir(&app)?))
}

// Asíncrono: la voz pesa decenas de MB; emite `voz:progreso` a la UI (reusa la
// forma de `ProgresoDescarga`, con `nivel` = id de la voz).
#[tauri::command]
async fn descargar_voz(
    app: tauri::AppHandle,
    window: tauri::Window,
    voz: String,
) -> Result<voices::VoiceInfo, String> {
    let dir = models_dir(&app)?;
    let voz_evento = voz.clone();
    tauri::async_runtime::spawn_blocking(move || {
        voices::download(&dir, &voz, |descargado, total| {
            let _ = window.emit(
                "voz:progreso",
                ProgresoDescarga {
                    nivel: voz_evento.clone(),
                    descargado,
                    total,
                },
            );
        })
    })
    .await
    .map_err(|e| format!("La descarga de la voz se interrumpió: {e}"))?
}

#[tauri::command]
fn eliminar_voz(app: tauri::AppHandle, voz: String) -> Result<(), String> {
    voices::delete(&models_dir(&app)?, &voz)
}

// Directorio de previsualizaciones de audio (TTS): efímero, bajo la cache de la app.
fn preview_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("No se pudo resolver la cache de la app: {e}"))?
        .join("preview"))
}

// ADR-009: lista las voces edge-tts (online). Asíncrono: consulta el endpoint de
// Microsoft por red.
#[tauri::command]
async fn listar_voces_edge() -> Result<Vec<tts_edge::EdgeVoice>, String> {
    tauri::async_runtime::spawn_blocking(tts_edge::list)
        .await
        .map_err(|e| format!("El listado de voces edge-tts se interrumpió: {e}"))?
}

// ADR-009: sintetiza una muestra con la voz edge-tts elegida y devuelve su URL
// local para reproducirla (ADR-005). Asíncrono y con red (vía Microsoft).
#[tauri::command]
async fn probar_voz_edge(
    app: tauri::AppHandle,
    server: tauri::State<'_, media_server::MediaServer>,
    voz: String,
    texto: String,
) -> Result<String, String> {
    let dir = preview_dir(&app)?;
    // El nombre lleva un uuid para que el webview no reproduzca un audio cacheado.
    let archivo = dir.join(format!("edge-{}.mp3", uuid::Uuid::new_v4().simple()));
    let archivo_synth = archivo.clone();
    tauri::async_runtime::spawn_blocking(move || {
        tts_edge::synthesize(&voz, &texto, &archivo_synth)
    })
    .await
    .map_err(|e| format!("La previsualización edge-tts se interrumpió: {e}"))??;
    server.url_for(&archivo)
}

// Asíncrono: whisper puede tardar minutos y no debe congelar el hilo principal.
// ADR-007: la UI pasa el nivel; el backend resuelve el modelo guardado.
#[tauri::command]
async fn transcribir(
    app: tauri::AppHandle,
    path: String,
    nivel: String,
) -> Result<Project, String> {
    let modelo = models::model_path(&models_dir(&app)?, &nivel)?;
    if !modelo.is_file() {
        return Err(format!(
            "El modelo «{nivel}» no está descargado. Descárgalo desde «Modelo» primero."
        ));
    }
    tauri::async_runtime::spawn_blocking(move || project::transcribe(&PathBuf::from(path), &modelo))
        .await
        .map_err(|e| format!("La transcripción se interrumpió: {e}"))?
}

// ADR-006: la app no traduce; exporta la solicitud y el prompt para un LLM externo.
#[tauri::command]
fn exportar_traduccion(path: String) -> Result<ExportResult, String> {
    project::export_translation(&PathBuf::from(path))
}

#[tauri::command]
fn importar_traduccion(path: String, respuesta: String) -> Result<ImportResult, String> {
    project::import_translation(&PathBuf::from(path), &PathBuf::from(respuesta))
}

// ADR-008 (revisado): lista el estado del motor de traducción local (NLLB/ONNX).
#[tauri::command]
fn listar_motores_traduccion(
    app: tauri::AppHandle,
) -> Result<Vec<translate_engine::EngineInfo>, String> {
    Ok(translate_engine::list(&models_dir(&app)?))
}

// ADR-008 (revisado): descarga el modelo NLLB (varios archivos ONNX + tokenizer).
// Asíncrono: pesa ~900 MB; emite `modelo-traduccion:progreso` a la UI (reusa la
// forma de `ProgresoDescarga`, con `nivel` = id del modelo).
#[tauri::command]
async fn descargar_motor_traduccion(
    app: tauri::AppHandle,
    window: tauri::Window,
) -> Result<translate_engine::EngineInfo, String> {
    let dir = models_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        translate_engine::download(&dir, |descargado, total| {
            let _ = window.emit(
                "modelo-traduccion:progreso",
                ProgresoDescarga {
                    nivel: translate_engine::MODEL_DIR.to_string(),
                    descargado,
                    total,
                },
            );
        })
    })
    .await
    .map_err(|e| format!("La descarga del modelo de traducción se interrumpió: {e}"))?
}

#[derive(Clone, serde::Serialize)]
struct ProgresoTraduccion {
    traducidos: usize,
    total: usize,
}

// ADR-008 (revisado): traduce los segmentos con el motor local (sin red).
// Asíncrono: la inferencia puede tardar; emite `traduccion:progreso` a la UI.
#[tauri::command]
async fn traducir_local(
    app: tauri::AppHandle,
    window: tauri::Window,
    path: String,
) -> Result<ImportResult, String> {
    let dir = models_dir(&app)?;
    if !translate_engine::is_downloaded(&dir) {
        return Err(
            "El modelo de traducción no está descargado. Descárgalo desde «Modelos y voces» primero."
                .to_string(),
        );
    }
    let modelo = translate_engine::model_path(&dir);
    tauri::async_runtime::spawn_blocking(move || {
        project::translate_local(&PathBuf::from(path), &modelo, |traducidos, total| {
            let _ = window.emit(
                "traduccion:progreso",
                ProgresoTraduccion { traducidos, total },
            );
        })
    })
    .await
    .map_err(|e| format!("La traducción local se interrumpió: {e}"))?
}

#[derive(Clone, serde::Serialize)]
struct ProgresoDoblaje {
    generados: usize,
    total: usize,
}

// ADR-009: dobla todos los segmentos traducidos con el motor elegido (Piper local
// o edge-tts online). Asíncrono: sintetiza segmento a segmento y emite
// `doblaje:progreso` a la UI. Piper exige la voz descargada; edge-tts exige red.
#[tauri::command]
async fn generar_doblaje(
    app: tauri::AppHandle,
    window: tauri::Window,
    path: String,
    ajustes: tts::DubSettings,
) -> Result<project::DubResult, String> {
    let dir = models_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        project::generate_dub(&PathBuf::from(path), &ajustes, &dir, |generados, total| {
            let _ = window.emit("doblaje:progreso", ProgresoDoblaje { generados, total });
        })
    })
    .await
    .map_err(|e| format!("La generación del doblaje se interrumpió: {e}"))?
}

// ADR-009: genera (o regenera) el doblaje de un único segmento y devuelve la URL
// local del WAV para reproducirlo (ADR-005). Asíncrono.
#[tauri::command]
async fn generar_doblaje_segmento(
    app: tauri::AppHandle,
    server: tauri::State<'_, media_server::MediaServer>,
    path: String,
    segmento: String,
    ajustes: tts::DubSettings,
) -> Result<String, String> {
    let dir = models_dir(&app)?;
    let wav = tauri::async_runtime::spawn_blocking(move || {
        project::generate_dub_segment(&PathBuf::from(path), &segmento, &ajustes, &dir)
    })
    .await
    .map_err(|e| format!("La generación del doblaje se interrumpió: {e}"))??;
    server.url_for(&wav)
}

// ADR-010: lee el conteo de páginas de un PDF sin importarlo al proyecto.
// Sirve para validar la UI antes de aceptar el archivo.
#[tauri::command]
fn conteo_paginas_pdf(pdf: String) -> Result<u32, String> {
    presentacion::conteo_paginas_pdf(std::path::Path::new(&pdf))
}

// ADR-010: importa un PDF de fondo al proyecto, lo copia bajo `slides/` y
// rasteriza las páginas en background (no bloquea la UI).
#[tauri::command]
async fn importar_pdf(path: String, pdf: String) -> Result<Project, String> {
    tauri::async_runtime::spawn_blocking(move || {
        project::import_pdf(&PathBuf::from(path), &PathBuf::from(pdf))
    })
    .await
    .map_err(|e| format!("La importación del PDF se interrumpió: {e}"))?
}

// ADR-010: importa un audio arbitrario como `manifest.audio`. Pensado para el
// modo presentación cuando no hay video del cual extraer audio.
#[tauri::command]
async fn importar_audio_presentacion(path: String, audio: String) -> Result<Project, String> {
    tauri::async_runtime::spawn_blocking(move || {
        project::import_audio_presentation(&PathBuf::from(path), &PathBuf::from(audio))
    })
    .await
    .map_err(|e| format!("La importación del audio se interrumpió: {e}"))?
}

// ADR-010: importa segmentos desde un JSON externo `{start, end, slide, source}`.
// Sobrescribe los segmentos existentes; la UI confirma antes.
#[tauri::command]
fn importar_segmentos_json(path: String, json: String) -> Result<Project, String> {
    let dir = PathBuf::from(&path);
    let manifest: project::Manifest = serde_json::from_str(
        &std::fs::read_to_string(dir.join("project.json"))
            .map_err(|e| format!("No se pudo leer el manifiesto: {e}"))?,
    )
    .map_err(|e| format!("Manifiesto inválido: {e}"))?;
    let page_count = manifest.slides.as_ref().map(|s| s.page_count);
    project::import_segments_json(&dir, &PathBuf::from(json), page_count)
}

#[derive(Clone, serde::Serialize)]
struct ProgresoPresentacion {
    etapa: usize,
    total: usize,
}

// ADR-010: rasteriza el PDF, concatena el audio doblado y compone el mp4
// final. Auto-dobla los segmentos traducidos que aún no tengan WAV (con el
// motor y la voz que la UI tenga configurados) para que el usuario no
// tenga que pasar por el botón «Generar todas» de la Timeline. Asíncrono:
// puede tardar varios segundos y no debe congelar el hilo principal.
#[tauri::command]
async fn renderizar_presentacion(
    app: tauri::AppHandle,
    window: tauri::Window,
    path: String,
    ajustes: tts::DubSettings,
) -> Result<project::RenderReport, String> {
    let dir = models_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        project::render_presentation(&PathBuf::from(path), &dir, &ajustes, |etapa, total| {
            let _ = window.emit(
                "presentacion:progreso",
                ProgresoPresentacion { etapa, total },
            );
        })
    })
    .await
    .map_err(|e| format!("El render de la presentación se interrumpió: {e}"))?
}

// ADR-010: regenera las imágenes de las páginas a partir del PDF persistido.
// Útil cuando la auto-recuperación del `open` no aplicó (porque el PDF se
// perdió) y el usuario no quiere reimportar todavía. Asíncrono por la misma
// razón que `importar_pdf`: la rasterización puede tardar.
#[tauri::command]
async fn regenerar_imagenes_pdf(path: String) -> Result<Project, String> {
    tauri::async_runtime::spawn_blocking(move || {
        project::regenerate_slide_pages(&PathBuf::from(path))
    })
    .await
    .map_err(|e| format!("La regeneración se interrumpió: {e}"))?
}

// ADR-010: pone cada segmento a `duracion_seg` segundos, uno tras otro a
// partir de 0. Crea un backup único de los `start`/`end` originales la
// primera vez. Pensado para el modo presentación cuando los timings
// importados son aproximados y se prefiere que la duración real del audio
// dicte el ritmo. Asíncrono por simetría con `importar_pdf`: el orden y
// copia son baratos pero la operación no debe congelar la UI.
#[tauri::command]
async fn planificar_tiempos_presentacion(
    path: String,
    duracion_seg: f64,
) -> Result<Project, String> {
    tauri::async_runtime::spawn_blocking(move || {
        project::planificar_tiempos_presentacion(&PathBuf::from(path), duracion_seg)
    })
    .await
    .map_err(|e| format!("La planificación de tiempos se interrumpió: {e}"))?
}

// ADR-010: consulta barata para que el botón «Restaurar» de la TopBar
// sólo aparezca cuando hay un backup real en disco.
#[tauri::command]
fn tiene_backup_timings(path: String) -> bool {
    project::tiene_backup_timings(&PathBuf::from(path))
}

// ADR-010: restaura los `start`/`end` originales desde `timings.original.json`
// y borra el backup. Falla con mensaje claro si no hay backup.
#[tauri::command]
fn restaurar_timings_originales(path: String) -> Result<Project, String> {
    project::restaurar_timings_originales(&PathBuf::from(path))
}

// ADR-010: aplica la duración natural de los WAV de doblaje a los
// `start`/`end` de los segmentos, sin compresión ni dilatación. Pensado
// para el segundo paso del flujo de recalibración, pero expuesto como
// comando por si el frontend quiere dispararlo a mano. En condiciones
// normales lo invoca automáticamente `generate_dub` o `render_presentation`
// cuando el proyecto está en `timing_mode = "placeholder"`.
#[tauri::command]
async fn aplicar_tiempos_reales(
    app: tauri::AppHandle,
    window: tauri::Window,
    path: String,
) -> Result<RecalibrationReport, String> {
    let _ = app;
    tauri::async_runtime::spawn_blocking(move || {
        project::aplicar_tiempos_reales(&PathBuf::from(path), |etapa, total| {
            let _ = window.emit(
                "presentacion:timings:progreso",
                ProgresoPresentacion { etapa, total },
            );
        })
    })
    .await
    .map_err(|e| format!("La recalibración se interrumpió: {e}"))?
}

/// Resultado de leer `segments.json` con su `timing_mode`.
/// Lo usa el frontend para decidir si muestra el botón «Aplicar tiempos».
#[derive(serde::Serialize)]
struct SegmentsConTiming {
    segments: Vec<Segment>,
    timing_mode: Option<String>,
}

// ADR-010: devuelve los segmentos y el `timing_mode` actual para que la
// UI pueda mostrar el botón «Aplicar tiempos» sólo cuando el proyecto está
// en modo placeholder. Es una consulta barata: lee sólo `segments.json`.
#[tauri::command]
fn leer_segments_con_timing(path: String) -> Result<SegmentsConTiming, String> {
    let raw = std::fs::read_to_string(PathBuf::from(&path).join("segments.json"))
        .map_err(|e| format!("No se pudo leer segments.json: {e}"))?;
    #[derive(serde::Deserialize)]
    struct SF {
        #[serde(default)]
        segments: Vec<Segment>,
        #[serde(default)]
        timing_mode: Option<String>,
    }
    let parsed: SF =
        serde_json::from_str(&raw).map_err(|e| format!("segments.json inválido: {e}"))?;
    Ok(SegmentsConTiming {
        segments: parsed.segments,
        timing_mode: parsed.timing_mode,
    })
}

/// Shims públicos para tests de integración que necesitan tocar el pipeline
/// interno sin pasar por la UI ni por `tauri::test`. Marcados con el prefijo
/// `__test_` para que sea evidente que no son parte de la API de cara al
/// usuario; cualquier uso desde la app es un bug.
#[doc(hidden)]
pub mod __test {
    use std::path::{Path, PathBuf};

    pub use super::project::{Presentation, RecalibrationReport, Segment};
    pub use super::tts::DubSettings;
    /// Alias para mantener el nombre usado en el shim de testing.
    pub use super::tts::Engine as DubEngine;

    pub fn crear_proyecto(
        path: &Path,
        nombre: &str,
        origen: &str,
        destino: &str,
    ) -> Result<super::project::Project, String> {
        super::project::create(path, nombre, origen, destino)
    }

    pub fn importar_pdf(path: &Path, pdf: &Path) -> Result<super::project::Project, String> {
        super::project::import_pdf(path, pdf)
    }

    pub fn abrir(path: &Path) -> Result<super::project::Project, String> {
        super::project::open(path)
    }

    pub fn importar_segmentos_json(
        path: &Path,
        json: &Path,
    ) -> Result<super::project::Project, String> {
        let manifest: super::project::Manifest = serde_json::from_str(
            &std::fs::read_to_string(path.join("project.json"))
                .map_err(|e| format!("no se pudo leer el manifiesto: {e}"))?,
        )
        .map_err(|e| format!("manifiesto inválido: {e}"))?;
        let page_count = manifest.slides.as_ref().map(|s| s.page_count);
        super::project::import_segments_json(path, json, page_count)
    }

    pub fn load_segments(path: &Path) -> Result<Vec<Segment>, String> {
        super::project::load_segments(path)
    }

    pub fn render_presentacion(
        path: &Path,
        models_dir: &Path,
        settings: &super::tts::DubSettings,
        on_progress: impl Fn(usize, usize),
    ) -> Result<super::project::RenderReport, String> {
        super::project::render_presentation(path, models_dir, settings, on_progress)
    }

    pub fn regenerar_imagenes_pdf(path: &Path) -> Result<super::project::Project, String> {
        super::project::regenerate_slide_pages(path)
    }

    pub fn planificar_tiempos_presentacion(
        path: &Path,
        duracion_seg: f64,
    ) -> Result<super::project::Project, String> {
        super::project::planificar_tiempos_presentacion(path, duracion_seg)
    }

    pub fn aplicar_tiempos_reales(
        path: &Path,
        on_progress: impl Fn(usize, usize),
    ) -> Result<super::project::RecalibrationReport, String> {
        super::project::aplicar_tiempos_reales(path, on_progress)
    }

    pub fn tiene_backup_timings(path: &Path) -> bool {
        super::project::tiene_backup_timings(path)
    }

    pub fn restaurar_timings_originales(path: &Path) -> Result<super::project::Project, String> {
        super::project::restaurar_timings_originales(path)
    }

    /// Crea un `MediaServer` y devuelve un handle de testing con su puerto.
    /// Permite a los tests E2E verificar que las páginas rasterizadas se
    /// sirven correctamente por HTTP, que es el camino que usa el frontend.
    pub fn iniciar_media_server() -> Result<super::media_server::MediaServerHandle, String> {
        let server = super::media_server::MediaServer::start()?;
        Ok(super::media_server::MediaServerHandle {
            port: server.puerto(),
            token: server.token().to_string(),
            inner: server,
        })
    }

    pub fn path_buf(p: &str) -> PathBuf {
        PathBuf::from(p)
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Fija el proveedor criptográfico de rustls: con aws-lc-rs y ring presentes
    // en el árbol, rustls no puede autodeterminarlo y entra en pánico al abrir
    // el WebSocket de edge-tts. Idempotente: ignora el error si ya está puesto.
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let media =
        media_server::MediaServer::start().expect("no se pudo iniciar el servidor de media local");
    tauri::Builder::default()
        .manage(media)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        // Recuerda y restaura el tamaño/posición de la ventana entre sesiones.
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            crear_proyecto,
            abrir_proyecto,
            guardar_segmentos,
            importar_video,
            cambiar_idiomas,
            extraer_audio,
            transcribir,
            exportar_traduccion,
            importar_traduccion,
            listar_motores_traduccion,
            descargar_motor_traduccion,
            traducir_local,
            listar_modelos,
            descargar_modelo,
            importar_modelo,
            eliminar_modelo,
            listar_voces,
            descargar_voz,
            eliminar_voz,
            listar_voces_edge,
            probar_voz_edge,
            generar_doblaje,
            generar_doblaje_segmento,
            conteo_paginas_pdf,
            importar_pdf,
            importar_audio_presentacion,
            importar_segmentos_json,
            renderizar_presentacion,
            regenerar_imagenes_pdf,
            planificar_tiempos_presentacion,
            aplicar_tiempos_reales,
            tiene_backup_timings,
            restaurar_timings_originales,
            leer_segments_con_timing,
            forma_onda,
            url_media,
            url_slide
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
