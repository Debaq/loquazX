//! Invocación de ffmpeg del sistema para extraer audio (ADR-003).

use std::path::Path;
use std::process::Command;

/// Extrae el audio como WAV PCM 16 bits, mono, 16 kHz: la única entrada
/// que whisper.cpp acepta sin conversión adicional.
pub fn extract_wav_16k(video: &Path, output: &Path) -> Result<(), String> {
    let result = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(video)
        .args(["-vn", "-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le"])
        .arg(output)
        .output();

    let salida = match result {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(
                "ffmpeg no está instalado o no está en el PATH. Instálalo con el gestor \
                 de paquetes del sistema (p. ej. `sudo pacman -S ffmpeg` o `sudo apt \
                 install ffmpeg`)."
                    .to_string(),
            )
        }
        Err(e) => return Err(format!("No se pudo ejecutar ffmpeg: {e}")),
        Ok(s) => s,
    };

    if !salida.status.success() {
        let stderr = String::from_utf8_lossy(&salida.stderr);
        // ffmpeg vuelca mucho contexto en stderr; el error real va al final.
        let cola: Vec<&str> = stderr.lines().rev().take(5).collect();
        let cola: Vec<&str> = cola.into_iter().rev().collect();
        return Err(format!(
            "ffmpeg falló al extraer el audio:\n{}",
            cola.join("\n")
        ));
    }
    Ok(())
}
