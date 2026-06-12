# Changelog

Todos los cambios notables de este proyecto se documentan en este archivo.

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el versionado sigue [Semantic Versioning](https://semver.org/lang/es/).

## [Unreleased]

### Added

- Formato de proyecto en carpeta `.lqzx` según ADR-002: `project.json` con `format_version`, `segments.json` y subdirectorios `source/`, `media/`, `runs/`, `exports/`.
- Comandos Tauri `crear_proyecto`, `abrir_proyecto` y `guardar_segmentos` con tests unitarios.
- Botones Nuevo, Abrir y Guardar de la barra superior conectados a diálogos nativos.
- Importación de video al proyecto (copia a `source/` o referencia a la ruta original, a elección del usuario) y reproducción en el panel de previsualización.
- Extracción de audio del video a `media/audio.wav` (WAV 16 kHz mono, el formato de entrada de whisper.cpp) usando el ffmpeg del sistema según ADR-003, con botón «Extraer audio» en la barra superior. Al reimportar un video, el audio extraído previo se invalida.
- Transcripción del audio a segmentos con `whisper-rs` según ADR-004: el idioma sale del manifiesto y el resultado reemplaza `segments.json` previa confirmación. Botón «Transcribir» en la barra superior.
- Gestión del modelo whisper por descarga según ADR-007: nuevo gestor «Modelo» que descarga el nivel elegido (`tiny`, `base`, `small`, `medium`, `large-v3`; por defecto `base`) desde Hugging Face con barra de progreso y lo guarda de forma persistente en el directorio de datos de la app para reutilizarlo siempre. Permite importar un `.bin` propio (offline) y borrar modelos. La transcripción usa el modelo guardado del nivel elegido sin pedir un archivo. Módulo `models.rs` y comandos Tauri `listar_modelos`/`descargar_modelo`/`importar_modelo`/`eliminar_modelo`.

### Changed

- La transcripción ya no abre un diálogo para seleccionar el archivo del modelo en cada uso: toma el modelo descargado del nivel configurado (ADR-007, reemplaza la gestión de modelo de ADR-004). La clave `localStorage` pasó de `loquazx.whisperModel` (ruta) a `loquazx.whisperLevel` (nivel).

### Fixed

- El video importado no se reproducía en Linux: WebKitGTK entrega las URI de media a GStreamer sin pasar por el protocolo `asset` de Tauri. Ahora el media se sirve por un servidor HTTP local con soporte de rangos (ADR-005) y el panel muestra el error de reproducción si lo hay.

## [0.1.0] - 2026-05-05

### Added

- Estructura inicial del repositorio basada en la plantilla del lab-handbook.
- Scaffold de Tauri 2 con Vite, React y TypeScript.
- Documentos del proyecto: README, CONTRIBUTING, CODE_OF_CONDUCT, AI_USAGE, CHANGELOG.
- Licencia AGPL-3.0.
- Plantillas de issues y pull requests.
- Workflow de integración continua que compila el frontend y verifica el código de Rust.
- ADR-001 con la decisión inicial del stack.

[Unreleased]: https://github.com/Debaq/loquazX/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Debaq/loquazX/releases/tag/v0.1.0
