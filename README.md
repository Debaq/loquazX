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

Requisitos en tiempo de ejecución:

- `ffmpeg` instalado y disponible en el `PATH` (ADR-003). En Linux está en los
  repositorios oficiales de todas las distribuciones.
- `pdftoppm` y `pdfinfo` (poppler) para el modo presentación (ADR-010). En
  Linux vienen en el paquete `poppler-utils`.
- Un modelo de whisper para transcribir. La app lo descarga desde el gestor
  «Modelo» (ADR-007) y lo guarda de forma persistente; no hace falta conseguirlo
  a mano. Conexión a internet solo para esa descarga; `base` es buen punto de
  partida. También puede importarse un `ggml-*.bin` propio para trabajar offline.

Requisitos de compilación: `cmake` y un compilador C/C++ con `libclang`
(necesarios para compilar `whisper-rs`).

```bash
git clone https://github.com/Debaq/loquazX.git
cd loquazX
npm install
npm run tauri dev
```

## Uso

La documentación detallada se irá agregando en `docs/` a medida que las funciones queden estables. El flujo actual es: crear/abrir proyecto → importar video → extraer audio → transcribir → traducir.

### Modelo de transcripción (ADR-007)

El botón «Modelo» abre el gestor de modelos whisper. Cada nivel (`tiny`, `base`, `small`, `medium`, `large-v3`) puede descargarse desde Hugging Face con barra de progreso; el archivo queda guardado en el directorio de datos de la app y se reutiliza siempre. También puedes importar un `ggml-*.bin` propio o borrar modelos. La transcripción usa el nivel marcado «en uso», sin pedir un archivo cada vez.

### Modo presentación (ADR-010)

Un proyecto puede traer, además del video fuente, un **PDF de fondo** y segmentos con `slide: number` (1‑based). El botón «Importar PDF» copia el PDF bajo `slides/` y pre‑cuenta las páginas; el botón «Importar audio» acepta un audio arbitrario cuando no hay video; «Importar segmentos JSON» levanta un JSON con `[{start, end, slide, source}]` y rellena los segmentos (sobrescribe tras confirmar). Tras traducir y doblar los segmentos como siempre, «Exportar video» produce el mp4 final (`exports/<nombre>.mp4`) sincronizando páginas del PDF con los huecos `[start, end)` y mezclando los WAV de `runs/dub/`.

**Recalibración de timings**: cuando los `start`/`end` originales son aproximados, comprimir el audio con `atempo` para encajar degrada la naturalidad de la voz. «Recalibrar» (icono de sliders) deja que el audio dictate la duración: sintetiza cada segmento a velocidad natural, reasigna los `start`/`end` cumulativamente con un gap de 0.2s, y guarda un backup (`segments.original.json`) que «Restaurar» (icono de flecha) revierte.

### Traducción (ADR-006)

loquazX no traduce por sí mismo: exporta el trabajo para que lo haga el LLM que prefieras y luego importa el resultado, sin hacer red.

1. **Exportar para traducir** escribe en `exports/` dos archivos: `traduccion-solicitud.json` (los segmentos con sus tiempos y texto origen) y `traduccion-prompt.md` (las instrucciones para el LLM).
2. Pega el prompt y el JSON de solicitud en el LLM de tu elección y obtén un JSON de respuesta con la traducción de cada segmento.
3. **Importar traducción** lee ese JSON de respuesta y rellena la traducción de cada segmento emparejando por `id`. La app informa cuántos se tradujeron, cuántos quedaron sin traducción y cuántos `id` no correspondían al proyecto.

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
