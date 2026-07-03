//! Firma común de síntesis de voz para el doblaje (ADR-009): un único `synth_segment`
//! despacha el motor elegido —Piper local (por defecto, sin red) o edge-tts online
//! (opt-in)— y deja un WAV mono ya ajustado al hueco del segmento.
//!
//! Cada motor produce primero un WAV «natural» (Piper sintetiza f32 sobre ONNX vía
//! `piper-rs`; edge-tts entrega mp3 que se transcodifica). Luego `audio::fit_duration`
//! lo estira o comprime con `atempo` para que calce el tiempo disponible del segmento,
//! conservando el tono. XTTS-v2 (clonación) queda reservado a una versión futura tras
//! esta misma firma.

use crate::audio;
use piper_rs::Piper;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Motor de síntesis elegido en la interfaz. Los nombres coinciden con los del
/// selector de voz del `EditPanel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    /// Piper local sobre ONNX. Sin red: el texto no sale del equipo (ADR-001).
    Piper,
    /// edge-tts de Microsoft. Opt-in: envía el texto traducido por red.
    #[serde(rename = "edge-tts")]
    Edge,
}

/// Ajustes de doblaje elegidos por idioma de salida. `speed`/`pitch` quedan
/// reservados para una iteración futura; hoy el ajuste de tiempo lo hace `atempo`.
#[derive(Debug, Clone, Deserialize)]
pub struct DubSettings {
    pub engine: Engine,
    /// Id de la voz: el `<id>` de Piper (p. ej. `es_ES-sharvard-medium`) o el
    /// `short_name` de edge-tts (p. ej. `es-ES-AlvaroNeural`).
    pub voice: String,
}

/// Sintetiza `text` con el motor de `settings` y lo deja en `out_wav`. Crea el
/// directorio destino. Las opciones `factor` y `target_secs` son
/// independientes y se combinan en una sola cadena de filtros:
///
/// - `factor`: aplica `atempo(factor)` al audio natural. Se usa para la
///   recalibración de velocidad global (ADR-010).
/// - `target_secs`: ajusta la duración del audio al `target_secs`
///   (modo clásico: encajar al `end - start` del segmento).
///
/// Si ambos son `Some`, se aplica `atempo(factor)` y luego `atrim +
/// apad` para llegar exactamente a `target_secs` (rellenando con silencio
/// si sobra espacio, recortando si sobra audio). Si solo uno es `Some`,
/// se aplica solo ese paso. Si ninguno, se deja el audio a velocidad
/// natural.
pub fn synth_segment(
    settings: &DubSettings,
    text: &str,
    models_dir: &Path,
    factor: Option<f64>,
    target_secs: Option<f64>,
    out_wav: &Path,
) -> Result<(), String> {
    if let Some(parent) = out_wav.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear el directorio de doblaje: {e}"))?;
    }
    // Si no hay factor ni target, el natural es el final: copiamos directo.
    if factor.is_none() && target_secs.is_none() {
        let _ = std::fs::remove_file(out_wav);
        return synth_natural(settings, text, models_dir, out_wav);
    }
    // WAV «natural» temporal junto al destino; se borra tras ajustar.
    let natural = out_wav.with_extension("natural.wav");
    synth_natural(settings, text, models_dir, &natural)?;

    if let (Some(f), Some(target)) = (factor, target_secs) {
        // atempo(factor) + atrim + apad para llegar exactamente al target.
        // atrim recorta si sobra audio; apad rellena con silencio si falta.
        // asetpts resetea los timestamps para que el atrim empiece en 0.
        let filter = format!(
            "atempo={f},asetpts=PTS-STARTPTS,atrim=0:{target},apad=whole_dur={target}"
        );
        let result = audio::run_ffmpeg_filter(
            &natural,
            out_wav,
            &filter,
            "ajustar audio con factor y target",
        );
        let _ = std::fs::remove_file(&natural);
        return result;
    }

    let result = if let Some(f) = factor {
        // Solo factor: atempo(f) sobre el natural, sin target. El audio
        // modificado se usa tal cual (duración = natural / f).
        let filter = format!("atempo={f},asetpts=PTS-STARTPTS");
        audio::run_ffmpeg_filter(
            &natural,
            out_wav,
            &filter,
            "aplicar factor global de atempo",
        )
    } else {
        // Solo target: comportamiento clásico (fit_duration con atempo).
        let target = target_secs.expect("target_secs es Some en este branch");
        audio::fit_duration(&natural, out_wav, target)
    };
    let _ = std::fs::remove_file(&natural);
    result
}

/// Sintetiza a velocidad natural (sin ajuste de tiempo) según el motor.
fn synth_natural(
    settings: &DubSettings,
    text: &str,
    models_dir: &Path,
    out_wav: &Path,
) -> Result<(), String> {
    match settings.engine {
        Engine::Piper => synth_piper(&settings.voice, text, models_dir, out_wav),
        Engine::Edge => synth_edge(&settings.voice, text, out_wav),
    }
}

/// Piper local: carga el modelo ONNX de la voz y su config, fonemiza con espeak-ng
/// (embebido en `piper-rs`) y escribe las muestras f32 como WAV PCM 16 bits.
fn synth_piper(voice: &str, text: &str, models_dir: &Path, out_wav: &Path) -> Result<(), String> {
    let onnx = crate::voices::model_path(models_dir, voice)?;
    if !onnx.is_file() {
        return Err(format!(
            "La voz Piper «{voice}» no está descargada. Descárgala desde «Modelos y voces» primero."
        ));
    }
    // La config de Piper es `<id>.onnx.json`, junto al `.onnx`.
    let config = PathBuf::from(format!("{}.json", onnx.display()));

    let mut piper =
        Piper::new(&onnx, &config).map_err(|e| format!("No se pudo cargar la voz Piper: {e}"))?;
    let (samples, sample_rate) = piper
        .create(text, false, None, None, None, None)
        .map_err(|e| format!("Falló la síntesis Piper: {e}"))?;
    write_wav_i16(out_wav, &samples, sample_rate)
}

/// edge-tts online: reutiliza la síntesis de `tts_edge` (mp3) y la transcodifica
/// al WAV mono uniforme con el que trabaja el resto del pipeline.
fn synth_edge(voice: &str, text: &str, out_wav: &Path) -> Result<(), String> {
    let tmp_mp3 = out_wav.with_extension("edge.mp3");
    let result = crate::tts_edge::synthesize(voice, text, &tmp_mp3)
        .and_then(|()| audio::transcode_to_wav(&tmp_mp3, out_wav, 22_050));
    let _ = std::fs::remove_file(&tmp_mp3);
    result
}

/// Escribe muestras f32 en `[-1, 1]` como WAV PCM mono 16 bits.
fn write_wav_i16(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|e| format!("No se pudo crear el WAV del doblaje: {e}"))?;
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer
            .write_sample(v)
            .map_err(|e| format!("No se pudo escribir el audio del doblaje: {e}"))?;
    }
    writer
        .finalize()
        .map_err(|e| format!("No se pudo cerrar el WAV del doblaje: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motor_se_deserializa_desde_los_nombres_de_la_ui() {
        let p: Engine = serde_json::from_str("\"piper\"").unwrap();
        assert_eq!(p, Engine::Piper);
        let e: Engine = serde_json::from_str("\"edge-tts\"").unwrap();
        assert_eq!(e, Engine::Edge);
        assert!(serde_json::from_str::<Engine>("\"xtts\"").is_err());
    }

    #[test]
    fn ajustes_se_deserializan_del_payload_de_la_ui() {
        let s: DubSettings =
            serde_json::from_str(r#"{"engine":"piper","voice":"es_ES-sharvard-medium"}"#).unwrap();
        assert_eq!(s.engine, Engine::Piper);
        assert_eq!(s.voice, "es_ES-sharvard-medium");
    }

    #[test]
    fn synth_piper_falla_si_la_voz_no_esta_descargada() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("x.wav");
        let err = synth_piper("es_ES-sharvard-medium", "hola", dir.path(), &out).unwrap_err();
        assert!(err.contains("no está descargada"), "mensaje: {err}");
    }

    /// Smoke test real de Piper de punta a punta. Ignorado por defecto (descarga
    /// una voz y requiere ffmpeg). Ejecutar con la voz ya bajada en un directorio:
    ///   LOQUAZ_TEST_VOICE_DIR=/tmp/piper_smoke LOQUAZ_TEST_VOICE=it_IT-riccardo-x_low \
    ///   cargo test piper_sintetiza_wav_real -- --ignored --nocapture
    #[test]
    #[ignore]
    fn piper_sintetiza_wav_real() {
        let dir = std::env::var("LOQUAZ_TEST_VOICE_DIR").expect("falta LOQUAZ_TEST_VOICE_DIR");
        let voice = std::env::var("LOQUAZ_TEST_VOICE").expect("falta LOQUAZ_TEST_VOICE");
        let models_dir = std::path::Path::new(&dir);
        let out = std::path::Path::new(&dir).join("smoke_dub.wav");
        let settings = DubSettings {
            engine: Engine::Piper,
            voice,
        };
        synth_segment(
            &settings,
            "Buongiorno, questa è una prova di doblaje con Piper.",
            models_dir,
            None,
            Some(2.0),
            &out,
        )
        .unwrap();
        let d = audio::wav_duration(&out).unwrap();
        // El ajuste de duración debe acercar el audio al hueco de 2 s.
        assert!((d - 2.0).abs() < 0.2, "duración tras atempo: {d}");
    }

    #[test]
    fn escribe_wav_con_la_frecuencia_dada() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("a.wav");
        // 11025 muestras a 22050 Hz => 0.5 s.
        write_wav_i16(&wav, &vec![0.0f32; 11_025], 22_050).unwrap();
        let d = audio::wav_duration(&wav).unwrap();
        assert!((d - 0.5).abs() < 1e-3, "duración: {d}");
    }
}
