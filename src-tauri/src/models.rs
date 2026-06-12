//! Gestión del modelo whisper (ADR-007): descarga por nivel desde Hugging Face,
//! almacenamiento persistente en el directorio de datos de la app, importación de
//! un `.bin` propio y borrado. Las funciones operan sobre un directorio de modelos
//! recibido como parámetro para poder testearse sin Tauri.

use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

/// Un nivel de modelo whisper ofrecido por la app.
pub struct ModelLevel {
    /// Identificador estable, p. ej. `base`. Forma el nombre de archivo y la URL.
    pub id: &'static str,
    /// Etiqueta legible para la interfaz.
    pub label: &'static str,
    /// Tamaño aproximado de la descarga en MB (referencia para la UI).
    pub approx_size_mb: u32,
}

/// Niveles soportados (ADR-007). `base` es el valor por defecto en la UI.
pub const LEVELS: [ModelLevel; 5] = [
    ModelLevel {
        id: "tiny",
        label: "Tiny",
        approx_size_mb: 75,
    },
    ModelLevel {
        id: "base",
        label: "Base",
        approx_size_mb: 142,
    },
    ModelLevel {
        id: "small",
        label: "Small",
        approx_size_mb: 466,
    },
    ModelLevel {
        id: "medium",
        label: "Medium",
        approx_size_mb: 1536,
    },
    ModelLevel {
        id: "large-v3",
        label: "Large v3",
        approx_size_mb: 2960,
    },
];

/// Estado de un nivel de modelo para la interfaz.
#[derive(Debug, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub label: String,
    pub approx_size_mb: u32,
    pub downloaded: bool,
    /// Ruta absoluta del archivo si está descargado.
    pub path: Option<String>,
}

fn level(id: &str) -> Result<&'static ModelLevel, String> {
    LEVELS
        .iter()
        .find(|l| l.id == id)
        .ok_or_else(|| format!("Nivel de modelo desconocido: {id}"))
}

/// Ruta del archivo del modelo de un nivel dentro del directorio de modelos.
pub fn model_path(dir: &Path, id: &str) -> Result<PathBuf, String> {
    let level = level(id)?;
    Ok(dir.join(format!("ggml-{}.bin", level.id)))
}

fn info(dir: &Path, level: &ModelLevel) -> ModelInfo {
    let path = dir.join(format!("ggml-{}.bin", level.id));
    let downloaded = path.is_file();
    ModelInfo {
        id: level.id.to_string(),
        label: level.label.to_string(),
        approx_size_mb: level.approx_size_mb,
        downloaded,
        path: downloaded.then(|| path.display().to_string()),
    }
}

/// Lista todos los niveles con su estado de descarga.
pub fn list(dir: &Path) -> Vec<ModelInfo> {
    LEVELS.iter().map(|l| info(dir, l)).collect()
}

/// Descarga el modelo del nivel indicado desde Hugging Face a `dir`, informando
/// el progreso por `on_progress(descargado, total)`. Descarga a un `.part` y lo
/// renombra al terminar para no dejar un modelo a medias en su lugar definitivo.
pub fn download(dir: &Path, id: &str, on_progress: impl Fn(u64, u64)) -> Result<ModelInfo, String> {
    let level = level(id)?;
    fs::create_dir_all(dir)
        .map_err(|e| format!("No se pudo crear el directorio de modelos: {e}"))?;
    let destination = dir.join(format!("ggml-{}.bin", level.id));
    let partial = destination.with_extension("bin.part");

    let url = format!("{BASE_URL}/ggml-{}.bin", level.id);
    let client = reqwest::blocking::Client::builder()
        .build()
        .map_err(|e| format!("No se pudo iniciar la descarga: {e}"))?;
    let mut response = client
        .get(&url)
        .send()
        .map_err(|e| format!("No se pudo descargar el modelo: {e}"))?
        .error_for_status()
        .map_err(|e| format!("La descarga del modelo falló: {e}"))?;

    let total = response.content_length().unwrap_or(0);
    let mut file = fs::File::create(&partial)
        .map_err(|e| format!("No se pudo crear el archivo del modelo: {e}"))?;
    let mut buffer = [0u8; 64 * 1024];
    let mut downloaded = 0u64;
    // Notificar cada ~4 MB para no inundar la UI de eventos.
    let mut last_notified = 0u64;
    on_progress(0, total);
    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|e| format!("Error al leer la descarga: {e}"))?;
        if read == 0 {
            break;
        }
        std::io::Write::write_all(&mut file, &buffer[..read])
            .map_err(|e| format!("No se pudo escribir el modelo: {e}"))?;
        downloaded += read as u64;
        if downloaded - last_notified >= 4 * 1024 * 1024 {
            last_notified = downloaded;
            on_progress(downloaded, total);
        }
    }
    drop(file);
    on_progress(downloaded, total);

    if downloaded == 0 {
        let _ = fs::remove_file(&partial);
        return Err("La descarga llegó vacía; reintenta.".to_string());
    }
    fs::rename(&partial, &destination)
        .map_err(|e| format!("No se pudo guardar el modelo descargado: {e}"))?;
    Ok(info(dir, level))
}

/// Importa un `.bin` propio al almacén como el modelo del nivel indicado.
pub fn import(dir: &Path, id: &str, source: &Path) -> Result<ModelInfo, String> {
    let level = level(id)?;
    if !source.is_file() {
        return Err(format!(
            "El archivo del modelo no existe: {}",
            source.display()
        ));
    }
    fs::create_dir_all(dir)
        .map_err(|e| format!("No se pudo crear el directorio de modelos: {e}"))?;
    let destination = dir.join(format!("ggml-{}.bin", level.id));
    fs::copy(source, &destination)
        .map_err(|e| format!("No se pudo copiar el modelo al almacén: {e}"))?;
    Ok(info(dir, level))
}

/// Borra el modelo descargado de un nivel, si existe.
pub fn delete(dir: &Path, id: &str) -> Result<(), String> {
    let path = model_path(dir, id)?;
    if path.is_file() {
        fs::remove_file(&path).map_err(|e| format!("No se pudo borrar el modelo: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_path_usa_el_nombre_ggml() {
        let dir = Path::new("/tmp/modelos");
        let path = model_path(dir, "base").unwrap();
        assert!(path.ends_with("ggml-base.bin"));
    }

    #[test]
    fn model_path_rechaza_nivel_desconocido() {
        assert!(model_path(Path::new("/tmp"), "gigante").is_err());
    }

    #[test]
    fn list_marca_descargado_solo_lo_presente() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("ggml-base.bin"), b"modelo falso").unwrap();

        let modelos = list(dir.path());
        assert_eq!(modelos.len(), LEVELS.len());
        let base = modelos.iter().find(|m| m.id == "base").unwrap();
        assert!(base.downloaded);
        assert!(base.path.is_some());
        let tiny = modelos.iter().find(|m| m.id == "tiny").unwrap();
        assert!(!tiny.downloaded);
        assert!(tiny.path.is_none());
    }

    #[test]
    fn import_copia_al_almacen_con_nombre_canonico() {
        let dir = tempfile::tempdir().unwrap();
        let origen = dir.path().join("mi-modelo.bin");
        fs::write(&origen, b"pesos").unwrap();
        let store = dir.path().join("store");

        let info = import(&store, "small", &origen).unwrap();

        assert!(info.downloaded);
        assert!(store.join("ggml-small.bin").is_file());
    }

    #[test]
    fn import_falla_si_el_origen_no_existe() {
        let dir = tempfile::tempdir().unwrap();
        assert!(import(dir.path(), "base", &dir.path().join("no-existe.bin")).is_err());
    }

    #[test]
    fn delete_quita_el_modelo_y_es_idempotente() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("ggml-base.bin"), b"x").unwrap();

        delete(dir.path(), "base").unwrap();
        assert!(!dir.path().join("ggml-base.bin").exists());
        // Borrar de nuevo no falla.
        delete(dir.path(), "base").unwrap();
    }
}
