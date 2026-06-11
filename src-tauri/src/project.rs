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
    /// Ausente hasta que se importe un video; opcional para no romper proyectos previos.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceVideo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceMode {
    Copy,
    Reference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceVideo {
    /// Relativa al proyecto en modo copia (`source/...`); absoluta en modo referencia.
    pub file: String,
    pub mode: SourceMode,
    pub original_path: String,
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
    /// Ruta absoluta del video listo para reproducir, si hay uno importado.
    pub video_path: Option<String>,
}

fn resolved_video_path(dir: &Path, manifest: &Manifest) -> Option<String> {
    let source = manifest.source.as_ref()?;
    let file = Path::new(&source.file);
    let absolute = if file.is_absolute() {
        file.to_path_buf()
    } else {
        dir.join(file)
    };
    Some(absolute.display().to_string())
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
        source: None,
    };
    write_json(&path.join("project.json"), &manifest)?;
    write_json(&path.join("segments.json"), &SegmentsFile::default())?;

    Ok(Project {
        path: path.display().to_string(),
        manifest,
        segments: Vec::new(),
        video_path: None,
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
        video_path: resolved_video_path(path, &manifest),
        manifest,
        segments,
    })
}

pub fn import_video(path: &Path, video: &Path, copy: bool) -> Result<Project, String> {
    let mut manifest: Manifest = read_json(&path.join("project.json"))
        .map_err(|e| format!("No es una carpeta de proyecto loquazX válida: {e}"))?;
    if !video.is_file() {
        return Err(format!("El video no existe: {}", video.display()));
    }

    let file = if copy {
        let name = video
            .file_name()
            .ok_or_else(|| format!("Ruta de video inválida: {}", video.display()))?;
        let destination = path.join("source").join(name);
        fs::copy(video, &destination)
            .map_err(|e| format!("No se pudo copiar el video al proyecto: {e}"))?;
        format!("source/{}", name.to_string_lossy())
    } else {
        let absolute = video
            .canonicalize()
            .map_err(|e| format!("No se pudo resolver la ruta del video: {e}"))?;
        absolute.display().to_string()
    };

    manifest.source = Some(SourceVideo {
        file,
        mode: if copy {
            SourceMode::Copy
        } else {
            SourceMode::Reference
        },
        original_path: video.display().to_string(),
    });
    write_json(&path.join("project.json"), &manifest)?;

    let segments = read_json::<SegmentsFile>(&path.join("segments.json"))
        .unwrap_or_default()
        .segments;
    Ok(Project {
        path: path.display().to_string(),
        video_path: resolved_video_path(path, &manifest),
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

    fn video_falso(dir: &Path) -> std::path::PathBuf {
        let video = dir.join("clip.mp4");
        fs::write(&video, b"contenido de prueba").unwrap();
        video
    }

    #[test]
    fn importar_video_en_modo_copia() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("demo.lqzx");
        create(&ruta, "Demo", "es", "en").unwrap();
        let video = video_falso(dir.path());

        let proyecto = import_video(&ruta, &video, true).unwrap();

        let source = proyecto.manifest.source.as_ref().unwrap();
        assert_eq!(source.mode, SourceMode::Copy);
        assert_eq!(source.file, "source/clip.mp4");
        assert!(ruta.join("source/clip.mp4").is_file());
        assert_eq!(
            proyecto.video_path.as_deref(),
            Some(ruta.join("source/clip.mp4").display().to_string().as_str())
        );
    }

    #[test]
    fn importar_video_en_modo_referencia() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("demo.lqzx");
        create(&ruta, "Demo", "es", "en").unwrap();
        let video = video_falso(dir.path());

        let proyecto = import_video(&ruta, &video, false).unwrap();

        let source = proyecto.manifest.source.as_ref().unwrap();
        assert_eq!(source.mode, SourceMode::Reference);
        assert!(Path::new(&source.file).is_absolute());
        assert!(!ruta.join("source/clip.mp4").exists());

        // El video referenciado sobrevive a cerrar y reabrir el proyecto.
        let reabierto = open(&ruta).unwrap();
        assert_eq!(reabierto.video_path, proyecto.video_path);
    }

    #[test]
    fn importar_video_falla_si_no_existe() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("demo.lqzx");
        create(&ruta, "Demo", "es", "en").unwrap();
        assert!(import_video(&ruta, &dir.path().join("nada.mp4"), true).is_err());
    }

    #[test]
    fn importar_video_falla_fuera_de_un_proyecto() {
        let dir = tempfile::tempdir().unwrap();
        let video = video_falso(dir.path());
        assert!(import_video(dir.path(), &video, true).is_err());
    }

    #[test]
    fn abrir_proyecto_antiguo_sin_campo_source() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("demo.lqzx");
        create(&ruta, "Demo", "es", "en").unwrap();
        let proyecto = open(&ruta).unwrap();
        assert!(proyecto.manifest.source.is_none());
        assert!(proyecto.video_path.is_none());
    }
}
