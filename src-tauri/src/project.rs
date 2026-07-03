//! Formato de proyecto loquazX según ADR-002: carpeta `.lqzx` con
//! `project.json` (manifiesto), `segments.json` y subdirectorios por etapa.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Versión del formato de carpeta. Incrementar solo con migración documentada.
pub const FORMAT_VERSION: u32 = 1;

const SUBDIRS: [&str; 5] = ["source", "media", "runs", "exports", "slides"];

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
    /// Ausente hasta que se extraiga el audio del video (ADR-003).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<ExtractedAudio>,
    /// Ausente hasta que se importe un PDF de fondo para el modo presentación (ADR-010).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slides: Option<Presentation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedAudio {
    /// Relativa al proyecto, p. ej. `media/audio.wav`.
    pub file: String,
    /// Segundos desde época Unix al momento de extraer.
    pub extracted_at: u64,
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

/// PDF de fondo del modo presentación (ADR-010). Vive bajo `slides/`, copiado al
/// proyecto: se asume que el usuario quiere un proyecto autocontenido.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Presentation {
    /// Relativa al proyecto, p. ej. `slides/original.pdf`.
    pub file: String,
    /// Conteo de páginas leído con `pdfinfo` al importar.
    pub page_count: u32,
    /// Segundos desde época Unix al momento de importar.
    pub imported_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub id: String,
    pub start: f64,
    pub end: f64,
    pub source: String,
    pub translation: String,
    /// Número de página del PDF que se muestra durante `[start, end)` (ADR-010).
    /// `None` significa "mantener la última página vista" (al inicio del video se
    /// asume la 1). Numeración 1‑based, validada contra `Presentation::page_count`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slide: Option<u32>,
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
    /// Ruta absoluta del WAV extraído para whisper, si existe.
    pub audio_path: Option<String>,
    /// Ids de los segmentos que ya tienen audio de doblaje en `runs/dub/` (ADR-009).
    pub dubs: Vec<String>,
    /// Ruta absoluta del PDF de fondo del modo presentación (ADR-010), si hay uno.
    pub slides_path: Option<String>,
    /// Conteo de páginas del PDF de fondo, si hay uno.
    pub slides_page_count: Option<u32>,
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

fn resolved_audio_path(dir: &Path, manifest: &Manifest) -> Option<String> {
    let audio = manifest.audio.as_ref()?;
    Some(dir.join(&audio.file).display().to_string())
}

fn resolved_slides_path(dir: &Path, manifest: &Manifest) -> Option<String> {
    let slides = manifest.slides.as_ref()?;
    Some(dir.join(&slides.file).display().to_string())
}

/// Construye un `Project` a partir del directorio, el manifiesto y los
/// segmentos. Centraliza el shape del `Project` para no repetir los campos
/// derivados en cada alta. `path` es la ruta absoluta del proyecto.
fn build_project(dir: &Path, manifest: Manifest, segments: Vec<Segment>) -> Project {
    let slides_path = resolved_slides_path(dir, &manifest);
    let slides_page_count = manifest.slides.as_ref().map(|s| s.page_count);
    Project {
        path: dir.display().to_string(),
        video_path: resolved_video_path(dir, &manifest),
        audio_path: resolved_audio_path(dir, &manifest),
        dubs: existing_dubs(dir, &segments),
        slides_path,
        slides_page_count,
        manifest,
        segments,
    }
}

fn unix_now() -> Result<u64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Reloj del sistema inválido: {e}"))?
        .as_secs())
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

    let created_at = unix_now()?;
    let manifest = Manifest {
        id: uuid::Uuid::new_v4().to_string(),
        format_version: FORMAT_VERSION,
        name: name.to_string(),
        source_language: source_language.to_string(),
        target_language: target_language.to_string(),
        created_at,
        source: None,
        audio: None,
        slides: None,
    };
    write_json(&path.join("project.json"), &manifest)?;
    write_json(&path.join("segments.json"), &SegmentsFile::default())?;

    Ok(build_project(path, manifest, Vec::new()))
}

/// Cambia los idiomas de origen y destino de un proyecto existente y reescribe el
/// manifiesto. El idioma de origen es el que whisper usa al transcribir (ADR-004):
/// fijarlo correctamente evita transcribir, p. ej., un audio en inglés como español.
pub fn set_languages(
    path: &Path,
    source_language: &str,
    target_language: &str,
) -> Result<Project, String> {
    let mut manifest: Manifest = read_json(&path.join("project.json"))
        .map_err(|e| format!("No es una carpeta de proyecto loquazX válida: {e}"))?;
    manifest.source_language = source_language.to_string();
    manifest.target_language = target_language.to_string();
    write_json(&path.join("project.json"), &manifest)?;
    open(path)
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
    // Auto-recuperación (ADR-010): si el proyecto tiene un PDF pero faltan
    // las imágenes rasterizadas (porque se cerró la app durante un import o
    // porque se manipularon a mano), las regeneramos antes de devolver.
    // Esto deja el proyecto siempre listo para preview y render sin
    // requerir un reimport explícito. Si la regeneración falla, devolvemos
    // el Project igual y el frontend mostrará el error con la opción de
    // reimportar.
    if let Some(slides) = manifest.slides.as_ref() {
        let pdf_abs = path.join(&slides.file);
        let pages_dir = path.join("slides").join("pages");
        let needs_regen = !pages_dir.join("page-1.png").is_file();
        if needs_regen && pdf_abs.is_file() {
            eprintln!(
                "[loquazX] Regenerando páginas del PDF en {}",
                pages_dir.display()
            );
            if let Err(e) = instalar_paginas_rasterizadas(&pdf_abs, &pages_dir) {
                eprintln!("[loquazX] No se pudieron regenerar las páginas: {e}");
            }
        }
    }
    // Un segments.json ausente o corrupto no impide abrir: el manifiesto manda.
    let segments = read_json::<SegmentsFile>(&path.join("segments.json"))
        .unwrap_or_default()
        .segments;

    Ok(build_project(path, manifest, segments))
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
    // El audio extraído pertenece al video anterior: queda obsoleto al reimportar.
    if let Some(audio) = manifest.audio.take() {
        let _ = fs::remove_file(path.join(&audio.file));
    }
    write_json(&path.join("project.json"), &manifest)?;

    let segments = read_json::<SegmentsFile>(&path.join("segments.json"))
        .unwrap_or_default()
        .segments;
    Ok(build_project(path, manifest, segments))
}

pub fn extract_audio(path: &Path) -> Result<Project, String> {
    let mut manifest: Manifest = read_json(&path.join("project.json"))
        .map_err(|e| format!("No es una carpeta de proyecto loquazX válida: {e}"))?;
    let video = resolved_video_path(path, &manifest)
        .ok_or_else(|| "El proyecto no tiene un video importado.".to_string())?;
    let video = Path::new(&video);
    if !video.is_file() {
        return Err(format!(
            "El video del proyecto no existe: {}",
            video.display()
        ));
    }

    let relative = "media/audio.wav";
    let output = path.join(relative);
    // `media/` existe desde create(), pero proyectos manipulados a mano pueden no tenerla.
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear el directorio media: {e}"))?;
    }
    crate::audio::extract_wav_16k(video, &output)?;

    manifest.audio = Some(ExtractedAudio {
        file: relative.to_string(),
        extracted_at: unix_now()?,
    });
    write_json(&path.join("project.json"), &manifest)?;

    let segments = read_json::<SegmentsFile>(&path.join("segments.json"))
        .unwrap_or_default()
        .segments;
    Ok(build_project(path, manifest, segments))
}

pub fn transcribe(path: &Path, model: &Path) -> Result<Project, String> {
    let manifest: Manifest = read_json(&path.join("project.json"))
        .map_err(|e| format!("No es una carpeta de proyecto loquazX válida: {e}"))?;
    let audio = resolved_audio_path(path, &manifest).ok_or_else(|| {
        "El proyecto no tiene audio extraído. Extrae el audio primero.".to_string()
    })?;
    let audio = Path::new(&audio);
    if !audio.is_file() {
        return Err(format!(
            "El audio extraído no existe: {}. Vuelve a extraerlo.",
            audio.display()
        ));
    }

    let segments: Vec<Segment> =
        crate::transcribe::transcribe(audio, model, &manifest.source_language)?
            .into_iter()
            .map(|s| Segment {
                id: uuid::Uuid::new_v4().to_string(),
                start: s.start,
                end: s.end,
                source: s.text,
                translation: String::new(),
                slide: None,
            })
            .collect();

    // ADR-004: la transcripción reemplaza los segmentos; la UI confirma antes.
    write_json(&path.join("segments.json"), &SegmentsFile { segments })?;

    open(path)
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

/// Resultado de exportar la solicitud de traducción (ADR-006).
#[derive(Debug, Serialize)]
pub struct ExportResult {
    /// Ruta absoluta del JSON de solicitud.
    pub request_file: String,
    /// Ruta absoluta del prompt para el LLM.
    pub prompt_file: String,
    /// Cantidad de segmentos exportados.
    pub segment_count: usize,
}

/// Resultado de importar la respuesta de traducción (ADR-006).
#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub project: Project,
    pub report: crate::translation::MergeReport,
}

/// ADR-006: escribe en `exports/` la solicitud JSON y el prompt para el LLM externo.
pub fn export_translation(path: &Path) -> Result<ExportResult, String> {
    let manifest: Manifest = read_json(&path.join("project.json"))
        .map_err(|e| format!("No es una carpeta de proyecto loquazX válida: {e}"))?;
    let segments = read_json::<SegmentsFile>(&path.join("segments.json"))
        .unwrap_or_default()
        .segments;
    if segments.is_empty() {
        return Err("No hay segmentos para traducir. Transcribe el audio primero.".to_string());
    }

    let exports = path.join("exports");
    // `exports/` existe desde create(), pero proyectos manipulados a mano pueden no tenerla.
    fs::create_dir_all(&exports)
        .map_err(|e| format!("No se pudo crear el directorio exports: {e}"))?;

    let request = crate::translation::build_request(
        &manifest.source_language,
        &manifest.target_language,
        &segments,
    );
    let prompt = crate::translation::build_prompt(&request);

    let request_file = exports.join("traduccion-solicitud.json");
    let prompt_file = exports.join("traduccion-prompt.md");
    write_json(&request_file, &request)?;
    fs::write(&prompt_file, prompt).map_err(|e| format!("No se pudo escribir el prompt: {e}"))?;

    Ok(ExportResult {
        request_file: request_file.display().to_string(),
        prompt_file: prompt_file.display().to_string(),
        segment_count: segments.len(),
    })
}

/// ADR-006: lee el JSON de respuesta del LLM y rellena `translation` por `id`.
pub fn import_translation(path: &Path, response: &Path) -> Result<ImportResult, String> {
    if !path.join("project.json").is_file() {
        return Err(format!(
            "No es una carpeta de proyecto loquazX válida: {}",
            path.display()
        ));
    }
    let response: crate::translation::TranslationResponse =
        read_json(response).map_err(|e| format!("El JSON de traducción no es válido: {e}"))?;
    let segments = read_json::<SegmentsFile>(&path.join("segments.json"))
        .unwrap_or_default()
        .segments;

    let (segments, report) = crate::translation::apply_response(segments, &response);
    write_json(&path.join("segments.json"), &SegmentsFile { segments })?;

    Ok(ImportResult {
        project: open(path)?,
        report,
    })
}

/// ADR-008: traduce los segmentos con el motor local y persiste el resultado.
/// Reutiliza `build_request` (ADR-006) para armar la entrada y `apply_response`
/// para volcar la salida del motor sobre `segments.json`, igual que la importación.
pub fn translate_local(
    path: &Path,
    model: &Path,
    on_progress: impl Fn(usize, usize),
) -> Result<ImportResult, String> {
    let manifest: Manifest = read_json(&path.join("project.json"))
        .map_err(|e| format!("No es una carpeta de proyecto loquazX válida: {e}"))?;
    let segments = read_json::<SegmentsFile>(&path.join("segments.json"))
        .unwrap_or_default()
        .segments;
    if segments.is_empty() {
        return Err("No hay segmentos para traducir. Transcribe el audio primero.".to_string());
    }

    let request = crate::translation::build_request(
        &manifest.source_language,
        &manifest.target_language,
        &segments,
    );
    let (response_segments, _engine_report) =
        crate::translate_engine::translate(model, &request, on_progress)?;

    let response = crate::translation::TranslationResponse {
        schema: crate::translation::RESPONSE_SCHEMA.to_string(),
        target_language: manifest.target_language.clone(),
        segments: response_segments,
    };
    let (segments, report) = crate::translation::apply_response(segments, &response);
    write_json(&path.join("segments.json"), &SegmentsFile { segments })?;

    Ok(ImportResult {
        project: open(path)?,
        report,
    })
}

/// Directorio donde viven los WAV de doblaje generados (bajo `runs/`, ADR-002).
fn dub_dir(dir: &Path) -> PathBuf {
    dir.join("runs").join("dub")
}

/// Ruta del WAV de doblaje de un segmento, emparejado por `id` como el resto del
/// pipeline (ADR-009). Hay un único WAV por segmento (regenerable in situ).
pub fn dub_path(dir: &Path, seg_id: &str) -> PathBuf {
    dub_dir(dir).join(format!("{seg_id}.wav"))
}

/// Ids de los segmentos que ya tienen un WAV de doblaje en disco.
fn existing_dubs(dir: &Path, segments: &[Segment]) -> Vec<String> {
    segments
        .iter()
        .filter(|s| dub_path(dir, &s.id).is_file())
        .map(|s| s.id.clone())
        .collect()
}

/// Resumen de una corrida de doblaje sobre un proyecto.
#[derive(Debug, Default, Clone, Serialize)]
pub struct DubReport {
    /// Segmentos sintetizados con éxito.
    pub generated: usize,
    /// Segmentos sin traducción (se omiten, no son un error).
    pub skipped: usize,
}

/// Resultado de doblar un proyecto: el proyecto recargado y el resumen.
#[derive(Debug, Serialize)]
pub struct DubResult {
    pub project: Project,
    pub report: DubReport,
}

pub fn load_segments(path: &Path) -> Result<Vec<Segment>, String> {
    read_json::<Manifest>(&path.join("project.json"))
        .map_err(|e| format!("No es una carpeta de proyecto loquazX válida: {e}"))?;
    Ok(read_json::<SegmentsFile>(&path.join("segments.json"))
        .unwrap_or_default()
        .segments)
}

/// Genera el doblaje de todos los segmentos con traducción (ADR-009): por cada
/// uno sintetiza con `settings` y ajusta el audio al hueco `end - start`,
/// dejando `runs/dub/<id>.wav`. Emite `on_progress(hechos, total)` segmento a
/// segmento. Los segmentos sin traducción se omiten. La primera generación con
/// Piper exige la voz descargada; con edge-tts exige red.
pub fn generate_dub(
    path: &Path,
    settings: &crate::tts::DubSettings,
    models_dir: &Path,
    on_progress: impl Fn(usize, usize),
) -> Result<DubResult, String> {
    let segments = load_segments(path)?;
    let pendientes: Vec<&Segment> = segments
        .iter()
        .filter(|s| !s.translation.trim().is_empty())
        .collect();
    let total = pendientes.len();
    if total == 0 {
        return Err(
            "No hay segmentos traducidos para doblar. Traduce el audio primero.".to_string(),
        );
    }

    on_progress(0, total);
    for (i, segment) in pendientes.iter().enumerate() {
        let target = (segment.end - segment.start).max(0.0);
        crate::tts::synth_segment(
            settings,
            &segment.translation,
            models_dir,
            target,
            &dub_path(path, &segment.id),
        )?;
        on_progress(i + 1, total);
    }

    let report = DubReport {
        generated: total,
        skipped: segments.len() - total,
    };
    Ok(DubResult {
        project: open(path)?,
        report,
    })
}

/// Genera (o regenera) el doblaje de un único segmento y devuelve la ruta del
/// WAV resultante. Útil para la regeneración por segmento del `EditPanel`.
pub fn generate_dub_segment(
    path: &Path,
    seg_id: &str,
    settings: &crate::tts::DubSettings,
    models_dir: &Path,
) -> Result<PathBuf, String> {
    let segments = load_segments(path)?;
    let segment = segments
        .iter()
        .find(|s| s.id == seg_id)
        .ok_or_else(|| format!("No existe el segmento «{seg_id}»."))?;
    if segment.translation.trim().is_empty() {
        return Err("El segmento no tiene traducción que doblar.".to_string());
    }
    let out = dub_path(path, seg_id);
    let target = (segment.end - segment.start).max(0.0);
    crate::tts::synth_segment(settings, &segment.translation, models_dir, target, &out)?;
    Ok(out)
}

/// Rasteriza un PDF en `pages_dir` de forma atómica: escribe a un directorio
/// staging `slides/.pages_new/` y solo al terminar bien la rasterización
/// reemplaza `pages/`. Así, si el proceso se interrumpe a mitad (cierre de la
/// app, crash, falta de espacio), el proyecto nunca queda en un estado
/// inconsistente donde el manifiesto dice "tengo PDF" pero `pages/` está
/// vacío. Si la rasterización falla, el staging se descarta y `pages/` queda
/// como estaba.
fn instalar_paginas_rasterizadas(pdf: &Path, pages_dir: &Path) -> Result<(), String> {
    let parent = pages_dir
        .parent()
        .ok_or_else(|| "pages_dir no tiene directorio padre".to_string())?;
    let staging = parent.join(".pages_new");
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .map_err(|e| format!("No se pudo limpiar el staging previo: {e}"))?;
    }
    fs::create_dir_all(&staging)
        .map_err(|e| format!("No se pudo crear el directorio de staging: {e}"))?;

    // Si pdftoppm falla aquí, el staging queda con archivos parciales; lo
    // limpiamos antes de propagar el error para no dejar basura.
    if let Err(e) = crate::presentacion::rasterizar_pdf(pdf, &staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }

    if pages_dir.exists() {
        fs::remove_dir_all(pages_dir)
            .map_err(|e| format!("No se pudo limpiar el pages/ anterior: {e}"))?;
    }
    fs::rename(&staging, pages_dir).map_err(|e| {
        // Si el rename falla, intentamos recuperar poniendo staging como pages/
        // para que al menos las imágenes queden visibles; si esto también
        // falla, dejamos staging y propagamos el error original.
        let _ = fs::rename(&staging, pages_dir);
        format!("No se pudo mover el staging a pages/: {e}")
    })?;
    Ok(())
}

/// Importa un PDF como fondo del modo presentación (ADR-010). Copia el archivo
/// al proyecto bajo `slides/`, rasteriza cada página a `slides/pages/page-N.png`
/// con `pdftoppm` (300 DPI) y solo al final persiste el conteo en el manifiesto.
/// De esta forma, si la rasterización se interrumpe, el proyecto anterior
/// queda intacto (auto-recuperación al abrir).
///
/// Si ya había un PDF, las páginas viejas se descartan y los segmentos
/// previos conservan su `slide`, que se acota contra el nuevo `page_count`
/// al renderizar.
pub fn import_pdf(path: &Path, pdf: &Path) -> Result<Project, String> {
    if !pdf.is_file() {
        return Err(format!("El PDF no existe: {}", pdf.display()));
    }
    let ext = pdf
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    if ext != "pdf" {
        return Err("El archivo seleccionado no es un PDF.".to_string());
    }
    let mut manifest: Manifest = read_json(&path.join("project.json"))
        .map_err(|e| format!("No es una carpeta de proyecto loquazX válida: {e}"))?;

    let page_count = crate::presentacion::conteo_paginas_pdf(pdf)?;

    let slides_dir = path.join("slides");
    fs::create_dir_all(&slides_dir)
        .map_err(|e| format!("No se pudo crear el directorio slides: {e}"))?;
    let pages_dir = slides_dir.join("pages");
    let name = pdf
        .file_name()
        .ok_or_else(|| format!("Ruta de PDF inválida: {}", pdf.display()))?;
    let destination = slides_dir.join(name);
    fs::copy(pdf, &destination)
        .map_err(|e| format!("No se pudo copiar el PDF al proyecto: {e}"))?;
    let relative = format!("slides/{}", name.to_string_lossy());

    // Rasteriza atómicamente: si esto falla o se interrumpe, el pages/ viejo
    // (o ninguno) queda intacto y el proyecto no se actualiza.
    instalar_paginas_rasterizadas(&destination, &pages_dir)?;

    // Recién ahora, con las imágenes ya en disco, persistimos el manifiesto.
    manifest.slides = Some(Presentation {
        file: relative,
        page_count,
        imported_at: unix_now()?,
    });
    write_json(&path.join("project.json"), &manifest)?;

    let segments = read_json::<SegmentsFile>(&path.join("segments.json"))
        .unwrap_or_default()
        .segments;
    Ok(build_project(path, manifest, segments))
}

/// Importa un audio arbitrario al proyecto, sin pasar por video. Pensado para el
/// modo presentación (ADR-010): el usuario provee el audio original y la app lo
/// transcodifica al formato uniforme (WAV PCM 16 bits mono a 16 kHz) reusando el
/// pipeline de `extract_audio`. Persiste el resultado como `manifest.audio`.
pub fn import_audio_presentation(path: &Path, audio: &Path) -> Result<Project, String> {
    if !audio.is_file() {
        return Err(format!("El audio no existe: {}", audio.display()));
    }
    let mut manifest: Manifest = read_json(&path.join("project.json"))
        .map_err(|e| format!("No es una carpeta de proyecto loquazX válida: {e}"))?;

    let relative = "media/audio.wav";
    let output = path.join(relative);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear el directorio media: {e}"))?;
    }
    crate::audio::extract_wav_16k(audio, &output)?;

    manifest.audio = Some(ExtractedAudio {
        file: relative.to_string(),
        extracted_at: unix_now()?,
    });
    write_json(&path.join("project.json"), &manifest)?;

    let segments = read_json::<SegmentsFile>(&path.join("segments.json"))
        .unwrap_or_default()
        .segments;
    Ok(build_project(path, manifest, segments))
}

/// Regenera las imágenes de las páginas del PDF a partir del PDF persistido
/// en el proyecto (ADR-010). Pensado para recuperar proyectos donde la
/// auto-recuperación del `open` no aplicó (PDF perdido, error transitorio al
/// importar) o donde el usuario quiere forzar la regeneración. Falla con un
/// mensaje claro si no hay un PDF configurado o si el PDF ya no existe en
/// disco; en ese caso la única salida es reimportar.
pub fn regenerate_slide_pages(path: &Path) -> Result<Project, String> {
    let manifest: Manifest = read_json(&path.join("project.json"))
        .map_err(|e| format!("No es una carpeta de proyecto loquazX válida: {e}"))?;
    let slides = manifest
        .slides
        .as_ref()
        .ok_or_else(|| "El proyecto no tiene un PDF importado.".to_string())?;
    let pdf_abs = path.join(&slides.file);
    if !pdf_abs.is_file() {
        return Err(format!(
            "El PDF del proyecto no existe en disco: {}. Vuelve a importar el PDF.",
            pdf_abs.display()
        ));
    }
    let pages_dir = path.join("slides").join("pages");
    instalar_paginas_rasterizadas(&pdf_abs, &pages_dir)?;
    let segments = read_json::<SegmentsFile>(&path.join("segments.json"))
        .unwrap_or_default()
        .segments;
    Ok(build_project(path, manifest, segments))
}

/// Esquema del JSON externo de import de segmentos (ADR-010). El usuario
/// provee `start`, `end`, `source` y opcionalmente `slide` y `translation`.
/// La app asigna `id = uuid`; si falta `translation` queda vacía para que el
/// flujo de traducción la rellene después. Aceptar `translation` evita que el
/// usuario tenga que traducir aparte cuando ya tiene el texto final a mano
/// (caso típico: import de un JSON generado por un LLM externo, ADR-006).
#[derive(Debug, Deserialize)]
pub struct SegmentImport {
    pub start: f64,
    pub end: f64,
    pub source: String,
    #[serde(default)]
    pub translation: String,
    #[serde(default)]
    pub slide: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct SegmentsImportFile {
    segments: Vec<SegmentImport>,
}

/// Importa segmentos desde un JSON externo (ADR-010). Sobrescribe los segmentos
/// existentes; la UI confirma antes. Valida que `start < end` y que `slide`
/// (si está) esté dentro de `page_count` cuando hay un PDF de fondo.
pub fn import_segments_json(
    path: &Path,
    json: &Path,
    page_count: Option<u32>,
) -> Result<Project, String> {
    if !path.join("project.json").is_file() {
        return Err(format!(
            "No es una carpeta de proyecto loquazX válida: {}",
            path.display()
        ));
    }
    let file: SegmentsImportFile =
        read_json(json).map_err(|e| format!("El JSON de segmentos no es válido: {e}"))?;
    if file.segments.is_empty() {
        return Err("El JSON no contiene segmentos.".to_string());
    }

    let mut segments = Vec::with_capacity(file.segments.len());
    for (i, s) in file.segments.into_iter().enumerate() {
        if !s.start.is_finite() || !s.end.is_finite() {
            return Err(format!(
                "El segmento #{i} tiene tiempos inválidos (no finitos)."
            ));
        }
        if s.end <= s.start {
            return Err(format!(
                "El segmento #{i} tiene end ({}) ≤ start ({}).",
                s.end, s.start
            ));
        }
        if let Some(p) = s.slide {
            match page_count {
                Some(max) if p < 1 || p > max => {
                    return Err(format!(
                        "El segmento #{i} pide la página {p} pero el PDF tiene {max}."
                    ));
                }
                None => {
                    return Err(format!(
                        "El segmento #{i} tiene `slide` pero el proyecto no tiene un PDF importado."
                    ));
                }
                _ => {}
            }
        }
        segments.push(Segment {
            id: uuid::Uuid::new_v4().to_string(),
            start: s.start,
            end: s.end,
            source: s.source,
            translation: s.translation,
            slide: s.slide,
        });
    }

    write_json(&path.join("segments.json"), &SegmentsFile { segments })?;
    open(path)
}

/// Resultado de renderizar la presentación (ADR-010).
#[derive(Debug, Serialize)]
pub struct RenderReport {
    /// Ruta absoluta del mp4 producido.
    pub output: String,
    /// Duración total del video, en segundos.
    pub duration_secs: f64,
}

/// Renderiza el video de presentación: dobla los segmentos traducidos que
/// aún no tengan WAV, concatena el audio con silencios en los huecos y
/// compone el mp4 final (ADR-010). Las páginas del PDF ya están
/// rasterizadas bajo `slides/pages/` desde el import, así que el render no
/// depende de `pdftoppm`.
///
/// El auto-doblaje elimina un paso manual en el flujo de presentación: con
/// que el usuario tenga los segmentos traducidos, ya puede exportar; el
/// render se encarga de sintetizar los WAV que falten. Si el usuario
/// prefiere doblar con otro motor, puede hacerlo antes vía el botón
/// «Generar todas» de la Timeline (que no usa este atajo).
///
/// `on_progress(hechos, total)` se emite por cada segmento doblado más las
/// dos etapas finales (audio, video). Si no hay nada que doblar, `total`
/// vale 2.
pub fn render_presentation(
    path: &Path,
    models_dir: &Path,
    settings: &crate::tts::DubSettings,
    on_progress: impl Fn(usize, usize),
) -> Result<RenderReport, String> {
    let manifest: Manifest = read_json(&path.join("project.json"))
        .map_err(|e| format!("No es una carpeta de proyecto loquazX válida: {e}"))?;
    let slides = manifest
        .slides
        .as_ref()
        .ok_or_else(|| "El proyecto no tiene un PDF importado.".to_string())?;
    if !path.join(&slides.file).is_file() {
        return Err(format!(
            "El PDF del proyecto no existe: {}",
            path.join(&slides.file).display()
        ));
    }

    let segments = load_segments(path)?;
    if segments.is_empty() {
        return Err("No hay segmentos en el proyecto.".to_string());
    }
    let page_count = slides.page_count;
    let pages_dir = path.join("slides").join("pages");
    // Verificación defensiva: si el proyecto fue manipulado a mano y faltan
    // las imágenes, no se puede renderizar. La rasterización ocurre al
    // importar; reimportar el PDF la regenera.
    if !pages_dir.join("page-1.png").is_file() {
        return Err(
            "Faltan las imágenes de las páginas del PDF. Vuelve a importar el PDF.".to_string(),
        );
    }

    // Auto-doblaje: sintetiza los WAV de los segmentos que tienen traducción
    // y aún no tienen audio. Los segmentos sin traducción quedan en silencio.
    let pendientes: Vec<&Segment> = segments
        .iter()
        .filter(|s| !s.translation.trim().is_empty() && !dub_path(path, &s.id).is_file())
        .collect();
    let total_pendientes = pendientes.len();
    let total_etapas = total_pendientes + 2;
    on_progress(0, total_etapas);
    for (i, s) in pendientes.iter().enumerate() {
        let target = (s.end - s.start).max(0.0);
        crate::tts::synth_segment(
            settings,
            &s.translation,
            models_dir,
            target,
            &dub_path(path, &s.id),
        )?;
        on_progress(i + 1, total_etapas);
    }

    // 1) Construye la pista de audio a partir de los WAV de doblaje.
    let audio_out = path.join("slides").join("audio.wav");
    crate::presentacion::concatenar_audio_doblaje(&segments, path, &audio_out)?;
    on_progress(total_pendientes + 1, total_etapas);

    // 2) Compone el mp4 final.
    let exports_dir = path.join("exports");
    fs::create_dir_all(&exports_dir)
        .map_err(|e| format!("No se pudo crear el directorio exports: {e}"))?;
    let project_name = &manifest.name;
    let output = exports_dir.join(format!("{project_name}.mp4"));
    let duration =
        crate::presentacion::componer_mp4(&pages_dir, page_count, &segments, &audio_out, &output)?;
    on_progress(total_etapas, total_etapas);

    Ok(RenderReport {
        output: output.display().to_string(),
        duration_secs: duration,
    })
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
            slide: None,
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

    /// Genera un mp4 real (tono de 440 Hz, 1 s) con el ffmpeg del sistema,
    /// requisito asumido por ADR-003 tanto en desarrollo como en CI.
    fn video_real(dir: &Path) -> std::path::PathBuf {
        let video = dir.join("clip.mp4");
        let status = std::process::Command::new("ffmpeg")
            .args(["-y", "-f", "lavfi", "-i", "sine=frequency=440:duration=1"])
            .args(["-c:a", "aac"])
            .arg(&video)
            .status()
            .expect("ffmpeg debe estar instalado para correr estos tests (ADR-003)");
        assert!(status.success(), "ffmpeg no pudo generar el clip de prueba");
        video
    }

    #[test]
    fn extraer_audio_genera_wav_16k_mono() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("demo.lqzx");
        create(&ruta, "Demo", "es", "en").unwrap();
        import_video(&ruta, &video_real(dir.path()), true).unwrap();

        let proyecto = extract_audio(&ruta).unwrap();

        let audio = proyecto.manifest.audio.as_ref().unwrap();
        assert_eq!(audio.file, "media/audio.wav");
        let wav = ruta.join("media/audio.wav");
        assert!(wav.is_file());
        assert_eq!(
            proyecto.audio_path.as_deref(),
            Some(wav.display().to_string().as_str())
        );

        // Cabecera WAV: mono en el byte 22, frecuencia de muestreo en el 24.
        let bytes = fs::read(&wav).unwrap();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 1);
        assert_eq!(
            u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            16000
        );

        // El audio extraído sobrevive a cerrar y reabrir el proyecto.
        let reabierto = open(&ruta).unwrap();
        assert_eq!(reabierto.audio_path, proyecto.audio_path);
    }

    #[test]
    fn extraer_audio_falla_sin_video_importado() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("demo.lqzx");
        create(&ruta, "Demo", "es", "en").unwrap();
        assert!(extract_audio(&ruta).is_err());
    }

    #[test]
    fn extraer_audio_falla_fuera_de_un_proyecto() {
        let dir = tempfile::tempdir().unwrap();
        assert!(extract_audio(dir.path()).is_err());
    }

    #[test]
    fn reimportar_video_invalida_el_audio_extraido() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("demo.lqzx");
        create(&ruta, "Demo", "es", "en").unwrap();
        import_video(&ruta, &video_real(dir.path()), true).unwrap();
        extract_audio(&ruta).unwrap();

        let proyecto = import_video(&ruta, &video_falso(dir.path()), true).unwrap();

        assert!(proyecto.manifest.audio.is_none());
        assert!(proyecto.audio_path.is_none());
        assert!(!ruta.join("media/audio.wav").exists());
    }

    #[test]
    fn transcribir_falla_sin_audio_extraido() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("demo.lqzx");
        create(&ruta, "Demo", "es", "en").unwrap();
        let error = transcribe(&ruta, &dir.path().join("modelo.bin")).unwrap_err();
        assert!(error.contains("audio"));
    }

    #[test]
    fn transcribir_falla_fuera_de_un_proyecto() {
        let dir = tempfile::tempdir().unwrap();
        assert!(transcribe(dir.path(), &dir.path().join("modelo.bin")).is_err());
    }

    #[test]
    fn exportar_traduccion_escribe_solicitud_y_prompt() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("demo.lqzx");
        create(&ruta, "Demo", "es", "en").unwrap();
        save_segments(&ruta, vec![segmento_demo()]).unwrap();

        let resultado = export_translation(&ruta).unwrap();

        assert_eq!(resultado.segment_count, 1);
        assert!(ruta.join("exports/traduccion-solicitud.json").is_file());
        assert!(ruta.join("exports/traduccion-prompt.md").is_file());
        let solicitud = fs::read_to_string(ruta.join("exports/traduccion-solicitud.json")).unwrap();
        assert!(solicitud.contains("\"source\": \"Hola\""));
        assert!(solicitud.contains(crate::translation::REQUEST_SCHEMA));
    }

    #[test]
    fn exportar_traduccion_falla_sin_segmentos() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("demo.lqzx");
        create(&ruta, "Demo", "es", "en").unwrap();
        let error = export_translation(&ruta).unwrap_err();
        assert!(error.contains("segmentos"));
    }

    #[test]
    fn importar_traduccion_rellena_y_persiste() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("demo.lqzx");
        create(&ruta, "Demo", "es", "en").unwrap();
        let mut segmento = segmento_demo();
        segmento.translation = String::new();
        save_segments(&ruta, vec![segmento]).unwrap();

        let respuesta = dir.path().join("respuesta.json");
        fs::write(
            &respuesta,
            format!(
                "{{\"schema\":\"{}\",\"target_language\":\"en\",\"segments\":[{{\"id\":\"s1\",\"translation\":\"Hello\"}}]}}",
                crate::translation::RESPONSE_SCHEMA
            ),
        )
        .unwrap();

        let resultado = import_translation(&ruta, &respuesta).unwrap();

        assert_eq!(resultado.report.translated, 1);
        assert_eq!(resultado.report.missing, 0);
        assert_eq!(resultado.report.unknown, 0);
        assert_eq!(resultado.project.segments[0].translation, "Hello");
        // Persistió en disco.
        let reabierto = open(&ruta).unwrap();
        assert_eq!(reabierto.segments[0].translation, "Hello");
    }

    #[test]
    fn importar_traduccion_falla_con_json_invalido() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("demo.lqzx");
        create(&ruta, "Demo", "es", "en").unwrap();
        let respuesta = dir.path().join("malo.json");
        fs::write(&respuesta, "esto no es json").unwrap();
        assert!(import_translation(&ruta, &respuesta).is_err());
    }

    #[test]
    fn importar_traduccion_falla_fuera_de_un_proyecto() {
        let dir = tempfile::tempdir().unwrap();
        let respuesta = dir.path().join("r.json");
        fs::write(&respuesta, "{\"segments\":[]}").unwrap();
        assert!(import_translation(dir.path(), &respuesta).is_err());
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
