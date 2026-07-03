//! Render del video de presentación a partir del PDF de fondo y los WAV de
//! doblaje (ADR-010). Asume que el usuario ya tradujo y dobló los segmentos:
//! esta etapa sólo compone la imagen y el audio en un mp4.

use crate::audio::{self, wav_duration};
use crate::project::{dub_path, Segment};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Lee el conteo de páginas de un PDF usando `pdfinfo` (poppler). Si el binario
/// no está disponible devuelve un mensaje en español para que la UI lo muestre.
pub fn conteo_paginas_pdf(pdf: &Path) -> Result<u32, String> {
    let output = match Command::new("pdfinfo").arg(pdf).output() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(
                "pdfinfo no está instalado o no está en el PATH. Instálalo con el gestor \
                 de paquetes del sistema (p. ej. `sudo pacman -S poppler` o `sudo apt \
                 install poppler-utils`). pdfinfo viene con poppler."
                    .to_string(),
            )
        }
        Err(e) => return Err(format!("No se pudo ejecutar pdfinfo: {e}")),
        Ok(o) => o,
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pdfinfo falló: {}", stderr.trim()));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for linea in stdout.lines() {
        let mut partes = linea.splitn(2, ':');
        let clave = partes.next().unwrap_or("").trim();
        let valor = partes.next().unwrap_or("").trim();
        if clave.eq_ignore_ascii_case("Pages") {
            return valor
                .parse::<u32>()
                .map_err(|e| format!("No se pudo leer el conteo de páginas: {e}"));
        }
    }
    Err("pdfinfo no reportó el conteo de páginas.".to_string())
}

/// Rasteriza el PDF a PNG en `out_dir` con `pdftoppm` (poppler) a 300 DPI:
/// alta calidad para Full HD y superior, sin saltos visibles al proyectar
/// (ADR-010). Devuelve el conteo de páginas leídas con `conteo_paginas_pdf`.
/// Los archivos se nombran `page-1.png`, `page-2.png`, … con padding según
/// la cantidad de dígitos.
pub fn rasterizar_pdf(pdf: &Path, out_dir: &Path) -> Result<u32, String> {
    let page_count = conteo_paginas_pdf(pdf)?;
    fs::create_dir_all(out_dir)
        .map_err(|e| format!("No se pudo crear el directorio de páginas: {e}"))?;

    let prefijo = out_dir.join("page");
    let status = Command::new("pdftoppm")
        .args(["-r", "300", "-png"])
        .arg(pdf)
        .arg(&prefijo)
        .status();
    match status {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(
            "pdftoppm no está instalado o no está en el PATH. Instálalo con el gestor \
             de paquetes del sistema (p. ej. `sudo pacman -S poppler` o `sudo apt \
             install poppler-utils`). pdftoppm viene con poppler."
                .to_string(),
        ),
        Err(e) => Err(format!("No se pudo ejecutar pdftoppm: {e}")),
        Ok(s) if !s.success() => Err("pdftoppm falló al rasterizar el PDF.".to_string()),
        Ok(_) => {
            // Verificación rápida: el conteo de archivos generados debe coincidir.
            let generados = fs::read_dir(out_dir)
                .map_err(|e| format!("No se pudo leer el directorio de páginas: {e}"))?
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|x| x.to_str())
                        .map(|x| x.eq_ignore_ascii_case("png"))
                        .unwrap_or(false)
                })
                .count() as u32;
            if generados != page_count {
                return Err(format!(
                    "pdftoppm generó {generados} páginas pero el PDF dice {page_count}."
                ));
            }
            Ok(page_count)
        }
    }
}

/// Construye la pista de audio del video (ADR-010): concatena los WAV de
/// doblaje de `runs/dub/` con silencios en los huecos. Salida: un único WAV PCM
/// 16 kHz mono en `out`. Devuelve la duración del WAV resultante.
pub fn concatenar_audio_doblaje(
    segments: &[Segment],
    project_dir: &Path,
    out: &Path,
) -> Result<f64, String> {
    if segments.is_empty() {
        return Err("No hay segmentos para construir el audio.".to_string());
    }

    let total_dur = segments
        .iter()
        .map(|s| s.end)
        .fold(0.0_f64, f64::max)
        .max(1.0);

    // Bloques en orden: silencio_inicial, [WAV_dub_o_silencio, silencio_hueco]*
    let mut entradas: Vec<EntradaAudio> = Vec::new();
    if let Some(primero) = segments.first() {
        if primero.start > 0.0 {
            entradas.push(EntradaAudio::Silencio(primero.start));
        }
    }
    for (i, s) in segments.iter().enumerate() {
        let wav = dub_path(project_dir, &s.id);
        let dur = (s.end - s.start).max(0.0);
        if wav.is_file() {
            entradas.push(EntradaAudio::Wav(wav, dur));
        } else {
            entradas.push(EntradaAudio::Silencio(dur));
        }
        if let Some(sig) = segments.get(i + 1) {
            let hueco = sig.start - s.end;
            if hueco > 0.0 {
                entradas.push(EntradaAudio::Silencio(hueco));
            }
        }
    }
    // Relleno final hasta `total_dur` por si el último segmento acaba antes.
    if let Some(ultimo) = segments.last() {
        let cola = total_dur - ultimo.end;
        if cola > 0.0 {
            entradas.push(EntradaAudio::Silencio(cola));
        }
    }

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear el directorio de audio: {e}"))?;
    }
    ejecutar_concat_audio(&entradas, out)?;
    wav_duration(out)
}

/// Entrada del pipeline de concat de audio (ADR-010): o bien un WAV existente
/// (con duración objetivo) o bien silencio generado con `anullsrc`.
enum EntradaAudio {
    Wav(std::path::PathBuf, f64),
    Silencio(f64),
}

fn ejecutar_concat_audio(entradas: &[EntradaAudio], out: &Path) -> Result<(), String> {
    if entradas.is_empty() {
        return Err("No hay entradas para construir el audio.".to_string());
    }
    // `-t` a nivel de input recorta la salida del filter graph completo, no
    // cada input del concat. Por eso el recorte va dentro del filter con
    // `atrim` y `asetpts`, que sí actúa por stream.
    let mut args: Vec<String> = vec!["-y".into()];
    for e in entradas {
        match e {
            EntradaAudio::Wav(path, _) => {
                args.push("-i".into());
                args.push(path.display().to_string());
            }
            EntradaAudio::Silencio(_) => {
                args.push("-f".into());
                args.push("lavfi".into());
                args.push("-i".into());
                args.push("anullsrc=r=16000:cl=mono".into());
            }
        }
    }
    let mut filtros: Vec<String> = Vec::new();
    for (i, e) in entradas.iter().enumerate() {
        let dur = match e {
            EntradaAudio::Wav(_, d) => *d,
            EntradaAudio::Silencio(d) => *d,
        };
        filtros.push(format!("[{i}:a]atrim=0:{dur},asetpts=PTS-STARTPTS[a{i}]"));
    }
    let concat_inputs: Vec<String> = (0..entradas.len()).map(|i| format!("[a{i}]")).collect();
    filtros.push(format!(
        "{}concat=n={}:v=0:a=1[out]",
        concat_inputs.join(""),
        entradas.len()
    ));
    args.push("-filter_complex".into());
    args.push(filtros.join(";"));
    args.push("-map".into());
    args.push("[out]".into());
    args.push("-c:a".into());
    args.push("pcm_s16le".into());
    args.push("-ar".into());
    args.push("16000".into());
    args.push("-ac".into());
    args.push("1".into());
    args.push(out.display().to_string());

    run_ffmpeg_args(&args, "concatenar el audio de la presentación")
}

/// Compone el mp4 final (ADR-010): concatena las páginas del PDF según los
/// cambios de `slide` en los segmentos y mezcla el audio. Devuelve la duración
/// del video.
pub fn componer_mp4(
    pages_dir: &Path,
    page_count: u32,
    segments: &[Segment],
    audio: &Path,
    out: &Path,
) -> Result<f64, String> {
    if page_count == 0 {
        return Err("El PDF no tiene páginas.".to_string());
    }
    let audio_dur = wav_duration(audio)?;
    if audio_dur <= 0.0 {
        return Err("El audio de la presentación está vacío.".to_string());
    }

    let timeline = calcular_timeline_imagenes(segments, page_count, audio_dur);
    if timeline.is_empty() {
        return Err("No se pudo calcular la línea de tiempo de imágenes.".to_string());
    }

    let concat_txt = pages_dir.parent().unwrap_or(pages_dir).join("concat.txt");
    escribir_concat_imagenes(&concat_txt, &timeline, pages_dir)?;

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear el directorio exports: {e}"))?;
    }

    run_ffmpeg_args(
        &[
            "-y".into(),
            "-f".into(),
            "concat".into(),
            "-safe".into(),
            "0".into(),
            "-i".into(),
            concat_txt.display().to_string(),
            "-i".into(),
            audio.display().to_string(),
            "-vf".into(),
            "scale=trunc(iw/2)*2:trunc(ih/2)*2".into(),
            "-c:v".into(),
            "libx264".into(),
            "-pix_fmt".into(),
            "yuv420p".into(),
            "-c:a".into(),
            "aac".into(),
            "-b:a".into(),
            "192k".into(),
            "-shortest".into(),
            "-movflags".into(),
            "+faststart".into(),
            out.display().to_string(),
        ],
        "componer el video de la presentación",
    )?;

    Ok(audio_dur)
}

#[derive(Debug, Clone, Copy)]
struct BloqueImagen {
    page: u32,
    dur: f64,
}

/// Calcula los bloques `(page, dur)` del timeline de imágenes (ADR-010).
/// `page` es 1‑based y se mantiene entre segmentos hasta el próximo cambio
/// explícito (`slide: Some(p)`). Si nadie asigna `slide`, se usa la página 1.
fn calcular_timeline_imagenes(
    segments: &[Segment],
    page_count: u32,
    total_dur: f64,
) -> Vec<BloqueImagen> {
    let mut bloques: Vec<BloqueImagen> = Vec::new();
    let mut current_page: u32 = 1;
    let mut cursor = 0.0_f64;

    for s in segments {
        if s.start > cursor {
            if let Some(b) = push_bloque(&mut bloques, current_page, s.start - cursor) {
                bloques.push(b);
            }
            cursor = s.start;
        }
        if let Some(p) = s.slide {
            // Se acota al rango válido para no romper el render si quedó un slide
            // viejo tras reimportar un PDF más chico.
            let p = p.clamp(1, page_count);
            current_page = p;
        }
    }
    if cursor < total_dur {
        if let Some(b) = push_bloque(&mut bloques, current_page, total_dur - cursor) {
            bloques.push(b);
        }
    }
    bloques
}

/// Agrupa bloques contiguos de la misma página para no repetir entradas en el
/// concat.txt. Ignora duraciones ≤ 0. Recibe el `Vec` por valor para devolver
/// el bloque a agregar y permitir que el caller lo empuje; o devuelve `None`.
fn push_bloque(bloques: &mut [BloqueImagen], page: u32, dur: f64) -> Option<BloqueImagen> {
    if dur <= 0.0 {
        return None;
    }
    if let Some(ultimo) = bloques.last_mut() {
        if ultimo.page == page {
            ultimo.dur += dur;
            return None;
        }
    }
    Some(BloqueImagen { page, dur })
}

/// Escribe el `concat.txt` que `ffmpeg -f concat` consume. Las rutas se
/// escriben con prefijo `file '...'` y comillas simples escapadas para tolerar
/// espacios en el path.
fn escribir_concat_imagenes(
    concat_txt: &Path,
    bloques: &[BloqueImagen],
    pages_dir: &Path,
) -> Result<(), String> {
    let mut contenido = String::new();
    for b in bloques {
        let ruta = pages_dir.join(format!("page-{}.png", b.page));
        contenido.push_str(&format!(
            "file '{}'\nduration {}\n",
            ruta.display().to_string().replace('\'', "'\\''"),
            b.dur,
        ));
    }
    fs::write(concat_txt, contenido)
        .map_err(|e| format!("No se pudo escribir el concat.txt: {e}"))?;
    Ok(())
}

/// Wrapper sobre `Command::new("ffmpeg")` que traduce la ausencia del binario y
/// los fallos a mensajes en español. `accion` describe la tarea para el error.
fn run_ffmpeg_args(args: &[String], accion: &str) -> Result<(), String> {
    let arg_refs: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
    let result = Command::new("ffmpeg").args(&arg_refs).output();
    let salida = match result {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(audio::ffmpeg_missing_message())
        }
        Err(e) => return Err(format!("No se pudo ejecutar ffmpeg: {e}")),
        Ok(s) => s,
    };
    if !salida.status.success() {
        let stderr = String::from_utf8_lossy(&salida.stderr);
        let cola: Vec<&str> = stderr.lines().rev().take(5).collect();
        let cola: Vec<&str> = cola.into_iter().rev().collect();
        return Err(format!("ffmpeg falló al {accion}:\n{}", cola.join("\n")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(start: f64, end: f64, slide: Option<u32>) -> Segment {
        Segment {
            id: format!("s{start}"),
            start,
            end,
            source: String::new(),
            translation: String::new(),
            slide,
        }
    }

    #[test]
    fn timeline_sin_slide_queda_en_pagina_1() {
        let segmentos = vec![seg(0.0, 4.0, None), seg(4.0, 9.0, None)];
        let t = calcular_timeline_imagenes(&segmentos, 5, 9.0);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].page, 1);
        assert!((t[0].dur - 9.0).abs() < 1e-9);
    }

    #[test]
    fn timeline_cambia_pagina_en_cada_slide() {
        let segmentos = vec![
            seg(0.0, 4.0, Some(1)),
            seg(4.0, 9.0, Some(3)),
            seg(9.0, 12.0, Some(2)),
        ];
        let t = calcular_timeline_imagenes(&segmentos, 5, 12.0);
        assert_eq!(t.len(), 3);
        assert_eq!(t[0].page, 1);
        assert_eq!(t[1].page, 3);
        assert_eq!(t[2].page, 2);
        assert!((t.iter().map(|b| b.dur).sum::<f64>() - 12.0).abs() < 1e-9);
    }

    #[test]
    fn timeline_agrupa_bloques_contiguos_de_la_misma_pagina() {
        let segmentos = vec![
            seg(0.0, 4.0, Some(2)),
            seg(4.0, 9.0, None),
            seg(9.0, 12.0, Some(3)),
        ];
        let t = calcular_timeline_imagenes(&segmentos, 5, 12.0);
        // 0–9 página 2, 9–12 página 3.
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].page, 2);
        assert!((t[0].dur - 9.0).abs() < 1e-9);
        assert_eq!(t[1].page, 3);
        assert!((t[1].dur - 3.0).abs() < 1e-9);
    }

    #[test]
    fn timeline_acota_slide_fuera_de_rango() {
        let segmentos = vec![seg(0.0, 4.0, Some(99))];
        let t = calcular_timeline_imagenes(&segmentos, 3, 4.0);
        assert_eq!(t[0].page, 3);
    }
}
