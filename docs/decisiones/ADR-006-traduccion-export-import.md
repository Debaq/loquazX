# ADR-006: Traducción vía exportación/importación de JSON a un LLM externo

- Fecha: 2026-06-11
- Estado: Aceptada
- Decisor: Nicolás Baier

## Contexto

Tras la transcripción (ADR-004), cada segmento tiene tiempos (`start`, `end`) y texto origen (`source`), pero el campo `translation` queda vacío. La etapa de traducción debe rellenarlo.

ADR-001 fija un principio: la app no envía el contenido del usuario a servicios externos por defecto. Una integración directa con una API de traducción en la nube violaría ese principio y ataría el proyecto a un proveedor, sus costos y sus límites. Por otro lado, integrar un motor de traducción local de calidad (modelos NMT en Rust o un sidecar) es un esfuerzo grande que no queremos asumir todavía.

La calidad de la traducción audiovisual depende del contexto: el traductor necesita saber cuánto dura cada línea para que la traducción sea pronunciable en doblaje y legible como subtítulo.

## Alternativas evaluadas

1. **API de traducción en la nube (DeepL, Google, proveedor de LLM).** Buena calidad y cero fricción para el usuario, pero la app haría red con el contenido del usuario, contradiciendo ADR-001, y ataría el proyecto a un proveedor con costos y claves.
2. **Motor de traducción local embebido (modelo NMT en Rust o sidecar Python).** Respeta la privacidad por completo, pero es un esfuerzo de integración y empaquetado considerable (peso del modelo, rendimiento, multiplataforma) que bloquea avanzar ahora.
3. **Exportar una solicitud JSON + prompt e importar la respuesta JSON.** La app no hace red: genera un archivo con los segmentos a traducir y un prompt listo para pegar en el LLM que el usuario elija; luego importa el JSON traducido y rellena `translation` emparejando por `id`. El usuario decide qué herramienta usar (local u online, según su criterio de privacidad).

## Decisión

Se elige la alternativa 3 para la primera versión de la etapa de traducción.

- **Exportación:** un comando escribe en `exports/` dos archivos:
  - `traduccion-solicitud.json` con `schema`, `source_language`, `target_language` y un arreglo de segmentos `{ id, start, end, source }`. Los tiempos se incluyen para que el LLM ajuste el largo de la traducción al doblaje/subtítulo.
  - `traduccion-prompt.md` con instrucciones en español para el LLM: traducir de origen a destino, conservar cada `id`, no fundir ni dividir ni reordenar segmentos, devolver **solo** un JSON con el esquema de respuesta.
- **Importación:** un comando lee un JSON de respuesta `{ schema, target_language, segments: [{ id, translation }] }`, empareja por `id`, rellena `translation` y reescribe `segments.json`. Es tolerante: informa cuántos segmentos se tradujeron, cuántos quedaron sin traducción y cuántos `id` de la respuesta no existen en el proyecto, sin abortar por discrepancias.

El esquema lleva versión en el campo `schema` (`loquazx.translation.request/1`, `loquazx.translation.response/1`) para poder evolucionarlo.

## Consecuencias

- La app sigue sin hacer red en la etapa de traducción: cumple ADR-001 por construcción, porque es el usuario quien lleva el JSON al LLM.
- El usuario elige la herramienta de traducción y asume su criterio de privacidad al usarla.
- La fricción es manual (exportar, pegar, importar); se acepta a cambio de no atar el proyecto a un proveedor ni asumir aún el costo de un motor local.
- **Un motor de traducción local embebido queda como mejora futura**, en un ADR posterior, sin descartarlo: el formato de segmentos no cambia, así que un motor local podría rellenar el mismo campo `translation` más adelante reutilizando la misma estructura.
- El emparejamiento por `id` exige que la respuesta conserve los identificadores; el prompt lo recalca y la importación lo verifica.
