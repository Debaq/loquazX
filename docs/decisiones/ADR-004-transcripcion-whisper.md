# ADR-004: Transcripción con whisper-rs y modelo provisto por el usuario

- Fecha: 2026-06-11
- Estado: Aceptada
- Decisor: Nicolás Baier

## Contexto

Con el audio en `media/audio.wav` (ADR-003), la transcripción se hace con `whisper-rs`
(enlaces Rust a whisper.cpp), como anticipa ADR-001. whisper.cpp necesita un modelo
GGML (`ggml-*.bin`, de ~75 MB a ~3 GB según tamaño) que no puede ir embebido en el
código. Hay que decidir cómo llega ese archivo a la aplicación.

## Alternativas evaluadas

1. **Empaquetar un modelo en el instalador.** Cero fricción inicial, pero infla el
   instalador en cientos de MB y fija un único tamaño de modelo para todos los
   usuarios y máquinas.
2. **Descarga automática desde Hugging Face al primer uso.** Buena experiencia, pero
   exige red, barra de progreso, verificación de integridad y manejo de fallos de
   descarga: superficie de código considerable para una etapa temprana.
3. **El usuario provee el archivo GGML y lo selecciona con un diálogo.** Mínimo
   código, coherente con ADR-003 (apoyarse en recursos ya presentes en el sistema) y
   flexible: cada usuario elige el tamaño de modelo que su máquina soporta.

## Decisión

El usuario **selecciona el archivo de modelo GGML** mediante un diálogo nativo la
primera vez que transcribe; la interfaz recuerda la última ruta usada (almacenamiento
local del frontend) y la propone como valor por defecto en usos posteriores. El README
documenta de dónde descargar los modelos oficiales de whisper.cpp.

La transcripción corre en el backend con `whisper-rs`, usa el idioma de origen del
manifiesto y **reemplaza** `segments.json` (la interfaz pide confirmación si ya hay
segmentos). Cada segmento conserva inicio y fin en segundos y el texto reconocido;
la traducción queda vacía para la etapa siguiente.

## Consecuencias

- Requisito documentado en el README: descargar un modelo GGML de
  <https://huggingface.co/ggerganov/whisper.cpp>.
- La descarga automática con progreso queda como mejora futura; si se implementa,
  ese ADR reemplazará la parte de gestión del modelo de este.
- El registro de cada transcripción como run en `runs/` (ADR-002) se pospone hasta
  que exista más de una etapa generativa que comparar.
- Compilar `whisper-rs` exige `cmake` y un compilador C/C++ en el entorno de build
  (presentes en CI y documentados para desarrollo local).
