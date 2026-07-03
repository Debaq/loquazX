//! Invocación de ffmpeg del sistema para extraer audio (ADR-003).

use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::process::Command;

/// Extrae el audio como WAV PCM 16 bits, mono, 16 kHz: la única entrada
/// que whisper.cpp acepta sin conversión adicional.
pub fn extract_wav_16k(video: &Path, output: &Path) -> Result<(), String> {
    run_ffmpeg(
        &[
            OsStr::new("-y"),
            OsStr::new("-i"),
            video.as_os_str(),
            OsStr::new("-vn"),
            OsStr::new("-ac"),
            OsStr::new("1"),
            OsStr::new("-ar"),
            OsStr::new("16000"),
            OsStr::new("-c:a"),
            OsStr::new("pcm_s16le"),
            output.as_os_str(),
        ],
        "extraer el audio",
    )
}

/// Transcodifica cualquier audio (p. ej. el mp3 de edge-tts) a WAV PCM 16 bits
/// mono a `rate` Hz, el formato uniforme con el que el doblaje trabaja (ADR-009).
pub fn transcode_to_wav(input: &Path, output: &Path, rate: u32) -> Result<(), String> {
    let rate = rate.to_string();
    run_ffmpeg(
        &[
            OsStr::new("-y"),
            OsStr::new("-i"),
            input.as_os_str(),
            OsStr::new("-ac"),
            OsStr::new("1"),
            OsStr::new("-ar"),
            OsStr::new(&rate),
            OsStr::new("-c:a"),
            OsStr::new("pcm_s16le"),
            output.as_os_str(),
        ],
        "convertir el audio a WAV",
    )
}

/// Estira o comprime `input` con `atempo` para que dure `target_secs`,
/// conservando el tono (ADR-009). Escribe el resultado en `output`. Si el hueco
/// o la duración de origen no son válidos, copia sin ajustar.
pub fn fit_duration(input: &Path, output: &Path, target_secs: f64) -> Result<(), String> {
    let source_secs = wav_duration(input)?;
    let filter = atempo_chain(source_secs, target_secs);
    let mut args: Vec<&OsStr> = vec![OsStr::new("-y"), OsStr::new("-i"), input.as_os_str()];
    if let Some(ref filter) = filter {
        args.push(OsStr::new("-filter:a"));
        args.push(OsStr::new(filter));
    }
    args.extend([
        OsStr::new("-c:a"),
        OsStr::new("pcm_s16le"),
        output.as_os_str(),
    ]);
    run_ffmpeg(&args, "ajustar la duración del doblaje")
}

/// Construye la cadena de filtros `atempo` que lleva `source_secs` a
/// `target_secs`. `atempo` solo acepta factores en `[0.5, 2.0]`, así que se
/// encadena para factores mayores. Devuelve `None` si no hay nada que ajustar o
/// los tiempos no son utilizables (evita dividir por cero o estiramientos absurdos).
fn atempo_chain(source_secs: f64, target_secs: f64) -> Option<String> {
    if !source_secs.is_finite()
        || !target_secs.is_finite()
        || source_secs <= 0.0
        || target_secs <= 0.0
    {
        return None;
    }
    // factor > 1 acelera (comprime); < 1 ralentiza (estira). Se acota para no
    // degradar la naturalidad cuando la traducción excede mucho el hueco.
    let mut factor = (source_secs / target_secs).clamp(0.5, 4.0);
    if (factor - 1.0).abs() < 0.01 {
        return None;
    }
    let mut etapas: Vec<String> = Vec::new();
    while factor > 2.0 {
        etapas.push("atempo=2.0".to_string());
        factor /= 2.0;
    }
    while factor < 0.5 {
        etapas.push("atempo=0.5".to_string());
        factor /= 0.5;
    }
    etapas.push(format!("atempo={factor:.4}"));
    Some(etapas.join(","))
}

/// Duración en segundos de un WAV PCM, leída de su cabecera.
pub fn wav_duration(wav: &Path) -> Result<f64, String> {
    let file = File::open(wav).map_err(|e| format!("No se pudo abrir el audio: {e}"))?;
    let mut reader = BufReader::new(file);
    let (channels, sample_rate, bits, data_len) = parse_wav_header(&mut reader)?;
    let bytes_per_frame = (bits as usize / 8) * (channels.max(1) as usize);
    if sample_rate == 0 || bytes_per_frame == 0 {
        return Ok(0.0);
    }
    let frames = data_len as usize / bytes_per_frame;
    Ok(frames as f64 / sample_rate as f64)
}

/// Mensaje estándar cuando ffmpeg no está instalado. Reusado por otros
/// módulos (p. ej. `presentacion`) que también lo invocan.
pub fn ffmpeg_missing_message() -> String {
    "ffmpeg no está instalado o no está en el PATH. Instálalo con el gestor \
     de paquetes del sistema (p. ej. `sudo pacman -S ffmpeg` o `sudo apt \
     install ffmpeg`)."
        .to_string()
}

/// Corre ffmpeg con `args`, traduciendo la ausencia del binario y los fallos a
/// mensajes en español. `accion` describe la tarea para el mensaje de error.
fn run_ffmpeg(args: &[&OsStr], accion: &str) -> Result<(), String> {
    let result = Command::new("ffmpeg").args(args).output();
    let salida = match result {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(ffmpeg_missing_message()),
        Err(e) => return Err(format!("No se pudo ejecutar ffmpeg: {e}")),
        Ok(s) => s,
    };
    if !salida.status.success() {
        let stderr = String::from_utf8_lossy(&salida.stderr);
        // ffmpeg vuelca mucho contexto en stderr; el error real va al final.
        let cola: Vec<&str> = stderr.lines().rev().take(5).collect();
        let cola: Vec<&str> = cola.into_iter().rev().collect();
        return Err(format!("ffmpeg falló al {accion}:\n{}", cola.join("\n")));
    }
    Ok(())
}

/// Envolvente de amplitud del audio para dibujar la pista de onda en la
/// línea de tiempo: `peaks` en 0..1 y la duración en segundos.
#[derive(serde::Serialize)]
pub struct Waveform {
    pub peaks: Vec<f32>,
    pub duration: f32,
}

/// Lee un WAV PCM y devuelve `buckets` picos (máximo absoluto por tramo).
/// Pensado para el `audio.wav` que produce ffmpeg (s16le mono 16 kHz), pero
/// tolera otros conteos de canales y promedia para obtener un solo canal.
pub fn waveform(wav: &Path, buckets: usize) -> Result<Waveform, String> {
    let buckets = buckets.max(1);
    let file = File::open(wav).map_err(|e| format!("No se pudo abrir el audio: {e}"))?;
    let mut reader = BufReader::new(file);

    let (channels, sample_rate, bits, data_len) = parse_wav_header(&mut reader)?;
    if bits != 16 {
        return Err(format!(
            "Solo se admite audio PCM de 16 bits (este tiene {bits})."
        ));
    }
    let channels = channels.max(1) as usize;
    let bytes_per_frame = 2 * channels;
    let total_frames = (data_len as usize) / bytes_per_frame.max(1);
    let duration = if sample_rate > 0 {
        total_frames as f32 / sample_rate as f32
    } else {
        0.0
    };
    if total_frames == 0 {
        return Ok(Waveform {
            peaks: vec![0.0; buckets],
            duration,
        });
    }

    let frames_per_bucket = total_frames.div_ceil(buckets);
    let mut peaks = Vec::with_capacity(buckets);
    let mut frame = [0u8; 2 * 8]; // hasta 8 canales
    let mut frames_read = 0usize;
    let mut current_max = 0.0f32;
    let mut count_in_bucket = 0usize;

    while frames_read < total_frames {
        let buf = &mut frame[..bytes_per_frame];
        if reader.read_exact(buf).is_err() {
            break;
        }
        // Promedio de canales para un solo trazo.
        let mut sum = 0i32;
        for c in 0..channels {
            sum += i32::from(i16::from_le_bytes([buf[c * 2], buf[c * 2 + 1]]));
        }
        let amp = (sum / channels as i32).unsigned_abs() as f32 / 32768.0;
        if amp > current_max {
            current_max = amp;
        }
        frames_read += 1;
        count_in_bucket += 1;
        if count_in_bucket >= frames_per_bucket {
            peaks.push(current_max);
            current_max = 0.0;
            count_in_bucket = 0;
        }
    }
    if count_in_bucket > 0 {
        peaks.push(current_max);
    }
    peaks.resize(buckets, 0.0);

    Ok(Waveform { peaks, duration })
}

/// Recorre los chunks RIFF hasta `fmt ` y `data`. Devuelve
/// (canales, frecuencia, bits por muestra, bytes del chunk de datos).
fn parse_wav_header(reader: &mut impl Read) -> Result<(u16, u32, u16, u32), String> {
    let mut riff = [0u8; 12];
    reader
        .read_exact(&mut riff)
        .map_err(|_| "El audio no es un WAV válido.".to_string())?;
    if &riff[0..4] != b"RIFF" || &riff[8..12] != b"WAVE" {
        return Err("El audio no es un WAV válido.".to_string());
    }

    let mut channels = 0u16;
    let mut sample_rate = 0u32;
    let mut bits = 0u16;
    loop {
        let mut header = [0u8; 8];
        if reader.read_exact(&mut header).is_err() {
            return Err("WAV sin chunk de datos.".to_string());
        }
        let id = &header[0..4];
        let size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
        if id == b"fmt " {
            let mut fmt = vec![0u8; size as usize];
            reader
                .read_exact(&mut fmt)
                .map_err(|_| "Chunk fmt incompleto.".to_string())?;
            channels = u16::from_le_bytes([fmt[2], fmt[3]]);
            sample_rate = u32::from_le_bytes([fmt[4], fmt[5], fmt[6], fmt[7]]);
            bits = u16::from_le_bytes([fmt[14], fmt[15]]);
        } else if id == b"data" {
            return Ok((channels, sample_rate, bits, size));
        } else {
            // Chunk irrelevante (LIST, fact, etc.); saltarlo. Los chunks se
            // alinean a bordes pares.
            let padded = size + (size & 1);
            std::io::copy(&mut reader.take(padded as u64), &mut std::io::sink())
                .map_err(|_| "WAV truncado.".to_string())?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Escribe un WAV PCM mono 16 bits a `rate` Hz con las muestras dadas.
    fn write_wav(path: &Path, rate: u32, samples: &[i16]) {
        let data_len = (samples.len() * 2) as u32;
        let mut f = File::create(path).unwrap();
        f.write_all(b"RIFF").unwrap();
        f.write_all(&(36 + data_len).to_le_bytes()).unwrap();
        f.write_all(b"WAVE").unwrap();
        f.write_all(b"fmt ").unwrap();
        f.write_all(&16u32.to_le_bytes()).unwrap();
        f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
        f.write_all(&1u16.to_le_bytes()).unwrap(); // mono
        f.write_all(&rate.to_le_bytes()).unwrap();
        f.write_all(&(rate * 2).to_le_bytes()).unwrap(); // byte rate
        f.write_all(&2u16.to_le_bytes()).unwrap(); // block align
        f.write_all(&16u16.to_le_bytes()).unwrap(); // bits
        f.write_all(b"data").unwrap();
        f.write_all(&data_len.to_le_bytes()).unwrap();
        for s in samples {
            f.write_all(&s.to_le_bytes()).unwrap();
        }
    }

    #[test]
    fn calcula_picos_y_duracion() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("a.wav");
        // 4 muestras a 2 Hz => 2 s. Dos tramos: max |16384|/32768=0.5 y |-32768|/32768=1.
        write_wav(&wav, 2, &[0, 16384, -32768, 100]);

        let w = waveform(&wav, 2).unwrap();
        assert_eq!(w.peaks.len(), 2);
        assert!((w.duration - 2.0).abs() < 1e-6, "duración: {}", w.duration);
        assert!((w.peaks[0] - 0.5).abs() < 0.01, "pico 0: {}", w.peaks[0]);
        assert!((w.peaks[1] - 1.0).abs() < 0.01, "pico 1: {}", w.peaks[1]);
    }

    #[test]
    fn rellena_buckets_aunque_sobren() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("b.wav");
        write_wav(&wav, 16000, &[1000, -2000]);
        let w = waveform(&wav, 8).unwrap();
        assert_eq!(w.peaks.len(), 8);
    }

    #[test]
    fn rechaza_no_wav() {
        let dir = tempfile::tempdir().unwrap();
        let mal = dir.path().join("c.bin");
        std::fs::write(&mal, b"no soy un wav").unwrap();
        assert!(waveform(&mal, 4).is_err());
    }

    #[test]
    fn duracion_desde_la_cabecera() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("d.wav");
        // 8000 muestras a 16 kHz => 0.5 s.
        write_wav(&wav, 16000, &vec![0i16; 8000]);
        let d = wav_duration(&wav).unwrap();
        assert!((d - 0.5).abs() < 1e-6, "duración: {d}");
    }

    #[test]
    fn atempo_chain_sin_ajuste_cuando_calza() {
        // Mismas duraciones: nada que ajustar.
        assert!(atempo_chain(2.0, 2.0).is_none());
        // Tiempos inválidos: tampoco.
        assert!(atempo_chain(0.0, 2.0).is_none());
        assert!(atempo_chain(2.0, 0.0).is_none());
    }

    #[test]
    fn atempo_chain_comprime_y_estira() {
        // Origen más largo que el hueco => acelera (factor > 1, una etapa).
        let c = atempo_chain(3.0, 2.0).unwrap();
        assert_eq!(c, "atempo=1.5000");
        // Origen mucho más largo => factor acotado a 4 y encadenado (2.0 * 2.0).
        let c = atempo_chain(100.0, 1.0).unwrap();
        assert_eq!(c, "atempo=2.0,atempo=2.0000");
        // Origen más corto => ralentiza (factor < 1).
        let c = atempo_chain(1.0, 2.0).unwrap();
        assert_eq!(c, "atempo=0.5000");
    }
}
