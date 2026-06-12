mod audio;
mod media_server;
mod models;
mod project;
mod transcribe;
mod translation;

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

// Asíncrono: ffmpeg puede tardar y no debe congelar el hilo principal.
#[tauri::command]
async fn extraer_audio(path: String) -> Result<Project, String> {
    tauri::async_runtime::spawn_blocking(move || project::extract_audio(&PathBuf::from(path)))
        .await
        .map_err(|e| format!("La extracción de audio se interrumpió: {e}"))?
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let media =
        media_server::MediaServer::start().expect("no se pudo iniciar el servidor de media local");
    tauri::Builder::default()
        .manage(media)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            crear_proyecto,
            abrir_proyecto,
            guardar_segmentos,
            importar_video,
            extraer_audio,
            transcribir,
            exportar_traduccion,
            importar_traduccion,
            listar_modelos,
            descargar_modelo,
            importar_modelo,
            eliminar_modelo,
            url_media
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
