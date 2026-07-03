# ADR-010: Render de presentación con PDF + segmentos con slide

- Fecha: 2026-07-02
- Estado: Propuesta
- Decisor: Nicolás Baier
- Cierra: issue #18

## Contexto

Hoy un proyecto `.lqzx` asume video fuente (`source/` + `media/audio.wav`) y produce un mp4 final al exportar (futuro). Hay un caso de uso claro y frecuente: el usuario ya tiene la **presentación** en un PDF y los textos que la acompañan, y quiere **narrarla en otro idioma sin grabar video**. La transcripción con whisper no aplica cuando no hay audio del usuario (y aunque lo hubiera, el material de fondo es el PDF, no un video).

El modelo de proyecto actual ya adopta el patrón "fuente opcional" (el campo `source` en el manifiesto es opcional para no romper proyectos previos, ADR-002). Se propone extenderlo:

- El campo `slides` pasa a ser una segunda fuente opcional e independiente de `source`.
- Cada `Segment` puede llevar el número de página del PDF (`slide: 1..N`) que debe mostrarse durante su intervalo `[start, end)`.
- Un nuevo comando `renderizar_presentacion` produce el mp4 final sincronizando páginas del PDF con los huecos y mezclando los WAV de doblaje de `runs/dub/`.

Si en un mismo proyecto conviven `source` y `slides`, hoy manda `source` (el flujo video se mantiene); el render de presentación queda como ruta explícita desde la barra superior ("Exportar video" se habilita cuando hay `slides`).

## Alternativas evaluadas

1. **Formato de proyecto aparte (`*.lqzx-presentation` o carpeta distinta).** Aísla bien los dos modos pero duplica el shell (creación, apertura, guardado, traductor, doblaje). Multiplica la superficie a mantener para un feature que es esencialmente el mismo pipeline con otra fuente visual.
2. **Slides como assets sueltos (imágenes una por segmento, sin PDF).** Más simple de implementar pero le exige al usuario cortar el PDF a mano o generar imágenes. Pierde la noción de "página" y rompe el caso de uso real (el usuario ya tiene un PDF).
3. **Misma plantilla, fuente opcional `slides` en el manifiesto, comando `renderizar_presentacion` con `ffmpeg` y `pdftoppm`.** Reusa todo el pipeline existente (segmentos, traducción, doblaje), sólo cambia la fuente visual y la etapa de exportación. Consistente con la decisión de ADR-002 de hacer `source` opcional.

## Decisión

Se adopta la **alternativa 3**: misma plantilla `.lqzx`, fuente opcional `slides`, render en backend con `ffmpeg` (ADR-003) y `pdftoppm` de Poppler para rasterizar páginas.

### Cambios al formato

`project.json` gana un campo opcional:

```json
"slides": {
  "file": "slides/original.pdf",
  "page_count": 12,
  "imported_at": 1720123456
}
```

Ausente en proyectos previos: el campo es `#[serde(default, skip_serializing_if = "Option::is_none")]` igual que `source`. Se persiste la copia bajo `slides/` (por paralelismo con `source/`).

`Segment` gana un campo opcional `slide: number | null`:

```json
{ "id": "...", "start": 0.0, "end": 4.2, "source": "...", "translation": "...", "slide": 2 }
```

`null` significa "mostrar la última página vista" (al inicio del video se asume la 1). Numeración 1‑based, validada contra `page_count` en la UI y al renderizar.

### Backend

Nuevo módulo `presentacion.rs`:

- `rasterizar_pdf(pdf, out_dir) -> Result<u32>` invoca `pdftoppm -r 150 -png <pdf> <out_dir>/page`; devuelve `page_count`. Si `pdftoppm` no está en `PATH`, error explícito (mismo principio que ADR-003 para `ffmpeg`).
- `calcular_pista_audio(segments, dub_dir, total_dur) -> PathBuf` concatena los `runs/dub/<id>.wav` con `ffmpeg` y el demuxer `concat`, insertando silencio (`anullsrc`) en los huecos entre segmentos para que la duración total coincida con la del último `end` (o el mayor entre `end` y `page_count * dur_por_pagina`, lo que sea mayor).
- `calcular_timeline_imagenes(segments, page_count, total_dur)` arma el `concat.txt` de ffmpeg con `-loop 1 -t <dur> -i page_N.png` por cada bloque entre cambios de `slide`. Si ningún segmento tiene `slide`, todo el video usa la página 1.
- `render(...)`: pipeline `ffmpeg -f concat -safe 0 -i imgs.txt -i audio.wav -c:v libx264 -pix_fmt yuv420p -shortest -movflags +faststart out.mp4`. `-movflags +faststart` para que el mp4 abra rápido al servirse por el `MediaServer` (ADR-005).

Comandos Tauri nuevos:

| Comando | Notas |
|---|---|
| `importar_pdf(path, pdf)` | Copia al proyecto, persiste `page_count`. |
| `conteo_paginas_pdf(pdf)` | Devuelve el `page_count` sin copiar; útil para validación previa. |
| `importar_audio_presentacion(path, audio)` | Importa audio arbitrario cuando no hay video. |
| `importar_segmentos_json(path, json)` | Importa segmentos externos `{start, end, slide, source}` con `id = uuid` y `translation = ""`. |
| `renderizar_presentacion(path)` | Produce el mp4; emite `presentacion:progreso`. |

El mp4 final va a `exports/<nombre>.mp4` (paralelo a `traduccion-solicitud.json`).

### Frontend

- `TopBar`: cuatro botones nuevos: "Importar PDF" (`FileText`), "Importar audio" (`Music`), "Importar segmentos JSON" (`FilePlus2`), "Exportar video" (`Clapperboard`). El último se habilita cuando hay `slides` y al menos un segmento doblado.
- `VideoPreview`: cuando no hay video pero sí `slides`, muestra la página activa del PDF según el `currentTime` del reproductor virtual. Las páginas se pre-rasterizan al importar el PDF y se sirven por `url_media` (ADR-005), igual que el video.
- `EditPanel`: campo numérico "Diapositiva" (1‑based, con `max={page_count}`) por segmento. Persiste al cambiar de segmento y al guardar.
- `SegmentsList`: muestra `p.N` al lado del tiempo.
- `Timeline`: pista "Slides" opcional que cambia de color cuando cambia la página (sólo visual; no editable en este PR).

## Consecuencias

- La app gana una dependencia externa asumida: `pdftoppm` (poppler). Está en los repos oficiales de todas las distros Linux (`poppler-utils`); para Windows y macOS se documentará en el README junto con ffmpeg.
- `Segment` cambia de esquema; los proyectos previos no se rompen por el `#[serde(default)]`.
- El render de presentación convive con el flujo de video, pero en esta versión no se mezclan: si el proyecto tiene `source`, se usa la ruta de exportación existente (futuro); si tiene `slides` solamente, se ofrece el render de presentación. La coexistencia mixta se documenta como limitación.
- Los WAV de doblaje se reusan tal cual: el modo presentación no requiere regenerar audio.
- Los tests que rasterizan PDF o invocan ffmpeg se saltean si la dependencia no está, mismo patrón que `extract_audio` (ADR-003).

## Fuera de alcance

- Subtítulos quemados en el mp4 (issue aparte; requiere `libass` en ffmpeg).
- Generación de PDF a partir de las imágenes del proyecto (dirección opuesta, sin demanda).
- Multi‑idioma de salida en la exportación (se sigue doblando solo al `target_language`).
- Edición visual del `slide` arrastrando bloques en la `Timeline` (queda como mejora futura; en este PR se edita por segmento desde el `EditPanel`).
