mod audio;
mod project;

use project::{Project, Segment};
use std::path::PathBuf;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            crear_proyecto,
            abrir_proyecto,
            guardar_segmentos,
            importar_video,
            extraer_audio
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
