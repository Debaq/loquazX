# Registro de uso de IA

Este archivo documenta el uso de herramientas de IA generativa en el desarrollo de loquazX. Se actualiza durante el desarrollo, no al final.

## Política

- Se usa IA cuando es útil y se declara siempre.
- Las decisiones de diseño centrales son humanas, no de la IA.
- Toda salida de IA se revisa, edita y testea antes de incluirse en el proyecto.
- La IA no se usa para conversaciones con revisores ni editores (excepto traducción, cuando el idioma sea barrera real).

## Resumen de herramientas usadas

| Herramienta            | Versión                | Usos principales                                          |
|------------------------|------------------------|-----------------------------------------------------------|
| Claude Opus 4.7 (1M)   | 2026-01                | Bootstrap del repositorio, scaffolding inicial, ADRs      |
| Claude Fable 5         | 2026-06                | Implementación del formato de proyecto (ADR-002)          |
| Claude Opus 4.8 (1M)   | 2026-06                | Traducción export/import (ADR-006) y modelo whisper (ADR-007) |

Esta tabla se actualizará cuando se sumen nuevas herramientas.

## Registro cronológico

### 2026-05-05 — Bootstrap del repositorio

**Herramienta:** Claude Opus 4.7 (1M context)

**Contexto:** Creación inicial del repositorio loquazX siguiendo las prácticas del lab-handbook. Se partió de un prototipo previo en Python (`subdub`) que validó el flujo de trabajo (transcripción con whisper, traducción, TTS con edge-tts y mezcla con ffmpeg).

**Aporte de la IA:** Generación del scaffold Tauri 2 + Vite + React, redacción inicial de README, CONTRIBUTING, CODE_OF_CONDUCT, CHANGELOG, plantillas de issues y pull requests, y borrador del workflow de CI. Borrador del ADR-001.

**Decisiones humanas:**

- Nombre del proyecto (`loquazX`).
- Stack: Tauri 2 + Rust + Vite + React + TypeScript.
- Licencia AGPL-3.0.
- Visibilidad pública desde el primer commit.
- Idioma de la documentación: español neutro.

**Revisión humana:** Cada archivo generado se revisará antes del primer push y los textos se ajustarán al estilo del lab-handbook.

**Commits asociados:** se enlazarán los SHAs una vez creados.

### 2026-05-05 — ADR-002 (formato de proyecto) e interfaz base

**Herramienta:** Claude Opus 4.7 (1M context)

**Contexto:** Smoke test del scaffold Tauri (verificación de que `npm run tauri dev` arranca y abre la ventana). A continuación, redacción de un ADR propuesto para definir el formato de proyecto y construcción de una interfaz base como punto de partida antes de implementar la carga de video y la extracción de audio.

**Aporte de la IA:** Borrador de ADR-002 con tres alternativas de formato y decisión por carpeta `.lqzx`. Estructura de componentes React (`TopBar`, `SegmentsList`, `VideoPreview`, `EditPanel`, `Transport`), tipo `Segment`, layout en grid con tema oscuro y datos demo. Sin lógica de carga ni invocación a comandos de Tauri todavía.

**Decisiones humanas:**

- Hacer smoke test antes de avanzar con cualquier feature.
- Priorizar la interfaz visible antes de la carga de video.
- ADR-002 queda en estado *Propuesta* hasta resolver puntos abiertos (copia vs. referencia del video, política de purga de runs).

**Revisión humana:** Verificación visual de la ventana Tauri tras los cambios y aprobación de los dos commits.

**Commits asociados:** `e60224b` (ADR-002), `91fb701` (interfaz base).

### 2026-06-11 — Implementación del formato de proyecto (ADR-002)

**Herramienta:** Claude Fable 5 (Claude Code)

**Contexto:** Primera funcionalidad con backend real: persistencia de proyectos según la estructura de carpeta `.lqzx` propuesta en ADR-002 (issue #2).

**Aporte de la IA:** Módulo `project.rs` con creación, apertura y guardado de segmentos (escritura atómica vía temporal + rename, validación de `format_version`), comandos Tauri `crear_proyecto`/`abrir_proyecto`/`guardar_segmentos`, seis tests unitarios, conexión de los botones Nuevo/Abrir/Guardar con los diálogos nativos del plugin `dialog`, y actualización de CHANGELOG.

**Decisiones humanas:**

- ADR-002 pasa a *Aceptada* (2026-06-11); los puntos abiertos (copia vs. referencia del video, purga de runs) se resolverán en ADRs o issues posteriores.
- Idiomas por defecto `es` → `en` hasta que exista selector en la UI.

**Revisión humana:** Revisado y aprobado por Nicolás en el PR #3.

**Commits asociados:** se enlazarán en el PR que cierra #2.

### 2026-06-11 — Importación y previsualización de video

**Herramienta:** Claude Fable 5 (Claude Code)

**Contexto:** Con el formato de proyecto mergeado (PR #3), siguiente paso del flujo: cargar el video original al proyecto (issue #4).

**Aporte de la IA:** Función `import_video` en `project.rs` con modos copia/referencia y campo opcional `source` en el manifiesto (compatible con proyectos previos), comando Tauri `importar_video`, cinco tests nuevos, reproducción del video en `VideoPreview` vía protocolo de assets (alcance `$HOME/**`), y diálogo de elección copiar/referenciar al importar.

**Decisiones humanas:**

- Continuar con la carga de video como siguiente feature.
- El punto abierto del ADR-002 (copia vs. referencia) se resuelve como lo prevé el propio ADR: preferencia del usuario al importar.

**Revisión humana:** Pendiente de revisión en el PR asociado antes de merge.

**Commits asociados:** se enlazarán en el PR que cierra #4.

### 2026-06-11 — Extracción de audio para whisper (ADR-003)

**Herramienta:** Claude Fable 5 (Claude Code)

**Contexto:** Con el video importado (PR #5), siguiente paso del pipeline: obtener el audio en WAV 16 kHz mono, el formato que whisper.cpp exige (issue #6).

**Aporte de la IA:** Borrador del ADR-003 (ffmpeg del sistema vs. sidecar vs. decodificación en Rust). Módulo `audio.rs` que invoca ffmpeg, función `extract_audio` en `project.rs` con campo opcional `audio` en el manifiesto, comando Tauri asíncrono `extraer_audio`, invalidación del audio al reimportar video, botón «Extraer audio» en la barra superior y cuatro tests nuevos (incluida verificación de la cabecera WAV). ffmpeg agregado a las dependencias de CI.

**Decisiones humanas:**

- Continuar con la extracción de audio como siguiente feature.
- Usar el ffmpeg del sistema por ahora; el empaquetado como sidecar se reevaluará al apuntar a Windows/macOS (documentado en ADR-003).

**Revisión humana:** Pendiente de revisión en el PR asociado antes de merge.

**Commits asociados:** se enlazarán en el PR que cierra #6.

### 2026-06-11 — Transcripción con whisper-rs (ADR-004)

**Herramienta:** Claude Fable 5 (Claude Code)

**Contexto:** Con el audio extraído (PR #7), siguiente paso del pipeline: transcribir `media/audio.wav` a segmentos con timing y texto (issue #8).

**Aporte de la IA:** Borrador del ADR-004 (modelo provisto por el usuario vs. empaquetado vs. descarga automática). Módulo `transcribe.rs` con `whisper-rs` 0.16 (carga del WAV vía `hound` con validación de formato, conversión a f32, mapeo de segmentos), función `transcribe` en `project.rs` que reemplaza `segments.json`, comando Tauri asíncrono `transcribir`, botón «Transcribir» con confirmación de reemplazo y recordatorio de la ruta del modelo en `localStorage`, cinco tests nuevos, `cmake`/`libclang-dev` en CI y documentación de requisitos en README.

**Decisiones humanas:**

- Continuar con la transcripción como siguiente feature, usando `whisper-rs` como prevé ADR-001.
- El modelo GGML lo provee el usuario; la descarga automática queda como mejora futura (documentado en ADR-004).
- El registro de runs en `runs/` se pospone hasta que haya más de una etapa generativa.

**Revisión humana:** Pendiente de revisión en el PR asociado antes de merge.

**Commits asociados:** se enlazarán en el PR que cierra #8.

### 2026-06-11 — Fix: el video no se reproducía en Linux (ADR-005)

**Herramienta:** Claude Fable 5 (Claude Code)

**Contexto:** Al probar la app tras el merge de la transcripción (PR #9), el video importado no cargaba: `MediaError code=4` en el `<video>` (issue #10).

**Aporte de la IA:** Diagnóstico instrumentando el frontend en caliente (el `fetch` al protocolo `asset` respondía 206 correcto, pero `GST_DEBUG` reveló `FormatError` en `MediaPlayerPrivateGStreamer`: WebKitGTK no enruta media por schemes custom). Borrador del ADR-005 (blob en memoria vs. `file://` vs. servidor HTTP local). Módulo `media_server.rs` (127.0.0.1, puerto efímero, token por sesión, allowlist de rutas canónicas, soporte de `Range`), comando `url_media`, `VideoPreview` consumiendo la URL local con error visible en pantalla, y seis tests del servidor. Verificación visual de la reproducción en la app real.

**Decisiones humanas:**

- Reporte del bug tras prueba manual.
- Servir media por HTTP local en todas las plataformas para tener un único camino de código (documentado en ADR-005).

**Revisión humana:** Pendiente de revisión en el PR asociado antes de merge.

**Commits asociados:** se enlazarán en el PR que cierra #10.

### 2026-06-11 — Etapa de traducción por export/import (ADR-006)

**Herramienta:** Claude Opus 4.8 (Claude Code)

**Contexto:** Con la transcripción mergeada (PR #11), siguiente paso del pipeline: rellenar el campo `translation` de cada segmento (issue #12).

**Aporte de la IA:** Borrador del ADR-006 (API en la nube vs. motor local embebido vs. export/import de JSON a un LLM externo). Módulo `translation.rs` con tipos versionados (`TranslationRequest`/`TranslationResponse`), `build_request` (incluye los tiempos para ajustar el largo al doblaje), `build_prompt` en español y `apply_response` tolerante con reporte de cruce; funciones `export_translation`/`import_translation` en `project.rs`, comandos Tauri `exportar_traduccion`/`importar_traduccion`, botones «Exportar para traducir» e «Importar traducción» en la barra superior con persistencia previa de los segmentos editados, nueve tests nuevos y actualización de CHANGELOG y README.

**Decisiones humanas:**

- Continuar con la traducción como siguiente feature.
- La app no traduce ni hace red: exporta el trabajo a un LLM externo a elección del usuario; un motor de traducción local queda como mejora futura (export ahora, local después).
- Incluir los tiempos `start`/`end` en la solicitud para que el LLM ajuste el largo de la traducción.

**Revisión humana:** Pendiente de revisión en el PR asociado antes de merge.

**Commits asociados:** se enlazarán en el PR que cierra #12.

### 2026-06-11 — Descarga y gestión del modelo whisper (ADR-007)

**Herramienta:** Claude Opus 4.8 (Claude Code)

**Contexto:** La selección manual del archivo GGML en cada transcripción (ADR-004) resultó incómoda; se pidió una configuración que descargue un modelo por nivel y lo guarde para usarlo siempre (issue #14).

**Aporte de la IA:** Borrador del ADR-007 (seguir manual vs. empaquetar vs. descargar y persistir). Módulo `models.rs` (niveles `tiny`–`large-v3`, descarga con `reqwest`/`rustls` a `app_data/models/` con escritura atómica `.part` + rename y progreso por callback, importación de `.bin` propio, borrado; seis tests). Comandos Tauri `listar_modelos`/`descargar_modelo` (emite `modelo:progreso`)/`importar_modelo`/`eliminar_modelo`; `transcribir` pasa a recibir el nivel y resolver el modelo guardado. Componente `ModelManager` (modal con descarga/importación/borrado, barra de progreso por eventos y selección del nivel «en uso»), botón «Modelo» en la barra y estilos. Actualización de CHANGELOG, README y nota de reemplazo en ADR-004.

**Decisiones humanas:**

- Reemplazar el selector de archivo por la descarga, conservando un fallback de importación de `.bin` propio.
- Niveles `tiny`–`large-v3` con `base` por defecto.

**Revisión humana:** Pendiente de revisión en el PR asociado antes de merge.

**Commits asociados:** se enlazarán en el PR que cierra #14.

### 2026-06-12 — Numeración del flujo y decisión del motor TTS (ADR-009)

**Herramienta:** Claude Opus 4.8 (Claude Code)

**Contexto:** Antes de implementar el doblaje se detectó que la barra superior numeraba los pasos saltándose «Importar video» (que vivía solo en el panel central). En paralelo se definió el motor de síntesis de voz para la etapa de doblaje.

**Aporte de la IA:** Fix de la barra superior: botón «Importar video» (paso 2, icono `Film`) y numeración del selector de idioma de origen (paso 4), renumeración del resto (extraer audio→3, transcribir→5, traducir→6) y eliminación del botón redundante de `VideoPreview`. Borrador del ADR-009 (edge-tts online vs. Piper local vs. XTTS-v2 vs. dos motores tras firma común). Selector de voz en `EditPanel` con Piper (default local), edge-tts (opt-in online) y XTTS-v2 deshabilitado, con guía de cuál usar. Implementación de la descarga de voces Piper: helper de descarga compartido `download.rs` (refactor de `models.rs` para reusarlo), módulo `voices.rs` (lista/descarga `.onnx`+`.onnx.json`/borrado, con tests), comandos Tauri `listar_voces`/`descargar_voz`/`eliminar_voz`, y conversión del modal «Modelo» en «Modelos y voces» con pestañas (Transcripción · Voces). Revisión del ADR-008: la traducción local pasa de LLM GGUF/llama.cpp (lento y pesado en CPU) a NMT NLLB-200 vía ONNX, reusando el `ort` que entra para Piper como backend de inferencia compartido. Previsualización de voces edge-tts: módulo `tts_edge.rs` (crate `msedge-tts`, blocking), comandos `listar_voces_edge`/`probar_voz_edge`, y pestaña «Voces edge-tts (online)» en el modal con filtro por idioma, texto de muestra y reproducción del mp3 por `url_media`. Ajuste de layout del modal para que solo la lista interior se desplace.

**Decisiones humanas:**

- Reporte del hueco de numeración y orden de quitar el botón redundante.
- Motor TTS: dos motores tras una firma común — Piper local por defecto (cumple ADR-001 por construcción) y edge-tts online como opt-in explícito (Rust puro, sin sidecar Python). XTTS-v2 (clonación de voz) queda visible pero deshabilitado como feature futuro.

**Revisión humana:** Pendiente de revisión en el PR asociado antes de merge.

**Commits asociados:** se enlazarán en el PR asociado.

### 2026-06-12 — Inferencia del motor de traducción local NLLB (ADR-008)

**Herramienta:** Claude Opus 4.8 (Claude Code)

**Contexto:** Con la descarga del modelo NLLB ya operativa, faltaba la pieza central del ADR-008 revisado: usar el modelo para traducir dentro de la app, sin red. `translate_engine::translate` era un *stub* que devolvía «no implementado».

**Aporte de la IA:** Implementación de la inferencia seq2seq sobre `ort` (ONNX Runtime 2.0-rc) y `tokenizers`: carga del encoder y el decoder cuantizados (variante sin caché `decoder_model_quantized.onnx`, porque `ort` no permite construir los tensores de caché vacíos —dim 0— que exigiría el decoder fusionado), tokenización con el formato NLLB (`[idioma_origen] tokens… </s>`), generación greedy en el decoder arrancando con `[</s>, idioma_destino]` (equivalente a `decoder_start_token_id` + `forced_bos_token_id`), y detokenización. Mapeo de códigos ISO cortos del proyecto a códigos FLORES-200 (`es`→`spa_Latn`, etc.). Cableado del botón «Traducir con IA local» (paso 6) en `TopBar`/`App.tsx`: verifica que el modelo esté descargado, persiste lo editado, escucha `traduccion:progreso` para mostrar el avance segmento a segmento y vuelca el resultado con `apply_response`. Tests de `nllb_code` y `argmax`. Entradas de CHANGELOG y este registro.

**Decisiones humanas:**

- Orden de implementar la inferencia ahora que la descarga funciona.

**Revisión humana:** Pendiente de revisión en el PR asociado antes de merge. La inferencia end-to-end requiere el modelo descargado (~900 MB) y se valida manualmente sobre un proyecto real; los tests automáticos cubren el mapeo de idiomas y utilidades, no la corrida ONNX.

**Commits asociados:** se enlazarán en el PR asociado.

---

### 2026-06-12 — Generación del doblaje por segmento (ADR-009)

**Herramienta:** Claude Opus 4.8 (Claude Code)

**Contexto:** Con la traducción resuelta y el andamiaje de voces ya en su sitio (descarga Piper en `voices.rs`, previsualización edge-tts en `tts_edge.rs`), faltaba la pieza central del ADR-009: convertir la traducción de cada segmento en audio. El selector de voz del `EditPanel` y la pista de doblaje de la `Timeline` estaban deshabilitados.

**Aporte de la IA:** Firma común de síntesis `tts::synth_segment` que despacha Piper (local, por defecto) o edge-tts (online, opt-in) y deja un WAV mono ajustado al hueco del segmento. Piper se sintetiza con el crate `piper-rs` (fonemización espeak-ng embebida + inferencia ONNX sobre el mismo `ort` de ADR-008); edge-tts reutiliza `tts_edge` (mp3) y se transcodifica a WAV. Helpers ffmpeg en `audio.rs` (`transcode_to_wav`, `fit_duration` con cadena `atempo` acotada, `wav_duration`) refactorizando la invocación común. Orquestación por proyecto en `project.rs` (`generate_dub` masivo y `generate_dub_segment`, WAV determinista en `runs/dub/<id>.wav`, `Project.dubs`). Comandos Tauri `generar_doblaje` (asíncrono, evento `doblaje:progreso`) y `generar_doblaje_segmento`. Frontend: `EditPanel` con motor + voz (Piper descargadas / edge por idioma) y generación + reproducción por segmento; `Timeline` con clips de doblaje y botón de generación masiva con progreso; carga y selección de voces compartida en `App.tsx`. Enlace de `pcaudio`/`sonic` en `build.rs` (espeak-ng en Linux los referencia sin emitir el enlace). Tests de `atempo_chain`, `wav_duration`, deserialización de motor/ajustes y un smoke test `#[ignore]` de Piper end-to-end.

**Decisiones humanas:**

- Implementar **ambos** motores en este PR tras la firma común (no solo uno).
- Doblar solo al `target_language` actual; el multi-idioma de salida queda para después.

**Revisión humana:** Pendiente de revisión en el PR asociado antes de merge. La síntesis Piper end-to-end se verificó localmente con la voz `it_IT-riccardo-x_low` (WAV real ajustado al hueco con `atempo`); los tests automáticos cubren utilidades y el ajuste de duración, no la corrida ONNX (smoke test ignorado por defecto).

**Commits asociados:** se enlazarán en el PR asociado.

---

### 2026-07-02 — Render de presentación con PDF + segmentos (ADR-010)

**Herramienta:** Claude Opus 4.8 (Claude Code)

**Contexto:** Feature pedida por Nicolás tras el merge del doblaje (PR #17): un usuario con un PDF y los textos de la charla debería poder armar un video narrado en otro idioma sin pasar por grabar video. La idea es que un proyecto `.lqzx` pueda traer un PDF de fondo además (o en lugar) del video, y que cada segmento lleve el número de página del PDF que se muestra durante su `start`/`end`. Issue #18.

**Aporte de la IA:** Borrador del ADR-010 (extensión del formato de proyecto: campo `slides` opcional en el manifiesto y `slide` opcional por segmento). Asume ffmpeg (ADR-003) y suma pdftoppm y pdfinfo de poppler como dependencia externa para el render. Nuevo módulo Rust `presentacion.rs` que rasteriza el PDF con `pdftoppm`, concatena los WAV de doblaje de `runs/dub/` con silencios en los huecos (filter `concat` + `atrim`/`asetpts` para acotar cada entrada), y compone el mp4 final con libx264 + aac. Filtro `scale=trunc(iw/2)*2:trunc(ih/2)*2` para tolerar PDFs no múltiplos de 2 y `-movflags +faststart` para que abra rápido al servirse por el `MediaServer` (ADR-005). Comandos Tauri `importar_pdf` (copia bajo `slides/`, lee `page_count`), `conteo_paginas_pdf` (para validar antes de confirmar), `importar_audio_presentacion` (audio arbitrario sin video, reusa `extract_wav_16k`), `importar_segmentos_json` (importa `{start, end, slide, source}` con `id = uuid` y `translation = ""`) y `renderizar_presentacion` (asíncrono, emite `presentacion:progreso` por etapa). El mp4 queda en `exports/<nombre>.mp4`. Frontend: cuatro botones nuevos en la `TopBar` («Importar PDF» · «Importar audio» · «Importar segmentos JSON» · «Exportar video»), campo numérico «Diapositiva» (1–page_count) en el `EditPanel`, etiqueta `p.N` en la lista de segmentos, y preview del PDF en `VideoPreview` cuando no hay video (muestra la página del segmento seleccionado, con fallback a la última vista o la 1). Test de integración E2E `#[ignore]` en `tests/render_presentacion.rs` que valida el pipeline completo contra `pdftoppm`/`pdfinfo`/`ffmpeg`.

**Decisiones humanas:**

- Misma plantilla de proyecto (no formato aparte), siguiendo el patrón "fuente opcional" que ya se adoptó para `source`.
- Render en backend Rust con `ffmpeg` (consistente con el resto del proyecto).
- Extender `Segment` con `slide` opcional (mínimo cambio al esquema).
- Numeración 1‑based para `slide`.
- La app no graba audio: si el usuario no tiene video, importa un audio arbitrario o escribe los segmentos a mano; la transcripción por whisper queda fuera de este modo (se documenta como limitación).

**Revisión humana:** Pendiente de revisión en el PR asociado antes de merge. La generación end-to-end del mp4 se valida manualmente con un PDF y un par de segmentos doblados; los tests automáticos cubren utilidades del pipeline de audio y el parseo del JSON de import.

**Commits asociados:** se enlazarán en el PR que cierra #18.

### 2026-07-02 — Recalibración de timings al audio natural (ADR-010)

**Herramienta:** Claude Opus 4.8 (Claude Code)

**Contexto:** Nicolás reportó que en el modo presentación, los `start`/`end` originales hacen que el audio se comprima o se estire (con `atempo`), perdiendo naturalidad. Como el modo presentación tiene control total del tiempo (no hay video que limite), propuso un sistema de recalibración: dejar que el audio dictate la duración de cada segmento. Está usando la API de Microsoft (edge-tts).

**Aporte de la IA:** Refactor de `tts::synth_segment` para aceptar `target_secs: Option<f64>`: si es `Some`, mantiene el comportamiento actual (ajusta con `atempo`); si es `None`, deja el WAV a velocidad natural. Nuevo helper público `compute_recalibrated_timings` separado de la orquestación de TTS para que el cálculo sea testeable de forma determinista. Nuevo comando Tauri `recalibrar_audios` (asíncrono con `spawn_blocking`, emite `recalibrar:progreso` por segmento) que sintetiza cada segmento a velocidad natural, mide la duración y reasigna los `start`/`end` cumulativamente con un gap fijo de 0.2s entre segmentos. Los segmentos sin texto respetan su `start`/`end` original (silencio); los recalibrados van en orden cronológico y se ponen uno tras otro. Antes de modificar, hace un backup único de `segments.json` en `segments.original.json` (para que llamadas subsiguientes sean idempotentes). Comandos Tauri asociados: `restaurar_timings_originales` (lee el backup y restaura) y `tiene_backup_timings` (para que la UI muestre el botón "Restaurar" solo si hay algo que restaurar). Frontend: dos botones nuevos en la `TopBar` ("Recalibrar" con icono `Sliders`, "Restaurar" con icono `RotateCcw` este último condicional). Tres tests unitarios nuevos del cálculo + un test E2E que cubre backup/restore. Documentación ampliada en ADR-010 con la subsección "Recalibración de timings" explicando la decisión, el flujo y los trade-offs (en particular: el mp4 recalibrado tiene duración = suma de audios + gaps + silencios, distinta del mp4 ajustado al timeline original).

**Decisiones humanas:**

- Recalibrar deja que el audio dictate la duración, no al revés: la calidad de la voz es la prioridad.
- Gap fijo de 0.2s entre segmentos (suficiente para una respiración natural, no tan grande como para alargar el video innecesariamente).
- Segmentos sin texto respetan su `start`/`end` original (silencio en su lugar, no se mueven).
- Backup único en `segments.original.json`: la primera recalibración es la "original"; las siguientes son idempotentes.
- La recalibración NO modifica los `runs/dub/*.wav` que ya estén en disco: si el usuario quiere re-doblar con los timings viejos, regenera manualmente.

**Revisión humana:** Pendiente de revisión en el PR asociado antes de merge. La verificación end-to-end con TTS real queda del lado del usuario (edge-tts con su cuenta de Microsoft); los tests automáticos cubren el cálculo de timings y el flujo de backup/restore.

**Commits asociados:** se enlazarán en el PR asociado.

---

## Plantilla en blanco para nuevas entradas

```
### YYYY-MM-DD — [Descripción breve]

**Herramienta:** [Nombre y versión]

**Contexto:** [Qué se estaba haciendo].

**Aporte de la IA:** [Qué generó].

**Decisiones humanas:** [Qué decidió la persona].

**Revisión humana:** [Qué se revisó y modificó].

**Commits asociados:** [SHAs o enlaces].
```
