//! Gestión de voces Piper para el doblaje (ADR-009): descarga local de voces TTS
//! desde `rhasspy/piper-voices`, almacenamiento persistente en el directorio de
//! modelos y borrado. Comparte la plomería de descarga con el gestor de whisper
//! (ADR-007) vía `download::download_file`.
//!
//! Una voz Piper son **dos** archivos: el modelo `<id>.onnx` y su configuración
//! `<id>.onnx.json`. Ambos deben estar presentes para considerarla descargada.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

const BASE_URL: &str = "https://huggingface.co/rhasspy/piper-voices/resolve/main";

/// Una voz Piper ofrecida por la app, asociada a un idioma de salida.
pub struct Voice {
    /// Identificador estable de Piper, p. ej. `es_ES-sharvard-medium`. Forma los
    /// nombres de archivo (`<id>.onnx`, `<id>.onnx.json`) y parte de la URL.
    pub id: &'static str,
    /// Etiqueta legible para la interfaz.
    pub label: &'static str,
    /// Código corto del idioma de salida (alineado con `languages.ts`).
    pub language: &'static str,
    /// Carpeta de la voz dentro del repo de Hugging Face, p. ej.
    /// `es/es_ES/sharvard/medium`.
    pub repo_path: &'static str,
    /// Tamaño aproximado de la descarga en MB (referencia para la UI).
    pub approx_size_mb: u32,
}

/// Voces soportadas (ADR-009). Una por idioma de salida con cobertura en Piper;
/// los idiomas sin voz Piper (p. ej. japonés) se cubren con edge-tts (online).
pub const VOICES: [Voice; 8] = [
    Voice {
        id: "es_ES-sharvard-medium",
        label: "Español (ES) — Sharvard",
        language: "es",
        repo_path: "es/es_ES/sharvard/medium",
        approx_size_mb: 63,
    },
    Voice {
        id: "en_US-lessac-medium",
        label: "Inglés (US) — Lessac",
        language: "en",
        repo_path: "en/en_US/lessac/medium",
        approx_size_mb: 63,
    },
    Voice {
        id: "pt_BR-faber-medium",
        label: "Portugués (BR) — Faber",
        language: "pt",
        repo_path: "pt/pt_BR/faber/medium",
        approx_size_mb: 63,
    },
    Voice {
        id: "fr_FR-siwis-medium",
        label: "Francés (FR) — Siwis",
        language: "fr",
        repo_path: "fr/fr_FR/siwis/medium",
        approx_size_mb: 63,
    },
    Voice {
        id: "de_DE-thorsten-medium",
        label: "Alemán (DE) — Thorsten",
        language: "de",
        repo_path: "de/de_DE/thorsten/medium",
        approx_size_mb: 63,
    },
    Voice {
        id: "it_IT-riccardo-x_low",
        label: "Italiano (IT) — Riccardo",
        language: "it",
        repo_path: "it/it_IT/riccardo/x_low",
        approx_size_mb: 28,
    },
    Voice {
        id: "zh_CN-huayan-medium",
        label: "Chino (CN) — Huayan",
        language: "zh",
        repo_path: "zh/zh_CN/huayan/medium",
        approx_size_mb: 63,
    },
    Voice {
        id: "es_MX-ald-medium",
        label: "Español (MX) — Ald",
        language: "es",
        repo_path: "es/es_MX/ald/medium",
        approx_size_mb: 63,
    },
];

/// Estado de una voz para la interfaz.
#[derive(Debug, Serialize)]
pub struct VoiceInfo {
    pub id: String,
    pub label: String,
    pub language: String,
    pub approx_size_mb: u32,
    pub downloaded: bool,
    /// Ruta absoluta del modelo `.onnx` si está descargado.
    pub path: Option<String>,
}

fn voice(id: &str) -> Result<&'static Voice, String> {
    VOICES
        .iter()
        .find(|v| v.id == id)
        .ok_or_else(|| format!("Voz desconocida: {id}"))
}

/// Ruta del modelo `.onnx` de una voz dentro del directorio de modelos.
pub fn model_path(dir: &Path, id: &str) -> Result<PathBuf, String> {
    let voice = voice(id)?;
    Ok(dir.join(format!("{}.onnx", voice.id)))
}

/// Ruta de la configuración `.onnx.json` de una voz.
fn config_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.onnx.json"))
}

fn info(dir: &Path, voice: &Voice) -> VoiceInfo {
    let onnx = dir.join(format!("{}.onnx", voice.id));
    let json = dir.join(format!("{}.onnx.json", voice.id));
    // Una voz solo sirve si están sus dos archivos.
    let downloaded = onnx.is_file() && json.is_file();
    VoiceInfo {
        id: voice.id.to_string(),
        label: voice.label.to_string(),
        language: voice.language.to_string(),
        approx_size_mb: voice.approx_size_mb,
        downloaded,
        path: downloaded.then(|| onnx.display().to_string()),
    }
}

/// Lista todas las voces con su estado de descarga.
pub fn list(dir: &Path) -> Vec<VoiceInfo> {
    VOICES.iter().map(|v| info(dir, v)).collect()
}

/// Descarga la voz indicada (modelo `.onnx` + config `.onnx.json`) a `dir`,
/// informando el progreso del modelo por `on_progress(descargado, total)`. La
/// config es pequeña y se baja primero sin progreso.
pub fn download(dir: &Path, id: &str, on_progress: impl Fn(u64, u64)) -> Result<VoiceInfo, String> {
    let voice = voice(id)?;
    let base = format!("{BASE_URL}/{}/{}", voice.repo_path, voice.id);

    // Config primero (pocos KB): si falla, no dejamos el modelo huérfano.
    crate::download::download_file(
        &format!("{base}.onnx.json"),
        &config_path(dir, id),
        |_, _| {},
    )?;
    crate::download::download_file(
        &format!("{base}.onnx"),
        &dir.join(format!("{id}.onnx")),
        on_progress,
    )?;

    Ok(info(dir, voice))
}

/// Borra los archivos de una voz descargada, si existen. Idempotente.
pub fn delete(dir: &Path, id: &str) -> Result<(), String> {
    let onnx = model_path(dir, id)?;
    let json = config_path(dir, id);
    for path in [onnx, json] {
        if path.is_file() {
            fs::remove_file(&path).map_err(|e| format!("No se pudo borrar la voz: {e}"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_path_usa_la_extension_onnx() {
        let path = model_path(Path::new("/tmp/modelos"), "en_US-lessac-medium").unwrap();
        assert!(path.ends_with("en_US-lessac-medium.onnx"));
    }

    #[test]
    fn model_path_rechaza_voz_desconocida() {
        assert!(model_path(Path::new("/tmp"), "klingon-medium").is_err());
    }

    #[test]
    fn list_marca_descargada_solo_con_ambos_archivos() {
        let dir = tempfile::tempdir().unwrap();
        // Solo el .onnx: aún no cuenta como descargada.
        fs::write(dir.path().join("es_ES-sharvard-medium.onnx"), b"pesos").unwrap();
        let voces = list(dir.path());
        assert_eq!(voces.len(), VOICES.len());
        let es = voces
            .iter()
            .find(|v| v.id == "es_ES-sharvard-medium")
            .unwrap();
        assert!(!es.downloaded);
        assert!(es.path.is_none());

        // Con la config también, ya cuenta.
        fs::write(dir.path().join("es_ES-sharvard-medium.onnx.json"), b"{}").unwrap();
        let voces = list(dir.path());
        let es = voces
            .iter()
            .find(|v| v.id == "es_ES-sharvard-medium")
            .unwrap();
        assert!(es.downloaded);
        assert!(es.path.is_some());
    }

    #[test]
    fn delete_quita_ambos_archivos_y_es_idempotente() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("fr_FR-siwis-medium.onnx"), b"x").unwrap();
        fs::write(dir.path().join("fr_FR-siwis-medium.onnx.json"), b"{}").unwrap();

        delete(dir.path(), "fr_FR-siwis-medium").unwrap();
        assert!(!dir.path().join("fr_FR-siwis-medium.onnx").exists());
        assert!(!dir.path().join("fr_FR-siwis-medium.onnx.json").exists());
        // Borrar de nuevo no falla.
        delete(dir.path(), "fr_FR-siwis-medium").unwrap();
    }

    #[test]
    fn cada_voz_declara_idioma_conocido() {
        // Las voces deben mapear a códigos que la UI ofrece como destino.
        let validos = ["es", "en", "pt", "fr", "de", "it", "ja", "zh"];
        for v in VOICES.iter() {
            assert!(
                validos.contains(&v.language),
                "idioma inválido: {}",
                v.language
            );
        }
    }
}
