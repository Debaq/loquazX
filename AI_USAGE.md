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
