# ADR-003: Extracción de audio con ffmpeg del sistema

- Fecha: 2026-06-11
- Estado: Aceptada
- Decisor: Nicolás Baier

## Contexto

Para transcribir con whisper.cpp (vía `whisper-rs`, según ADR-001) se necesita el audio
del video en WAV PCM 16 bits, mono, 16 kHz: es el único formato de entrada que la
biblioteca acepta sin conversión adicional. Hay que decidir cómo se produce ese archivo.

## Alternativas evaluadas

1. **Decodificar en Rust (symphonia + resampler).** Sin dependencia externa, pero
   symphonia no cubre todos los contenedores/códecs que un usuario puede importar
   (mkv con AC3, mov con PCM raro, etc.) y obliga a mantener una cadena de resampleo
   propia.
2. **ffmpeg empaquetado como sidecar de Tauri.** Experiencia sin fricción para el
   usuario, pero suma ~80 MB por plataforma al instalador y obliga a gestionar
   licencias y actualizaciones del binario desde ya.
3. **ffmpeg del sistema, invocado como proceso.** Cubre cualquier formato, cero peso
   en el instalador, y es lo que ADR-001 ya anticipaba ("capacidad de ejecutar
   binarios externos: ffmpeg"). El costo es exigir que ffmpeg esté instalado.

## Decisión

Se usa **ffmpeg del sistema** invocado como proceso externo. Si no está disponible,
la aplicación muestra un error claro indicando cómo instalarlo. La salida se escribe
en `media/audio.wav` dentro del proyecto y queda registrada en el manifiesto.

Parámetros fijos de extracción: `-vn -ac 1 -ar 16000 -c:a pcm_s16le`.

## Consecuencias

- Requisito de instalación documentado en el README; en Linux (objetivo inicial)
  ffmpeg está en todos los repositorios oficiales.
- Cuando el proyecto apunte a Windows/macOS se reevaluará empaquetarlo como sidecar;
  esa decisión reemplazaría este ADR.
- El mismo mecanismo de invocación servirá después para la mezcla final (doblaje),
  que también usa ffmpeg.
