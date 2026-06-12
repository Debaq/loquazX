//! Previsualización de voces edge-tts (ADR-009): lista las voces online de
//! «Microsoft Edge Read Aloud» y sintetiza una muestra para audicionarlas y
//! elegir la mejor. Es la vía OPT-IN con red: el texto va a servidores de
//! Microsoft (excepción explícita a ADR-001, no es la opción por defecto).

use msedge_tts::tts::client::connect;
use msedge_tts::tts::SpeechConfig;
use msedge_tts::voice::{get_voices_list, Voice};
use serde::Serialize;
use std::path::Path;

/// Una voz edge-tts expuesta a la interfaz.
#[derive(Debug, Serialize)]
pub struct EdgeVoice {
    /// Nombre técnico estable, p. ej. `es-ES-AlvaroNeural`.
    pub short_name: String,
    /// Locale completo, p. ej. `es-ES`.
    pub locale: String,
    /// Código corto del idioma (de `locale`), alineado con `languages.ts`.
    pub language: String,
    /// `Male`/`Female` si lo declara la voz.
    pub gender: String,
    pub friendly_name: String,
}

/// Idioma corto a partir del locale (`es-ES` -> `es`).
fn lang_from_locale(locale: &str) -> String {
    locale.split('-').next().unwrap_or("").to_string()
}

fn to_edge_voice(v: &Voice) -> Option<EdgeVoice> {
    let short_name = v.short_name.clone()?;
    let locale = v.locale.clone().unwrap_or_default();
    Some(EdgeVoice {
        language: lang_from_locale(&locale),
        short_name,
        gender: v.gender.clone().unwrap_or_default(),
        friendly_name: v.friendly_name.clone().unwrap_or_default(),
        locale,
    })
}

/// Lista las voces edge-tts. Requiere red (consulta el endpoint de Microsoft).
pub fn list() -> Result<Vec<EdgeVoice>, String> {
    let voices = get_voices_list()
        .map_err(|e| format!("No se pudieron listar las voces edge-tts (¿sin red?): {e}"))?;
    let mut out: Vec<EdgeVoice> = voices.iter().filter_map(to_edge_voice).collect();
    out.sort_by(|a, b| a.short_name.cmp(&b.short_name));
    Ok(out)
}

/// Sintetiza `text` con la voz `short_name` y escribe el mp3 en `out`. Requiere red.
pub fn synthesize(short_name: &str, text: &str, out: &Path) -> Result<(), String> {
    let voices = get_voices_list()
        .map_err(|e| format!("No se pudieron obtener las voces edge-tts (¿sin red?): {e}"))?;
    let voice = voices
        .iter()
        .find(|v| v.short_name.as_deref() == Some(short_name))
        .ok_or_else(|| format!("Voz edge-tts desconocida: {short_name}"))?;

    let config = SpeechConfig::from(voice);
    let mut tts = connect().map_err(|e| format!("No se pudo conectar a edge-tts: {e}"))?;
    let audio = tts
        .synthesize(text, &config)
        .map_err(|e| format!("Falló la síntesis edge-tts: {e}"))?;

    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear el directorio de previsualización: {e}"))?;
    }
    std::fs::write(out, &audio.audio_bytes)
        .map_err(|e| format!("No se pudo guardar el audio de previsualización: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_desde_locale() {
        assert_eq!(lang_from_locale("es-ES"), "es");
        assert_eq!(lang_from_locale("pt-BR"), "pt");
        assert_eq!(lang_from_locale("en"), "en");
        assert_eq!(lang_from_locale(""), "");
    }
}
