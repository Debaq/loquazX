//! Descarga HTTP con progreso reutilizable (ADR-007/009): baja a un `.part` y lo
//! renombra al terminar, informando el avance por callback. La comparten el gestor
//! de modelos whisper (`models`) y el de voces Piper (`voices`).

use std::fs;
use std::io::Read;
use std::path::Path;

/// Descarga `url` a `dest` pasando por un archivo temporal `<dest>.part` que se
/// renombra al terminar, para no dejar un archivo a medias en su lugar definitivo.
/// Notifica el avance por `on_progress(descargado, total)` y devuelve los bytes
/// escritos. Falla si la descarga llega vacía.
pub fn download_file(
    url: &str,
    dest: &Path,
    on_progress: impl Fn(u64, u64),
) -> Result<u64, String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear el directorio de descarga: {e}"))?;
    }
    let nombre = dest
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or_else(|| "Ruta de descarga inválida.".to_string())?;
    let partial = dest.with_file_name(format!("{nombre}.part"));

    let client = reqwest::blocking::Client::builder()
        .build()
        .map_err(|e| format!("No se pudo iniciar la descarga: {e}"))?;
    let mut response = client
        .get(url)
        .send()
        .map_err(|e| format!("No se pudo descargar: {e}"))?
        .error_for_status()
        .map_err(|e| format!("La descarga falló: {e}"))?;

    let total = response.content_length().unwrap_or(0);
    let mut file = fs::File::create(&partial)
        .map_err(|e| format!("No se pudo crear el archivo de descarga: {e}"))?;
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
            .map_err(|e| format!("No se pudo escribir la descarga: {e}"))?;
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
    fs::rename(&partial, dest).map_err(|e| format!("No se pudo guardar la descarga: {e}"))?;
    Ok(downloaded)
}
