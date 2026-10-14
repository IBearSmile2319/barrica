# Contribuir a Barrica

Gracias por ayudar. Unas pautas para que todo encaje.

## Reportar un problema

Abre un [issue](https://github.com/ibearsmile2319/barrica/issues/new) con:
- Lo que hiciste, lo que esperabas y lo que pasó.
- La información de **Acerca de → Copiar información** (o `barrica version`).
- El registro de la pestaña **Registros** del bottle, si algo no abrió.

## Añadir una app al catálogo

Edita `catalog/catalog.toml` siguiendo [catalog/README.md](catalog/README.md), sube `updated` a la fecha del día y comprueba que `cargo test` pasa. Si la probaste en un Mac, dilo en el pull request: con qué capa gráfica y qué tal fue.

## Código

- **Antes de un cambio grande,** abre un issue para hablarlo.
- **Estilo:** el del código que lo rodea. Comentarios en español, que expliquen el porqué y no el qué.
- **Antes del pull request:** `cargo test` y `cargo clippy --all-targets -- -D warnings`, sin avisos.
- **La lógica va en `src/core.rs` o `src/catalog.rs`,** con sus tests; la interfaz solo la muestra.
- **Parches del motor:** en `engine/patches/`, numerados, aplicables con `patch -p1` sobre las fuentes, marcados `BARRICA HACK` y explicados en [docs/MOTOR.md](docs/MOTOR.md).

## Licencia de las contribuciones

Al contribuir aceptas que tu aporte se publique bajo la licencia del proyecto: GPL-3.0-or-later, salvo los parches de `engine/patches/`, que van bajo LGPL-2.1-or-later como el resto de Wine.

## Lo que no aceptamos

- Código, binarios o ingeniería inversa de CrossOver o de otro software cerrado.
- Redistribuir D3DMetal ni otras piezas cuya licencia no lo permita.
