//! Formato de proyecto loquazX según ADR-002: carpeta `.lqzx` con
//! `project.json` (manifiesto), `segments.json` y subdirectorios por etapa.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Versión del formato de carpeta. Incrementar solo con migración documentada.
pub const FORMAT_VERSION: u32 = 1;

const SUBDIRS: [&str; 4] = ["source", "media", "runs", "exports"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub format_version: u32,
    pub name: String,
    pub source_language: String,
    pub target_language: String,
    /// Segundos desde época Unix; suficiente hasta que un ADR defina metadatos ricos.
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub id: String,
    pub start: f64,
    pub end: f64,
    pub source: String,
    pub translation: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SegmentsFile {
    segments: Vec<Segment>,
}

#[derive(Debug, Serialize)]
pub struct Project {
    pub path: String,
    pub manifest: Manifest,
    pub segments: Vec<Segment>,
}

pub fn create(
    path: &Path,
    name: &str,
    source_language: &str,
    target_language: &str,
) -> Result<Project, String> {
    if path.exists() {
        return Err(format!("La ruta ya existe: {}", path.display()));
    }
    fs::create_dir_all(path)
        .map_err(|e| format!("No se pudo crear el directorio del proyecto: {e}"))?;
    for sub in SUBDIRS {
        fs::create_dir(path.join(sub))
            .map_err(|e| format!("No se pudo crear el subdirectorio {sub}: {e}"))?;
    }

    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Reloj del sistema inválido: {e}"))?
        .as_secs();
    let manifest = Manifest {
        id: uuid::Uuid::new_v4().to_string(),
        format_version: FORMAT_VERSION,
        name: name.to_string(),
        source_language: source_language.to_string(),
        target_language: target_language.to_string(),
        created_at,
    };
    write_json(&path.join("project.json"), &manifest)?;
    write_json(&path.join("segments.json"), &SegmentsFile::default())?;

    Ok(Project {
        path: path.display().to_string(),
        manifest,
        segments: Vec::new(),
    })
}

pub fn open(path: &Path) -> Result<Project, String> {
    let manifest: Manifest = read_json(&path.join("project.json"))
        .map_err(|e| format!("No es una carpeta de proyecto loquazX válida: {e}"))?;
    if manifest.format_version > FORMAT_VERSION {
        return Err(format!(
            "El proyecto usa el formato {} pero esta versión de la aplicación soporta hasta {}.",
            manifest.format_version, FORMAT_VERSION
        ));
    }
    // Un segments.json ausente o corrupto no impide abrir: el manifiesto manda.
    let segments = read_json::<SegmentsFile>(&path.join("segments.json"))
        .unwrap_or_default()
        .segments;

    Ok(Project {
        path: path.display().to_string(),
        manifest,
        segments,
    })
}

pub fn save_segments(path: &Path, segments: Vec<Segment>) -> Result<(), String> {
    if !path.join("project.json").is_file() {
        return Err(format!(
            "No es una carpeta de proyecto loquazX válida: {}",
            path.display()
        ));
    }
    write_json(&path.join("segments.json"), &SegmentsFile { segments })
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let raw =
        fs::read_to_string(path).map_err(|e| format!("no se pudo leer {}: {e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("JSON inválido en {}: {e}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(value)
        .map_err(|e| format!("no se pudo serializar {}: {e}", path.display()))?;
    // Escritura a temporal + rename para no dejar el archivo a medias ante un corte.
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, raw).map_err(|e| format!("no se pudo escribir {}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("no se pudo reemplazar {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segmento_demo() -> Segment {
        Segment {
            id: "s1".into(),
            start: 0.0,
            end: 3.2,
            source: "Hola".into(),
            translation: "Hello".into(),
        }
    }

    #[test]
    fn crear_y_abrir_ida_y_vuelta() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("demo.lqzx");

        let creado = create(&ruta, "Demo", "es", "en").unwrap();
        assert_eq!(creado.manifest.format_version, FORMAT_VERSION);
        for sub in SUBDIRS {
            assert!(ruta.join(sub).is_dir(), "falta subdirectorio {sub}");
        }

        let abierto = open(&ruta).unwrap();
        assert_eq!(abierto.manifest.id, creado.manifest.id);
        assert_eq!(abierto.manifest.name, "Demo");
        assert_eq!(abierto.manifest.source_language, "es");
        assert_eq!(abierto.manifest.target_language, "en");
        assert!(abierto.segments.is_empty());
    }

    #[test]
    fn crear_falla_si_la_ruta_existe() {
        let dir = tempfile::tempdir().unwrap();
        assert!(create(dir.path(), "Demo", "es", "en").is_err());
    }

    #[test]
    fn abrir_falla_sin_manifiesto() {
        let dir = tempfile::tempdir().unwrap();
        assert!(open(dir.path()).is_err());
    }

    #[test]
    fn abrir_falla_con_formato_mas_nuevo() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("demo.lqzx");
        create(&ruta, "Demo", "es", "en").unwrap();
        let manifiesto = fs::read_to_string(ruta.join("project.json")).unwrap();
        let futuro = manifiesto.replace(
            &format!("\"format_version\": {FORMAT_VERSION}"),
            &format!("\"format_version\": {}", FORMAT_VERSION + 1),
        );
        fs::write(ruta.join("project.json"), futuro).unwrap();
        assert!(open(&ruta).is_err());
    }

    #[test]
    fn guardar_segmentos_persiste() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("demo.lqzx");
        create(&ruta, "Demo", "es", "en").unwrap();

        save_segments(&ruta, vec![segmento_demo()]).unwrap();

        let abierto = open(&ruta).unwrap();
        assert_eq!(abierto.segments.len(), 1);
        assert_eq!(abierto.segments[0].translation, "Hello");
    }

    #[test]
    fn guardar_segmentos_falla_fuera_de_un_proyecto() {
        let dir = tempfile::tempdir().unwrap();
        assert!(save_segments(dir.path(), vec![segmento_demo()]).is_err());
    }
}
