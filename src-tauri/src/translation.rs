//! Etapa de traducción (ADR-006): la app no traduce. Construye una solicitud
//! JSON con los segmentos y un prompt para un LLM externo, y aplica la respuesta
//! traducida sobre los segmentos emparejando por `id`.

use crate::project::Segment;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const REQUEST_SCHEMA: &str = "loquazx.translation.request/1";
pub const RESPONSE_SCHEMA: &str = "loquazx.translation.response/1";

/// Un segmento tal como se exporta para traducir: tiempos en segundos y texto
/// origen. Los tiempos viajan para que el LLM ajuste el largo de la traducción.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestSegment {
    pub id: String,
    pub start: f64,
    pub end: f64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationRequest {
    pub schema: String,
    pub source_language: String,
    pub target_language: String,
    pub segments: Vec<RequestSegment>,
}

/// La respuesta del LLM: cada `id` de la solicitud con su traducción.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSegment {
    pub id: String,
    pub translation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResponse {
    /// Tolerante: la respuesta del LLM podría omitir el esquema.
    #[serde(default)]
    pub schema: String,
    #[serde(default)]
    pub target_language: String,
    pub segments: Vec<ResponseSegment>,
}

/// Resumen del cruce de la respuesta con los segmentos del proyecto.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MergeReport {
    /// Segmentos a los que se les asignó una traducción no vacía.
    pub translated: usize,
    /// Segmentos del proyecto que quedaron sin traducción tras aplicar la respuesta.
    pub missing: usize,
    /// `id` presentes en la respuesta que no existen en el proyecto.
    pub unknown: usize,
}

/// Construye la solicitud de traducción a partir de los segmentos del proyecto.
pub fn build_request(
    source_language: &str,
    target_language: &str,
    segments: &[Segment],
) -> TranslationRequest {
    TranslationRequest {
        schema: REQUEST_SCHEMA.to_string(),
        source_language: source_language.to_string(),
        target_language: target_language.to_string(),
        segments: segments
            .iter()
            .map(|s| RequestSegment {
                id: s.id.clone(),
                start: s.start,
                end: s.end,
                source: s.source.clone(),
            })
            .collect(),
    }
}

/// Redacta el prompt en español para pegar en un LLM externo junto a la solicitud.
pub fn build_prompt(request: &TranslationRequest) -> String {
    format!(
        "# Tarea: traducción de subtítulos para doblaje\n\
\n\
Eres un traductor audiovisual profesional. Traduce del idioma `{origen}` al idioma \
`{destino}` los segmentos del archivo `traduccion-solicitud.json` (esquema \
`{esquema_req}`).\n\
\n\
Cada segmento trae:\n\
- `id`: identificador único, NO lo cambies.\n\
- `start` y `end`: inicio y fin en segundos. Su diferencia es el tiempo disponible \
en pantalla; ajusta el largo de la traducción para que sea pronunciable en doblaje \
y legible como subtítulo.\n\
- `source`: el texto original a traducir.\n\
\n\
Reglas:\n\
1. Conserva exactamente cada `id`. Un único `translation` por `id`.\n\
2. No fundas, dividas ni reordenes segmentos: la respuesta tiene la misma cantidad \
de elementos y el mismo orden que la solicitud.\n\
3. Traducción natural y fluida en `{destino}`; respeta nombres propios y términos técnicos.\n\
4. No agregues comentarios, notas ni markdown. Responde **solo** con un JSON válido \
con esta forma (esquema `{esquema_resp}`):\n\
\n\
```json\n\
{{\n\
  \"schema\": \"{esquema_resp}\",\n\
  \"target_language\": \"{destino}\",\n\
  \"segments\": [\n\
    {{ \"id\": \"<id del segmento>\", \"translation\": \"<traducción en {destino}>\" }}\n\
  ]\n\
}}\n\
```\n\
\n\
Guarda esa respuesta como un archivo `.json` e impórtalo en loquazX con «Importar traducción».\n",
        origen = request.source_language,
        destino = request.target_language,
        esquema_req = REQUEST_SCHEMA,
        esquema_resp = RESPONSE_SCHEMA,
    )
}

/// Aplica la respuesta sobre los segmentos (emparejando por `id`) y devuelve los
/// segmentos actualizados junto a un reporte del cruce. Es tolerante con
/// discrepancias: no aborta si faltan o sobran `id`.
pub fn apply_response(
    mut segments: Vec<Segment>,
    response: &TranslationResponse,
) -> (Vec<Segment>, MergeReport) {
    let mut by_id: HashMap<&str, &str> = HashMap::new();
    for r in &response.segments {
        // El último gana si la respuesta repite un `id`.
        by_id.insert(r.id.as_str(), r.translation.as_str());
    }

    let known: std::collections::HashSet<&str> = segments.iter().map(|s| s.id.as_str()).collect();
    let unknown = response
        .segments
        .iter()
        .filter(|r| !known.contains(r.id.as_str()))
        .count();

    for segment in &mut segments {
        if let Some(translation) = by_id.get(segment.id.as_str()) {
            segment.translation = (*translation).to_string();
        }
    }

    let translated = segments
        .iter()
        .filter(|s| !s.translation.trim().is_empty())
        .count();
    let missing = segments.len() - translated;

    (
        segments,
        MergeReport {
            translated,
            missing,
            unknown,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(id: &str, source: &str) -> Segment {
        Segment {
            id: id.into(),
            start: 0.0,
            end: 2.0,
            source: source.into(),
            translation: String::new(),
            slide: None,
        }
    }

    #[test]
    fn build_request_copia_tiempos_y_texto() {
        let req = build_request("es", "en", &[seg("a", "Hola")]);
        assert_eq!(req.schema, REQUEST_SCHEMA);
        assert_eq!(req.source_language, "es");
        assert_eq!(req.target_language, "en");
        assert_eq!(req.segments.len(), 1);
        assert_eq!(req.segments[0].id, "a");
        assert_eq!(req.segments[0].source, "Hola");
        assert_eq!(req.segments[0].end, 2.0);
    }

    #[test]
    fn build_prompt_menciona_idiomas_y_esquema() {
        let req = build_request("es", "en", &[]);
        let prompt = build_prompt(&req);
        assert!(prompt.contains("`es`"));
        assert!(prompt.contains("`en`"));
        assert!(prompt.contains(RESPONSE_SCHEMA));
        assert!(prompt.contains("Conserva exactamente cada `id`"));
    }

    #[test]
    fn apply_response_rellena_por_id() {
        let segments = vec![seg("a", "Hola"), seg("b", "Mundo")];
        let response = TranslationResponse {
            schema: RESPONSE_SCHEMA.into(),
            target_language: "en".into(),
            segments: vec![
                ResponseSegment {
                    id: "a".into(),
                    translation: "Hello".into(),
                },
                ResponseSegment {
                    id: "b".into(),
                    translation: "World".into(),
                },
            ],
        };
        let (out, report) = apply_response(segments, &response);
        assert_eq!(out[0].translation, "Hello");
        assert_eq!(out[1].translation, "World");
        assert_eq!(report.translated, 2);
        assert_eq!(report.missing, 0);
        assert_eq!(report.unknown, 0);
    }

    #[test]
    fn apply_response_reporta_faltantes_y_desconocidos() {
        let segments = vec![seg("a", "Hola"), seg("b", "Mundo")];
        let response = TranslationResponse {
            schema: String::new(),
            target_language: String::new(),
            segments: vec![
                ResponseSegment {
                    id: "a".into(),
                    translation: "Hello".into(),
                },
                ResponseSegment {
                    id: "zzz".into(),
                    translation: "Fantasma".into(),
                },
            ],
        };
        let (out, report) = apply_response(segments, &response);
        assert_eq!(out[0].translation, "Hello");
        assert_eq!(out[1].translation, "");
        assert_eq!(report.translated, 1);
        assert_eq!(report.missing, 1);
        assert_eq!(report.unknown, 1);
    }

    #[test]
    fn apply_response_no_borra_traduccion_previa_si_falta_en_respuesta() {
        let mut previo = seg("a", "Hola");
        previo.translation = "Hello".into();
        let response = TranslationResponse {
            schema: String::new(),
            target_language: String::new(),
            segments: vec![],
        };
        let (out, report) = apply_response(vec![previo], &response);
        assert_eq!(out[0].translation, "Hello");
        assert_eq!(report.translated, 1);
        assert_eq!(report.missing, 0);
    }
}
