# ADR-009: Motor de síntesis de voz (TTS) para el doblaje

- Fecha: 2026-06-12
- Estado: Aceptada
- Decisor: Nicolás Baier

## Contexto

Con la traducción resuelta (ADR-006 export/import; ADR-008 motor local futuro), el
campo `translation` de cada segmento ya puede llenarse. La siguiente etapa del
pipeline es el **doblaje**: convertir ese texto traducido en audio, por segmento,
para cada idioma de salida.

La etapa tiene dos diferencias frente a la traducción:

1. **Tiene ajustes** (voz, velocidad, tono, motor) que la traducción no necesita.
2. **Se hace por trozos**: cada segmento es independiente, trae su propio
   presupuesto de tiempo (`end - start`) y puede sintetizarse, guardarse en
   `runs/` (ADR-002) y regenerarse solo, sin tocar el resto. Esto la hace más
   simple que sintetizar una pista completa de una vez.

El prototipo previo en Python (`subdub`, ver AI_USAGE bootstrap) usaba **edge-tts**.
La interfaz ya tiene andamiaje para esta etapa: el selector de **Voz** en
`EditPanel` (con `edge-tts` y `XTTS-v2`), las pistas «Doblaje · idioma» en la
`Timeline` y la previsualización por `url_media` que prevé ADR-005.

La restricción rectora es ADR-001: ejecución local **sin enviar el contenido del
usuario a servicios externos por defecto**. El «por defecto» es importante: admite
una vía online siempre que sea opt-in explícito y no la opción por defecto.

## Alternativas evaluadas

1. **Solo edge-tts (online).** Es el TTS de «Leer en voz alta» de Edge por
   ingeniería inversa: WebSocket a `speech.platform.bing.com/.../edge/v1`, token
   `Sec-MS-GEC` y payload SSML, devolviendo audio. Se implementa en **Rust puro**
   (crate `msedge-tts` o portando el protocolo con `tokio-tungstenite`/`reqwest`),
   sin sidecar Python. Fácil, gratis y con muchas voces, pero **manda el texto
   traducido a servidores de Microsoft**: como única vía sería el default y
   violaría el «por defecto» de ADR-001. Requiere red en cada generación.
2. **Solo Piper (local).** TTS local sobre ONNX; voces de ~50-100 MB por idioma
   descargadas desde Hugging Face. Reutiliza la plomería de descarga/almacén de
   ADR-007 (igual que whisper y el motor de traducción de ADR-008). Cumple ADR-001
   por construcción, corre en CPU y es liviano. Contra: hay que descargar una voz
   por idioma y la calidad/prosodia es buena pero no clona voces.
3. **XTTS-v2 (local, clonación de voz).** Mejor calidad y voz clonable a partir de
   una muestra, pero pesado (~2 GB) y exige sidecar Python (ADR-001 ya lo
   anticipa), lo que rompe el «todo en Rust» y complica instalador y CI.
4. **Dos motores tras una firma común.** `Piper` como **default local** y
   `edge-tts` como **opt-in online**, ambos en Rust detrás de una misma firma
   `synth(segmento, ajustes) -> wav`. El usuario elige según necesite privacidad
   (Piper, sin red) o comodidad/variedad de voces (edge-tts, con red). XTTS-v2
   queda visible en la UI pero **deshabilitado**, como feature futuro.

## Decisión

Se elige la **alternativa 4**: dos motores tras una firma común, con Piper local
por defecto y edge-tts online como opt-in, y XTTS-v2 reservado a futuro.

- **Default local (ADR-001 por construcción):** Piper es la opción por defecto. Sin
  red, el contenido del usuario no sale de su máquina.
- **Opt-in online:** edge-tts queda disponible como elección explícita. No es el
  default y la UI advierte que sube el texto traducido a Microsoft. Esto respeta el
  «por defecto» de ADR-001 sin negarle al usuario una vía cómoda. Se implementa en
  Rust puro, sin sidecar Python.
- **XTTS-v2 a futuro:** aparece en el selector de voz **deshabilitado**, etiquetado
  como próximamente (clonación de voz). No se implementa en esta versión por su peso
  y la dependencia de un sidecar Python; entrará detrás de la misma firma `synth`.
- **Síntesis por segmento:** cada segmento se sintetiza por separado a un WAV
  guardado en `runs/` (ADR-002), emparejando por `id` como el resto del pipeline.
  La regeneración es por segmento, no global.
- **Ajuste de duración:** el audio sintetizado rara vez calza el hueco
  `end - start`. Se ajusta con `atempo` de ffmpeg (reusa la invocación de ADR-003)
  para estirarlo/comprimirlo al tiempo disponible, conservando el tono.
- **Voz por idioma de salida:** los ajustes (motor, voz, velocidad, tono) se eligen
  por idioma de salida, alineado con el soporte multi-idioma de la `Timeline`.
- **Previsualización:** el WAV generado se reproduce por `url_media` (ADR-005).
- **Descarga de voces Piper:** reutiliza el gestor de modelos de ADR-007 extendido
  para conocer las voces de TTS, con un prefijo propio en `app_data/models/` para no
  chocar con whisper (`ggml-*.bin`) ni con traducción (`mt-*.gguf`).
- **Esqueleto primero:** el módulo (`tts.rs`/`synth`) entra con firmas y un *stub*
  que devuelve «no implementado», igual que ADR-008, para mantener CI verde antes de
  arrastrar el protocolo de edge-tts y los binarios ONNX de Piper. La integración
  real entra en PRs siguientes detrás de las mismas firmas.

## Consecuencias

- El usuario que use Piper dobla sin red, cumpliendo ADR-001 por construcción; quien
  elija edge-tts acepta enviar el texto a Microsoft de forma explícita.
- Nuevas dependencias de backend: un runtime ONNX (`ort`) para Piper y un cliente
  WebSocket para edge-tts (`msedge-tts` o equivalente), ambos en Rust. El runtime
  ONNX se comparte con el motor de traducción NMT (ADR-008 revisado 2026-06-12, que
  pasa a NLLB sobre ONNX): una sola dependencia nativa sirve TTS y traducción.
- edge-tts depende de un endpoint no oficial de Microsoft (token `Sec-MS-GEC`) que
  puede cambiar sin aviso; se acepta por ser una vía secundaria, no el default.
- La primera generación con Piper exige descargar la voz del idioma (igual que
  whisper); edge-tts no descarga nada pero exige red.
- XTTS-v2 queda pendiente; si se confirma que su calidad/clonación justifica el
  sidecar Python, entra después detrás de la firma `synth` sin cambiar el formato.
- El ajuste de duración por `atempo` puede degradar la naturalidad si la traducción
  excede mucho el tiempo disponible; ADR-006 ya mitiga esto pidiendo al traductor
  ajustar el largo al doblaje.
