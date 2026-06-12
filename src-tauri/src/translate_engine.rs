//! Motor de traducción local (ADR-008, revisado 2026-06-12): NMT **NLLB-200
//! distilled 600M** sobre **ONNX Runtime** (`ort`), el mismo backend de inferencia
//! que el motor de voz Piper (ADR-009). Reemplaza la decisión original (LLM
//! instruct GGUF vía `llama.cpp`), que en CPU era lenta (~3-8 tok/s) y pesada
//! (2-5 GB RAM) para una calidad apenas mediocre. NLLB es chico (~900 MB en ONNX
//! cuantizado), rápido en CPU (<1 s por segmento) y cubre 200 idiomas en un modelo.
//!
//! El modelo son varios archivos (ONNX cuantizados + tokenizer) que viven juntos
//! en un subdirectorio del almacén `models/`. Reutiliza el formato de ADR-006:
//! produce los mismos `ResponseSegment` que el flujo de export/import.
//!
//! La **inferencia** corre el grafo seq2seq sobre `ort`: el encoder produce los
//! estados ocultos del texto origen y el decoder (variante **sin caché**,
//! `decoder_model_quantized.onnx`) genera token a token (greedy) forzando el
//! código de idioma destino. Es O(n²) por segmento, pero los segmentos son cortos
//! y mantiene el código simple y correcto (la generación con caché de
//! claves/valores queda como optimización futura tras la misma firma).

use crate::translation::{ResponseSegment, TranslationRequest};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;
use serde::Serialize;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

/// Subcarpeta del modelo dentro del almacén `models/`.
pub const MODEL_DIR: &str = "nllb-200-distilled-600M";

const BASE_URL: &str = "https://huggingface.co/Xenova/nllb-200-distilled-600M/resolve/main";

/// Un archivo que compone el modelo. La ruta relativa en el repo de Hugging Face
/// coincide con la ruta relativa dentro del subdirectorio local.
struct ModelFile {
    rel: &'static str,
    approx_size_mb: u32,
}

/// Archivos del modelo. Se usan los ONNX **cuantizados**: livianos y rápidos en CPU.
const FILES: [ModelFile; 5] = [
    ModelFile {
        rel: "onnx/encoder_model_quantized.onnx",
        approx_size_mb: 419,
    },
    ModelFile {
        rel: "onnx/decoder_model_quantized.onnx",
        approx_size_mb: 449,
    },
    ModelFile {
        rel: "tokenizer.json",
        approx_size_mb: 17,
    },
    ModelFile {
        rel: "config.json",
        approx_size_mb: 1,
    },
    ModelFile {
        rel: "generation_config.json",
        approx_size_mb: 1,
    },
];

/// Tamaño aproximado total de la descarga, para la interfaz.
pub const APPROX_SIZE_MB: u32 = 887;

/// Estado del motor de traducción para la interfaz.
#[derive(Debug, Serialize)]
pub struct EngineInfo {
    pub id: String,
    pub label: String,
    pub approx_size_mb: u32,
    pub downloaded: bool,
    pub path: Option<String>,
}

/// Resumen de una corrida del motor local sobre un proyecto.
#[derive(Debug, Default, Clone, Serialize)]
pub struct EngineReport {
    /// Segmentos a los que el motor asignó una traducción no vacía.
    pub translated: usize,
    /// Segmentos que el motor no logró traducir.
    pub failed: usize,
}

/// Directorio del modelo dentro del almacén `models/`.
pub fn model_path(dir: &Path) -> PathBuf {
    dir.join(MODEL_DIR)
}

/// `true` si todos los archivos del modelo están presentes.
pub fn is_downloaded(dir: &Path) -> bool {
    let base = model_path(dir);
    FILES.iter().all(|f| base.join(f.rel).is_file())
}

/// Estado del motor (un único modelo NLLB) para la interfaz.
pub fn list(dir: &Path) -> Vec<EngineInfo> {
    let downloaded = is_downloaded(dir);
    let base = model_path(dir);
    vec![EngineInfo {
        id: MODEL_DIR.to_string(),
        label: "NLLB-200 distilled 600M (NMT, 200 idiomas)".to_string(),
        approx_size_mb: APPROX_SIZE_MB,
        downloaded,
        path: downloaded.then(|| base.display().to_string()),
    }]
}

/// Descarga todos los archivos del modelo a `models/<MODEL_DIR>/`, informando un
/// avance global aproximado por `on_progress(descargado, total)` (denominador =
/// suma de tamaños declarados). Reusa el helper de descarga compartido.
pub fn download(dir: &Path, on_progress: impl Fn(u64, u64)) -> Result<EngineInfo, String> {
    let base = model_path(dir);
    let grand_total: u64 = FILES
        .iter()
        .map(|f| f.approx_size_mb as u64 * 1_000_000)
        .sum();
    // Bytes ya completados de archivos previos; la descarga es secuencial.
    let acc = Cell::new(0u64);
    on_progress(0, grand_total);
    for f in FILES.iter() {
        let url = format!("{BASE_URL}/{}", f.rel);
        let dest = base.join(f.rel);
        let bytes = crate::download::download_file(&url, &dest, |d, _| {
            on_progress((acc.get() + d).min(grand_total), grand_total);
        })?;
        acc.set(acc.get() + bytes);
    }
    on_progress(grand_total, grand_total);
    Ok(list(dir).into_iter().next().unwrap())
}

/// Tope duro de tokens generados por segmento, por si la generación no emite `</s>`.
const MAX_NEW_TOKENS: usize = 256;
const EOS_TOKEN: &str = "</s>";

/// Traduce el código ISO corto del proyecto (el que usa whisper y `languages.ts`)
/// al código FLORES-200 que entiende NLLB (p. ej. `es` -> `spa_Latn`).
pub fn nllb_code(short: &str) -> Result<&'static str, String> {
    let code = match short {
        "es" => "spa_Latn",
        "en" => "eng_Latn",
        "pt" => "por_Latn",
        "fr" => "fra_Latn",
        "de" => "deu_Latn",
        "it" => "ita_Latn",
        "ja" => "jpn_Jpan",
        "zh" => "zho_Hans",
        other => {
            return Err(format!(
                "El motor NLLB no tiene mapeado el idioma «{other}»."
            ))
        }
    };
    Ok(code)
}

/// Carga una sesión ONNX optimizada desde un archivo del modelo.
fn build_session(path: &Path) -> Result<Session, String> {
    Session::builder()
        .map_err(|e| format!("No se pudo preparar ONNX Runtime: {e}"))?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|e| format!("No se pudo configurar ONNX Runtime: {e}"))?
        .commit_from_file(path)
        .map_err(|e| format!("No se pudo cargar {}: {e}", path.display()))
}

/// Traduce todos los segmentos de la solicitud con el modelo NLLB en `model_dir`,
/// emitiendo `on_progress(hechos, total)` segmento a segmento. Carga el tokenizer
/// y ambas sesiones (encoder/decoder) una sola vez y reusa para todos los segmentos.
pub fn translate(
    model_dir: &Path,
    request: &TranslationRequest,
    on_progress: impl Fn(usize, usize),
) -> Result<(Vec<ResponseSegment>, EngineReport), String> {
    let total = request.segments.len();
    on_progress(0, total);

    let src_code = nllb_code(&request.source_language)?;
    let tgt_code = nllb_code(&request.target_language)?;

    let tokenizer = Tokenizer::from_file(model_dir.join("tokenizer.json"))
        .map_err(|e| format!("No se pudo cargar el tokenizer NLLB: {e}"))?;
    let eos_id = token_id(&tokenizer, EOS_TOKEN)?;
    let src_lang_id = token_id(&tokenizer, src_code)?;
    let tgt_lang_id = token_id(&tokenizer, tgt_code)?;

    let mut encoder = build_session(&model_dir.join("onnx/encoder_model_quantized.onnx"))?;
    let mut decoder = build_session(&model_dir.join("onnx/decoder_model_quantized.onnx"))?;

    let mut out = Vec::with_capacity(total);
    let mut report = EngineReport::default();
    for (i, segment) in request.segments.iter().enumerate() {
        let translation = if segment.source.trim().is_empty() {
            String::new()
        } else {
            translate_segment(
                &mut encoder,
                &mut decoder,
                &tokenizer,
                &segment.source,
                src_lang_id,
                tgt_lang_id,
                eos_id,
            )?
        };
        if translation.trim().is_empty() {
            report.failed += 1;
        } else {
            report.translated += 1;
        }
        out.push(ResponseSegment {
            id: segment.id.clone(),
            translation,
        });
        on_progress(i + 1, total);
    }
    Ok((out, report))
}

/// `id` de un token del vocabulario, como `i64` para los tensores int64 del grafo.
fn token_id(tokenizer: &Tokenizer, token: &str) -> Result<i64, String> {
    tokenizer
        .token_to_id(token)
        .map(|id| id as i64)
        .ok_or_else(|| format!("El tokenizer NLLB no conoce el token «{token}»."))
}

/// Traduce un único texto: encoder -> generación greedy en el decoder.
#[allow(clippy::too_many_arguments)]
fn translate_segment(
    encoder: &mut Session,
    decoder: &mut Session,
    tokenizer: &Tokenizer,
    text: &str,
    src_lang_id: i64,
    tgt_lang_id: i64,
    eos_id: i64,
) -> Result<String, String> {
    let encoded = tokenizer
        .encode(text, false)
        .map_err(|e| format!("No se pudo tokenizar el segmento: {e}"))?;
    // Formato de entrada NLLB: [idioma_origen] tokens… </s>.
    let mut ids: Vec<i64> = Vec::with_capacity(encoded.get_ids().len() + 2);
    ids.push(src_lang_id);
    ids.extend(encoded.get_ids().iter().map(|&t| t as i64));
    ids.push(eos_id);
    let src_len = ids.len() as i64;
    let mask = vec![1i64; ids.len()];

    let enc_ids = Tensor::from_array((vec![1, src_len], ids))
        .map_err(|e| format!("Tensor de entrada inválido: {e}"))?;
    let enc_mask = Tensor::from_array((vec![1, src_len], mask.clone()))
        .map_err(|e| format!("Tensor de máscara inválido: {e}"))?;
    let enc_out = encoder
        .run(ort::inputs!["input_ids" => enc_ids, "attention_mask" => enc_mask])
        .map_err(|e| format!("Falló el encoder NLLB: {e}"))?;
    let (hs_shape, hs_data) = enc_out[0]
        .try_extract_tensor::<f32>()
        .map_err(|e| format!("Salida del encoder inválida: {e}"))?;
    let hidden_shape = vec![hs_shape[0], hs_shape[1], hs_shape[2]];
    let hidden: Vec<f32> = hs_data.to_vec();
    drop(enc_out);

    // Generación greedy. El decoder arranca con [</s>, idioma_destino] (equivale al
    // `decoder_start_token_id` + `forced_bos_token_id` de NLLB en Hugging Face).
    let mut dec_ids: Vec<i64> = vec![eos_id, tgt_lang_id];
    let mut generated: Vec<u32> = Vec::new();
    let cap = (ids_budget(src_len)).min(MAX_NEW_TOKENS);
    for _ in 0..cap {
        // El decoder sin caché recibe la secuencia completa más los estados del
        // encoder en cada paso; no lleva entradas de caché por capa.
        let dec_ids_t = Tensor::from_array((vec![1, dec_ids.len() as i64], dec_ids.clone()))
            .map_err(tensor_err)?;
        let dec_mask_t =
            Tensor::from_array((vec![1, src_len], mask.clone())).map_err(tensor_err)?;
        let dec_hidden_t =
            Tensor::from_array((hidden_shape.clone(), hidden.clone())).map_err(tensor_err)?;
        let dec_out = decoder
            .run(ort::inputs![
                "input_ids" => dec_ids_t,
                "encoder_attention_mask" => dec_mask_t,
                "encoder_hidden_states" => dec_hidden_t,
            ])
            .map_err(|e| format!("Falló el decoder NLLB: {e}"))?;
        let (shape, logits) = dec_out[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Salida del decoder inválida: {e}"))?;
        let seq = shape[1] as usize;
        let vocab = shape[2] as usize;
        let last = &logits[(seq - 1) * vocab..seq * vocab];
        let next = argmax(last) as i64;
        drop(dec_out);
        if next == eos_id {
            break;
        }
        dec_ids.push(next);
        generated.push(next as u32);
    }

    let text = tokenizer
        .decode(&generated, true)
        .map_err(|e| format!("No se pudo detokenizar la traducción: {e}"))?;
    Ok(text.trim().to_string())
}

/// Presupuesto de tokens de salida proporcional al largo de la entrada.
fn ids_budget(src_len: i64) -> usize {
    src_len as usize * 2 + 16
}

/// Índice del valor máximo de un vector de logits.
fn argmax(logits: &[f32]) -> usize {
    logits
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Less))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn tensor_err(e: impl std::fmt::Display) -> String {
    format!("No se pudo construir un tensor del decoder: {e}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn model_path_es_el_subdirectorio_nllb() {
        let path = model_path(Path::new("/tmp/modelos"));
        assert!(path.ends_with(MODEL_DIR));
    }

    #[test]
    fn is_downloaded_exige_todos_los_archivos() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_downloaded(dir.path()));

        let base = model_path(dir.path());
        for f in FILES.iter() {
            let dest = base.join(f.rel);
            fs::create_dir_all(dest.parent().unwrap()).unwrap();
            fs::write(&dest, b"x").unwrap();
        }
        assert!(is_downloaded(dir.path()));

        let info = list(dir.path()).into_iter().next().unwrap();
        assert!(info.downloaded);
        assert!(info.path.is_some());
        assert_eq!(info.id, MODEL_DIR);
    }

    #[test]
    fn nllb_code_mapea_iso_corto_a_flores() {
        assert_eq!(nllb_code("es").unwrap(), "spa_Latn");
        assert_eq!(nllb_code("en").unwrap(), "eng_Latn");
        assert_eq!(nllb_code("zh").unwrap(), "zho_Hans");
        assert!(nllb_code("xx").is_err());
    }

    #[test]
    fn argmax_devuelve_el_indice_del_maximo() {
        assert_eq!(argmax(&[0.1, 0.9, 0.3]), 1);
        assert_eq!(argmax(&[2.0, 1.0, 1.5]), 0);
        assert_eq!(argmax(&[]), 0);
    }
}
