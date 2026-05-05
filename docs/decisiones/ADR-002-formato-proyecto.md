# ADR-002: Formato de proyecto loquazX

- Fecha: 2026-05-05
- Estado: Propuesta
- Decisor: Nicolás Baier

## Contexto

Un proyecto de loquazX implica artefactos heterogéneos y voluminosos: video original, pistas de audio extraídas, transcripciones, traducciones, múltiples intentos de síntesis de voz por línea, y mezclas finales. El usuario debe poder:

- Abrir y cerrar el proyecto sin perder estado.
- Conservar todas las versiones intermedias para comparar y revertir.
- Editar línea por línea sin reescribir archivos pesados.
- Compartir el proyecto (con o sin medios) en otra máquina.
- Auditar qué modelo, prompt y parámetros generaron cada artefacto.

Un único archivo binario (estilo SQLite o ZIP) simplifica la portabilidad pero penaliza la edición incremental y dificulta diff/merge en Git. Una carpeta con archivos planos facilita la inspección, el versionado externo y la regeneración parcial.

## Alternativas evaluadas

1. **Archivo único `.lqzx` (ZIP con manifiesto JSON).** Portátil y atómico, pero costoso de actualizar con cada regeneración de línea y opaco para herramientas externas.
2. **Base SQLite + carpeta de medios.** Buen rendimiento de consulta, pero introduce dependencia de esquema y migraciones tempranas para un formato que aún evoluciona.
3. **Carpeta de proyecto con manifiesto JSON y subdirectorios por etapa.** Inspeccionable, fácil de versionar, permite regenerar artefactos individuales sin reescribir el resto.

## Decisión

Se adopta el formato **carpeta de proyecto** con extensión `.lqzx` aplicada al directorio raíz. La estructura mínima es:

```
proyecto.lqzx/
  project.json          # manifiesto: id, versión de formato, metadatos, idioma origen y destino
  source/               # video original (referencia o copia según preferencia del usuario)
  media/
    audio.wav           # pista extraída del video
  segments.json         # lista ordenada de segmentos con timing y textos
  runs/                 # cada intento de transcripción, traducción o síntesis
    <run-id>/
      run.json          # tipo, modelo, parámetros, hash de entrada, timestamp
      output/           # artefactos producidos (texto, audio por segmento)
  exports/              # mezclas finales y subtítulos exportados
```

`segments.json` referencia los `run-id` activos por etapa para cada segmento, lo que permite cambiar la versión vigente sin mover archivos.

`project.json` incluye `format_version` (entero) para habilitar migraciones futuras.

## Consecuencias

- La aplicación abre un directorio, no un archivo; el diálogo nativo debe filtrar por carpetas con `project.json` válido.
- El usuario puede inspeccionar y respaldar el proyecto con herramientas estándar.
- El tamaño en disco crece con cada regeneración; se documentará un comando para purgar runs no referenciados.
- El formato exacto de `segments.json` y `run.json` se especificará en un ADR posterior cuando estabilicen los campos.
- La portabilidad entre máquinas requiere comprimir la carpeta manualmente o vía un comando `Exportar proyecto` que se definirá luego.
