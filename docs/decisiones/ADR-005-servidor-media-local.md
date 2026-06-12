# ADR-005: Servidor HTTP local para reproducir media en el webview

- Fecha: 2026-06-11
- Estado: Aceptada
- Decisor: Nicolás Baier

## Contexto

En Linux el video importado no se reproduce (issue #10). El protocolo `asset:` de
Tauri funciona para `fetch` (responde 206 con rangos), pero el reproductor de media
de WebKitGTK entrega la URI directamente a GStreamer, que no pasa por el handler de
schemes custom y falla con `FormatError`. Es una limitación conocida de WebKitGTK;
en Windows/macOS el protocolo asset sí sirve media.

## Alternativas evaluadas

1. **Cargar el video como blob en el frontend** (`fetch` → `URL.createObjectURL`).
   Mínimo código, pero carga el archivo completo en RAM: inviable para los videos
   largos que el doblaje contempla.
2. **URI `file://` directa en el `<video>`.** WebKit bloquea recursos locales desde
   orígenes no locales; no hay ajuste expuesto por wry que lo permita de forma
   segura.
3. **Servidor HTTP local mínimo con soporte de `Range`.** GStreamer reproduce
   `http://127.0.0.1` de forma nativa, el rango habilita seek sin cargar el archivo
   en memoria, y el mismo mecanismo servirá audio (previsualización de TTS) más
   adelante. Costo: ~150 líneas de Rust con la biblioteca estándar.

## Decisión

Se implementa un **servidor HTTP local** en el backend (`media_server.rs`):

- Escucha solo en `127.0.0.1` con puerto efímero asignado por el sistema.
- Cada URL incluye un **token aleatorio** por sesión y el servidor solo sirve
  archivos previamente registrados en una **allowlist** (rutas canónicas exactas),
  de modo que otras páginas o procesos locales no puedan leer archivos arbitrarios.
- Soporta `GET` con `Range: bytes=a-b`/`bytes=a-` (206) y sin rango (200), con
  `Accept-Ranges: bytes`.
- El comando Tauri `url_media` registra el archivo y devuelve su URL; el frontend
  la usa como `src` del `<video>` en lugar de `convertFileSrc`.

Se usa en todas las plataformas para tener un único camino de código.

## Consecuencias

- El protocolo `asset:` deja de usarse para media; puede retirarse del scope cuando
  no quede ningún uso.
- El servidor vive lo que vive la aplicación; no persiste nada.
- La síntesis de voz (TTS) reutilizará `url_media` para previsualizar audio por
  segmento.
- Si wry/WebKitGTK llegan a soportar media sobre schemes custom, este ADR puede
  revisarse para volver al protocolo asset.
