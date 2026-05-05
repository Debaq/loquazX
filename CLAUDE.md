# Instrucciones para Claude Code

Este repositorio es una aplicación Tauri 2 (Rust) + Vite + React + TypeScript para subtitular y doblar video.

## Convenciones

- Toda la documentación, commits, issues y comentarios están en español neutro.
- Commits siguen Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`, `ci:`).
- No reescribir historia de Git en `main` (sin rebase ni squash forzado).
- Antes de un cambio sustantivo, abrir o referenciar un issue.
- Cada PR cierra un issue con `Closes #N`.
- CI verde es prerrequisito para merge.

## Estilo

- Rust: `cargo fmt` + `cargo clippy --all-targets -- -D warnings` antes de commit.
- TypeScript: sin warnings, archivos de componentes en `PascalCase.tsx`.
- Comentarios solo cuando explican el por qué.

## Decisiones arquitectónicas

Cuando una decisión tome más de cinco minutos, documentarla como ADR en `docs/decisiones/`.

## Uso de IA

Mantener `AI_USAGE.md` actualizado el mismo día que se usa IA, no al final.

## Referencia externa

Las prácticas de este proyecto siguen el lab-handbook ubicado en `~/Escritorio/Proyectos/lab-handbook/`.
