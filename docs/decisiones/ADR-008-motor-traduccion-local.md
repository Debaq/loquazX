# ADR-008: Motor de traducción local embebido

- Fecha: 2026-06-11
- Estado: Revisada (ver «Revisión 2026-06-12»)
- Decisor: Nicolás Baier

> **Revisión 2026-06-12 — se reemplaza la decisión.** La elección original
> (LLM instruct local vía `llama.cpp`/GGUF, alternativa 3) se descarta: un
> instruct de 2-7B en CPU es lento (~3-8 tok/s) y pesado (2-5 GB RAM), y como la
> traducción se hace **segmento a segmento**, un video largo tardaría minutos u
> horas en máquinas modestas, para una calidad apenas mediocre. Las URLs de los
> modelos GGUF nunca se concretaron (`repo_file: "TODO/..."`).
>
> **Nueva decisión:** motor **NMT dedicado vía ONNX**, concretamente
> **NLLB-200-distilled-600M** (~600 MB, 200 idiomas en un solo modelo, <1 s por
> segmento en CPU, ~1-2 GB RAM). Esto retoma la alternativa 1 (NMT dedicado) pero
> ejecutándola sobre **ONNX Runtime** (`ort`) en vez de `candle`, porque ese mismo
> runtime ya entra para el motor de voz Piper (ADR-009): **`ort` pasa a ser el
> backend de inferencia compartido** de TTS y traducción, así que el NMT no agrega
> una dependencia nativa nueva, reusa la de Piper.
>
> Lo demás de ADR-008 se mantiene: el formato de segmentos no cambia, el motor
> sigue rellenando `translation` y aplicándolo con `apply_response`, coexiste con
> el export/import de ADR-006, y la descarga del modelo reutiliza la plomería de
> ADR-007 (`download::download_file`). El esqueleto `translate_engine.rs` se
> reescribe detrás de las mismas firmas (`list`/`download`/`translate`) para NLLB.
> La familia GGUF instruct (`mt-*.gguf`) se reemplaza por los artefactos ONNX de
> NLLB + su tokenizer.
>
> **Estado de implementación (2026-06-12):** descarga e inferencia **operativas**.
> La inferencia carga el encoder y el decoder cuantizados con `ort`, tokeniza con
> el formato NLLB (`[idioma_origen] tokens… </s>`) y genera greedy arrancando el
> decoder con `[</s>, idioma_destino]`. Se usa la variante **sin caché** del
> decoder (`decoder_model_quantized.onnx`, sin entradas de caché por capa): es
> O(n²) por segmento pero simple y correcta, aceptable porque los segmentos son
> cortos. La generación con caché de claves/valores queda como optimización futura
> tras la misma firma `translate`.

## Contexto

ADR-006 resolvió la etapa de traducción sin que la app haga red: exporta un JSON
de solicitud más un prompt para que el usuario lo lleve al LLM que elija, y luego
importa la respuesta JSON emparejando por `id`. Ese mismo ADR dejó anotado, como
mejora futura, un **motor de traducción local embebido**: el formato de segmentos
no cambia, así que un motor local puede rellenar el mismo campo `translation`
reutilizando la estructura existente.

Este ADR define ese motor. El objetivo es que el usuario pueda traducir dentro de
la app, sin red y sin pegar JSON a mano, conservando el principio de ADR-001 (no
enviar contenido del usuario a servicios externos) ahora *por construcción*, no
solo por convención: la traducción ocurre en su máquina.

ADR-007 ya construyó toda la plomería que un motor local necesita: descarga de
modelos por nivel desde Hugging Face con progreso, almacén persistente en
`app_data/models/`, importación de un archivo propio, borrado y ejecución en
`spawn_blocking` para no congelar la UI. El motor de traducción reutiliza ese
patrón con otra familia de modelos.

## Alternativas evaluadas

1. **NMT puro vía `candle` (Rust puro, modelos `safetensors`).** Modelos NMT
   dedicados (NLLB-200-distilled, OPUS-MT/Marian). Livianos (300-600 MB) y
   rápidos, pero traducen frase a frase sin entender la restricción de duración
   para doblaje, y exigen integrar tokenizers y la arquitectura del modelo a mano
   (más código nuevo, sin reusar la plomería de ADR-007 tal cual).
2. **Sidecar Python (CTranslate2 / argostranslate).** Calidad NMT madura, pero
   empaquetar un intérprete de Python multiplataforma rompe el principio de «todo
   en Rust» y complica el instalador y CI.
3. **LLM instruct local vía `llama.cpp` (modelos GGUF).** Mismo ecosistema que
   whisper.cpp (ADR-004/007). Un crate de binding (`llama-cpp-2`) carga un modelo
   instruct chico en GGUF descargado desde Hugging Face, y se le da **el mismo
   prompt de ADR-006** ejecutado localmente. Reutiliza descarga, almacén, niveles,
   progreso y `spawn_blocking` sin tocarlos. Contra: los modelos pesan más
   (1-3 GB) y la inferencia en CPU es más lenta que un NMT dedicado.

## Decisión

Se elige la **alternativa 3 (LLM instruct local vía `llama.cpp`/GGUF)** para la
primera versión del motor de traducción local, por coherencia con el stack
existente y porque reaprovecha el prompt y la plomería ya construidos.

- **Coexistencia con ADR-006:** el flujo de export/import **se mantiene** como
  alternativa para quien prefiera otro LLM o no quiera descargar el modelo. El
  motor local es una vía adicional, no un reemplazo.
- **Formato sin cambios:** el motor produce los mismos `ResponseSegment
  { id, translation }` y los aplica con el `apply_response` existente
  (`translation.rs`). La importación, el merge tolerante por `id` y `segments.json`
  no cambian.
- **Modelos:** familia GGUF instruct chica (p. ej. `qwen2.5-3b-instruct`,
  `gemma-2-2b-instruct`) descargada desde Hugging Face. Los niveles de traducción
  son **independientes** de los de whisper; viven en el mismo almacén
  `app_data/models/` con un prefijo propio (`mt-<nivel>.gguf`) para no chocar con
  `ggml-*.bin`.
- **Prompt:** se reutiliza el de ADR-006 (`build_prompt`), adaptado a un turno de
  chat instruct. El modelo recibe los tiempos de cada segmento para ajustar el
  largo de la traducción al doblaje/subtítulo, igual que el flujo manual.
- **Ejecución:** en el backend con el binding de `llama.cpp`, dentro de
  `spawn_blocking`, emitiendo progreso por segmento a la UI con un evento
  `traduccion:progreso` (`traducidos`/`total`), análogo a `modelo:progreso`.
- **Descarga del modelo:** reutiliza el gestor de ADR-007 extendido para conocer
  los niveles de traducción; la UI los lista junto a los de whisper.

## Consecuencias

- La app puede traducir sin red, cumpliendo ADR-001 por construcción para quien
  use el motor local.
- Nueva dependencia de backend (`llama-cpp-2` o equivalente) y modelos GGUF
  pesados; la primera traducción exige descargar el modelo (igual que whisper).
- La inferencia en CPU puede ser lenta en máquinas modestas; se acepta para la
  primera versión. La aceleración por GPU queda como mejora futura.
- El esqueleto inicial (`translate_engine.rs`) entra con funciones *stub* que
  devuelven un error «no implementado» para mantener CI verde sin arrastrar aún la
  dependencia nativa; la integración real de `llama.cpp` se hace en un PR
  siguiente, detrás de las mismas firmas.
- La calidad de un LLM instruct chico puede quedar por debajo de un NMT dedicado o
  de un LLM grande; si se confirma, un motor NMT (alternativa 1) puede sumarse
  después detrás de la misma firma `translate`, sin cambiar el formato.
