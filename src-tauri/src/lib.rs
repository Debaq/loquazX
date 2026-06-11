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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            crear_proyecto,
            abrir_proyecto,
            guardar_segmentos
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
