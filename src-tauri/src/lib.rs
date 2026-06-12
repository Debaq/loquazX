mod audio;
mod download;
mod media_server;
mod models;
mod project;
mod transcribe;
mod translate_engine;
mod translation;
mod tts;
mod tts_edge;
mod voices;

use project::{ExportResult, ImportResult, Project, Segment};
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
fn abrir_proyecto(path: String) -> Result<Project, String> {
    project::open(&PathBuf::from(path))
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
    server.url_for(std::path::Path::new(&path))
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
            forma_onda,
            url_media
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
