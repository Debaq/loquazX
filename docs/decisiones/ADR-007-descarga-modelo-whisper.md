# ADR-007: Descarga y almacenamiento persistente del modelo whisper

- Fecha: 2026-06-11
- Estado: Aceptada
- Decisor: Nicolás Baier

## Contexto

ADR-004 resolvió la gestión del modelo GGML de la forma más simple posible: el usuario selecciona el archivo `.bin` con un diálogo nativo en cada transcripción y la interfaz recuerda la última ruta. En la práctica esto es incómodo: obliga a conseguir el modelo por fuera de la app, saber qué tamaño descargar y reseleccionarlo. El propio ADR-004 anticipó que la descarga automática reemplazaría su gestión del modelo en un ADR posterior.

Queremos que la app descargue un modelo de un nivel a elección y lo guarde de forma persistente para reutilizarlo siempre, sin volver a pedir el archivo.

## Alternativas evaluadas

1. **Seguir con selección manual (statu quo, ADR-004).** Cero código nuevo, pero la fricción que motiva este ADR persiste.
2. **Empaquetar un modelo en el instalador.** Cero fricción de descarga, pero infla el instalador en cientos de MB y fija un único tamaño para todas las máquinas (ya descartada en ADR-004).
3. **Descargar el modelo bajo demanda desde Hugging Face y guardarlo en el directorio de datos de la app.** El usuario elige el nivel; la app descarga con progreso, guarda el archivo en `app_data/models/` y lo reutiliza. Requiere red, barra de progreso y manejo de fallos, pero es la mejor experiencia y ya estaba prevista.

## Decisión

Se elige la alternativa 3 y **reemplaza la parte de gestión del modelo de ADR-004** (la transcripción con `whisper-rs`, el idioma del manifiesto y el reemplazo de `segments.json` siguen vigentes).

- **Niveles ofrecidos:** `tiny`, `base`, `small`, `medium`, `large-v3`, descargados desde
  `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-<nivel>.bin`. El nivel por defecto es `base` (equilibrio calidad/peso).
- **Almacenamiento persistente:** los modelos viven en `app_data/models/ggml-<nivel>.bin` (directorio de datos de la app, resuelto por Tauri). Sobreviven entre proyectos y sesiones.
- **Descarga:** en el backend con `reqwest` (TLS vía `rustls` para no depender de OpenSSL del sistema), corriendo en `spawn_blocking` para no congelar la UI. Se descarga a un archivo temporal `.part` y se renombra al terminar (atómico: una descarga interrumpida no deja un modelo corrupto en su lugar). El progreso se informa a la interfaz con eventos `modelo:progreso` (`descargado`/`total`).
- **Fallback de importación:** el usuario puede importar un `.bin` propio al almacén (copia a `app_data/models/ggml-<nivel>.bin`), para trabajar sin red o reutilizar un modelo que ya tenga.
- **Transcripción:** deja de abrir un diálogo de archivo. La interfaz pasa el **nivel** elegido; el backend resuelve la ruta del modelo guardado y falla con un mensaje claro si ese nivel no está descargado todavía.

## Consecuencias

- La app ahora hace red, pero solo para descargar el modelo a pedido explícito del usuario; no envía contenido del usuario (coherente con ADR-001).
- Nueva dependencia `reqwest` (con `rustls`) en el backend; no requiere bibliotecas del sistema adicionales en CI.
- La verificación de integridad por checksum (SHA) queda como mejora futura: por ahora la descarga atómica y la validación de carga de `whisper-rs` cubren el caso común; un `.bin` corrupto se detecta al transcribir.
- El README deja de pedir descargar el modelo a mano; documenta el gestor de modelos y dónde se guardan.
- La clave `loquazx.whisperModel` de `localStorage` (ruta del archivo) se sustituye por `loquazx.whisperLevel` (nivel elegido).
