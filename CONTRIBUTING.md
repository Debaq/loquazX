# Cómo contribuir a loquazX

Gracias por el interés. Este documento describe cómo proponer cambios al proyecto.

## Antes de programar

1. Abre o busca un issue que describa el problema o la mejora.
2. Comenta el issue indicando que vas a trabajar en él.
3. Si el cambio es no trivial, espera a que se discuta el alcance antes de avanzar.

Programar sin issue genera trabajo que puede no entrar al proyecto. Es preferible perder cinco minutos discutiendo el alcance que tirar un día de código.

## Flujo de trabajo

1. Haz fork del repositorio o crea una rama si tienes acceso de escritura.
2. Crea una rama por cambio: `feat/<resumen>` o `fix/<resumen>`.
3. Haz commits incrementales con mensajes en formato [Conventional Commits](https://www.conventionalcommits.org/es/).
4. Asegúrate de que la integración continua queda en verde antes de pedir revisión.
5. Abre un pull request enlazando el issue: `Closes #<n>`.

## Estilo de commits

Tipos comunes: `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `ci`.

Mensaje:

```
tipo(área opcional): descripción en imperativo, sin punto final

Cuerpo opcional con detalles del por qué.
```

Ejemplos:

- `feat(transcribe): integra whisper-rs como backend por defecto`
- `fix(tts): respeta el tamaño máximo de bloque al sintetizar`
- `docs: agrega ADR sobre el formato de proyecto`

## Estilo de código

- TypeScript: sigue la configuración de `tsconfig.json`. Sin warnings.
- Rust: `cargo fmt` y `cargo clippy --all-targets -- -D warnings`.
- Componentes React: nombre de archivo `PascalCase.tsx`.
- Comentarios solo cuando explican el por qué, no el qué.

## Tests

Cuando agregues una función nueva, agrega los tests asociados. La suite tiene que correr en local antes de abrir el pull request.

```bash
npm test            # frontend
cargo test          # backend Rust
```

## Documentación

Cualquier cambio que altere el comportamiento visible para el usuario debe actualizar:

- `README.md` si afecta instalación o uso básico.
- `CHANGELOG.md` (sección `Unreleased`).
- `docs/` si introduce un nuevo concepto.
- `docs/decisiones/` si la decisión arquitectónica te tomó más de cinco minutos.

## Uso de IA

Si usaste IA generativa de manera sustantiva, agrega una entrada a `AI_USAGE.md` el mismo día. Ver la política completa en [`docs/04-politica-ia.md`](docs/04-politica-ia.md) (cuando exista) o en el lab-handbook de referencia.

## Idioma

Todos los textos del proyecto (commits, issues, comentarios, documentación) están en español neutro, sin modismos regionales.

## Código de conducta

La participación está regida por [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).
