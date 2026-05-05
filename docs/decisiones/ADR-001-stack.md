# ADR-001: Stack inicial Tauri 2 + Rust + Vite + React + TypeScript

- Fecha: 2026-05-05
- Estado: Aceptada
- Decisor: Nicolás Baier

## Contexto

loquazX necesita ser una aplicación de escritorio que coordine procesos pesados (transcripción con whisper, síntesis de voz, mezcla con ffmpeg) y al mismo tiempo ofrezca una interfaz reactiva para editar subtítulos línea por línea, regenerar audio y comparar versiones. Los requisitos clave son:

- Ejecución local sin enviar el contenido del usuario a servicios externos por defecto.
- Acceso al sistema de archivos para leer videos, escribir runs y manejar artefactos grandes.
- Empaquetado en binarios ligeros multiplataforma.
- Capacidad de ejecutar binarios externos (ffmpeg, modelos whisper) y bibliotecas nativas con buen rendimiento.

## Alternativas evaluadas

1. **Electron + Node + React.** Mayor consumo de recursos, dependencia de Chromium, peor desempeño para procesamiento intensivo en el backend.
2. **PyQt o PySide.** Familiaridad con Python (el prototipo previo es Python), pero peor experiencia para construir interfaces complejas y mayor fricción para distribuir binarios.
3. **Tauri 2 con frontend Vite + React + TypeScript y backend Rust.** Binarios pequeños, integración nativa con el sistema, ecosistema Rust maduro para procesamiento de medios y enlaces a `whisper.cpp`.

## Decisión

Se elige Tauri 2 + Rust + Vite + React + TypeScript.

## Consecuencias

- El equipo asume la curva de aprendizaje de Rust en módulos del backend.
- Se aprovechan crates como `whisper-rs` para no depender de un sidecar Python.
- Algunos componentes (XTTS, TTS avanzados) podrían necesitar un sidecar Python o un binario externo en una primera etapa; se documentará en un ADR posterior si ocurre.
- La distribución se hará con `tauri build` para Linux primero, y se evaluarán Windows y macOS más adelante.
