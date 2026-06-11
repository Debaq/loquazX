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
- Transcripción del audio a segmentos con `whisper-rs` según ADR-004: el usuario selecciona el modelo GGML (la última ruta se recuerda), el idioma sale del manifiesto y el resultado reemplaza `segments.json` previa confirmación. Botón «Transcribir» en la barra superior.

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
