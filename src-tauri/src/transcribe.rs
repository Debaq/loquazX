//! Transcripción del audio con whisper-rs (ADR-004).

use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Segmento crudo según whisper: tiempos en segundos y texto reconocido.
#[derive(Debug)]
pub struct RawSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

/// Transcribe un WAV PCM 16 bits, mono, 16 kHz (el formato que produce ADR-003)
/// con el modelo GGML indicado. `language` es el código ISO del idioma hablado.
pub fn transcribe(wav: &Path, model: &Path, language: &str) -> Result<Vec<RawSegment>, String> {
    if !model.is_file() {
        return Err(format!(
            "El modelo whisper no existe: {}. Descarga un modelo GGML desde \
             https://huggingface.co/ggerganov/whisper.cpp",
            model.display()
        ));
    }
    let samples = read_wav_samples(wav)?;

    // whisper.cpp escribe su log directo a stderr; los hooks lo desvían al crate
    // `log` (sin logger instalado, se descarta) para no ensuciar la consola.
    whisper_rs::install_logging_hooks();

    let ctx = WhisperContext::new_with_params(model, WhisperContextParameters::default())
        .map_err(|e| format!("No se pudo cargar el modelo whisper: {e}"))?;
    let mut state = ctx
        .create_state()
        .map_err(|e| format!("No se pudo inicializar whisper: {e}"))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some(language));
    params.set_translate(false);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    state
        .full(params, &samples)
        .map_err(|e| format!("whisper falló al transcribir: {e}"))?;

    let mut segments = Vec::new();
    for i in 0..state.full_n_segments() {
        let Some(segment) = state.get_segment(i) else {
            continue;
        };
        let text = segment.to_str_lossy().map_or_else(
            |e| format!("[texto ilegible: {e}]"),
            |t| t.trim().to_string(),
        );
        if text.is_empty() {
            continue;
        }
        // whisper entrega los tiempos en centésimas de segundo.
        segments.push(RawSegment {
            start: segment.start_timestamp() as f64 / 100.0,
            end: segment.end_timestamp() as f64 / 100.0,
            text,
        });
    }
    Ok(segments)
}

/// Lee el WAV y lo convierte a f32 normalizado, validando el formato exacto
/// que whisper exige y que la extracción de ADR-003 garantiza.
fn read_wav_samples(wav: &Path) -> Result<Vec<f32>, String> {
    let reader = hound::WavReader::open(wav)
        .map_err(|e| format!("No se pudo leer el audio {}: {e}", wav.display()))?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.sample_rate != 16_000 || spec.bits_per_sample != 16 {
        return Err(format!(
            "El audio no está en WAV 16 kHz mono 16 bits ({} Hz, {} canales, {} bits). \
             Vuelve a extraer el audio del video.",
            spec.sample_rate, spec.channels, spec.bits_per_sample
        ));
    }
    reader
        .into_samples::<i16>()
        .map(|s| s.map(|v| f32::from(v) / 32_768.0))
        .collect::<Result<Vec<f32>, _>>()
        .map_err(|e| format!("El audio {} está corrupto: {e}", wav.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Genera un WAV 16 kHz mono de un segundo con el ffmpeg del sistema (ADR-003).
    fn wav_16k(dir: &Path) -> std::path::PathBuf {
        let wav = dir.join("audio.wav");
        let status = std::process::Command::new("ffmpeg")
            .args(["-y", "-f", "lavfi", "-i", "sine=frequency=440:duration=1"])
            .args(["-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le"])
            .arg(&wav)
            .status()
            .expect("ffmpeg debe estar instalado para correr estos tests (ADR-003)");
        assert!(status.success(), "ffmpeg no pudo generar el WAV de prueba");
        wav
    }

    #[test]
    fn leer_wav_16k_entrega_muestras_normalizadas() {
        let dir = tempfile::tempdir().unwrap();
        let muestras = read_wav_samples(&wav_16k(dir.path())).unwrap();
        assert_eq!(muestras.len(), 16_000);
        assert!(muestras.iter().all(|m| (-1.0..=1.0).contains(m)));
    }

    #[test]
    fn leer_wav_rechaza_formato_distinto() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("estereo.wav");
        let status = std::process::Command::new("ffmpeg")
            .args(["-y", "-f", "lavfi", "-i", "sine=frequency=440:duration=1"])
            .args(["-ac", "2", "-ar", "44100", "-c:a", "pcm_s16le"])
            .arg(&wav)
            .status()
            .unwrap();
        assert!(status.success());
        assert!(read_wav_samples(&wav).is_err());
    }

    #[test]
    fn transcribir_falla_sin_modelo() {
        let dir = tempfile::tempdir().unwrap();
        let wav = wav_16k(dir.path());
        let error = transcribe(&wav, &dir.path().join("no-existe.bin"), "es").unwrap_err();
        assert!(error.contains("modelo"));
    }
}
