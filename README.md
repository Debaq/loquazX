# loquazX

> Aplicación de escritorio para subtitular y doblar video con IA, manteniendo control humano sobre cada línea de la traducción y de la voz generada.

[![CI](https://github.com/Debaq/loquazX/actions/workflows/ci.yml/badge.svg)](https://github.com/Debaq/loquazX/actions/workflows/ci.yml)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)

## Qué problema resuelve

Generar subtítulos y doblaje de video automáticamente es viable hoy, pero los pipelines existentes son cajas negras: no permiten editar la traducción línea por línea, ni regenerar el audio de un único bloque, ni preservar las versiones intermedias para comparar. loquazX expone cada paso del proceso (transcripción, traducción, síntesis, mezcla) como una etapa controlable, con persistencia de cada intento y previsualización por línea.

## Quién debería usarlo

Investigadores, divulgadores y equipos académicos que necesitan publicar video en varios idiomas y requieren auditar y modificar la traducción y la voz antes de exportar el resultado final.

## Estado

Alpha. En desarrollo activo. La interfaz, los formatos de proyecto y los comandos pueden cambiar entre releases hasta `v1.0.0`.

## Stack

- Tauri 2 (Rust) — backend y empaquetado
- Vite + React + TypeScript — frontend
- whisper.cpp (vía `whisper-rs`) — transcripción
- edge-tts y XTTS-v2 — síntesis de voz
- ffmpeg — extracción y mezcla de audio

Decisiones detalladas en [`docs/decisiones/`](docs/decisiones/).

## Instalación

Pendiente. Requiere compilación desde el código fuente hasta la primera release con binarios.

```bash
git clone https://github.com/Debaq/loquazX.git
cd loquazX
npm install
npm run tauri dev
```

## Uso

Pendiente. La documentación se irá agregando en `docs/` a medida que las funciones queden estables.

## Contribuir

Lee [`CONTRIBUTING.md`](CONTRIBUTING.md). Antes de programar, abre o comenta el issue correspondiente.

## Uso de IA en el desarrollo

Este proyecto utiliza herramientas de IA generativa de manera declarada y revisada. Ver [`AI_USAGE.md`](AI_USAGE.md).

## Código de conducta

La participación está regida por el [Código de Conducta](CODE_OF_CONDUCT.md).

## Licencia

AGPL-3.0. Ver [`LICENSE`](LICENSE).

## Citación

Pendiente hasta primera publicación. Cuando exista DOI en Zenodo se actualizará esta sección.
