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
