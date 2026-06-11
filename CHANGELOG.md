# Changelog

Todos los cambios notables de este proyecto se documentan en este archivo.

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el versionado sigue [Semantic Versioning](https://semver.org/lang/es/).

## [Unreleased]

### Added

- Formato de proyecto en carpeta `.lqzx` según ADR-002: `project.json` con `format_version`, `segments.json` y subdirectorios `source/`, `media/`, `runs/`, `exports/`.
- Comandos Tauri `crear_proyecto`, `abrir_proyecto` y `guardar_segmentos` con tests unitarios.
- Botones Nuevo, Abrir y Guardar de la barra superior conectados a diálogos nativos.
- Importación de video al proyecto (copia a `source/` o referencia a la ruta original, a elección del usuario) y reproducción en el panel de previsualización.

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
