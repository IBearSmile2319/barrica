# Barrica

Gestor de *bottles* de Wine para macOS, escrito en Rust. Es un proyecto nuevo, inspirado en CrossOver, que no usa código de CrossOver ni de GitHub.

Un **bottle** es un Windows aparte (un `WINEPREFIX`) donde instalas programas y juegos. Barrica los crea, los organiza y los abre, y añade cosas que CrossOver no trae: snapshots instantáneos, un log por ejecución y una biblioteca con el arte de tus juegos de Steam.

## Qué hace hoy

- **Bottles:** crear (Windows 98 a 11, 32 o 64 bits), renombrar, duplicar, exportar e importar, y mover a la Papelera (nunca borra).
- **Biblioteca:** juegos de Steam con su portada y banner reales, y launchers directos (`steam.exe -applaunch <id>`). Aplicaciones con el icono que CrossOver ya extrajo, o con su inicial si no hay icono.
- **Ejecutar:** `.exe`, `.msi`, `.lnk` y `.bat`. Cada ejecución guarda su salida en *Registros*.
- **Indicador «En ejecución»:** se basa en el socket real del `wineserver` de cada bottle.
- **Snapshots:** son clones APFS, instantáneos y sin coste de disco hasta que el bottle cambia. Restaurar manda el estado actual a la Papelera.
- **Herramientas de Windows:** `winecfg`, `regedit`, administrador de tareas, panel de control, explorador y desinstalador.
- **Ajustes:** versión de Windows, gráficos (Auto, D3DMetal, DXVK o WineD3D), MSync, modo Retina y variables de entorno.
- **CrossOver:**
  - **Vincular** un bottle: Barrica lo abre con el `wine` oficial de CrossOver (su motor, su D3DMetal y su licencia) y le suma su interfaz, snapshots y logs. Así corre CS2.
  - **Copiar** un bottle: clon APFS independiente para usarlo con un motor propio. El original nunca se toca.
- **Terminal** con el entorno del bottle ya preparado (`wine`, `winecfg` y `regedit` apuntan a él).
- **CLI completa:** `barrica help`.

## Instalar y abrir

Requisitos: macOS en Apple Silicon con Rosetta, y Rust 1.88 o superior.

```sh
./bundle.sh                 # compila en release y crea target/Barrica.app
open target/Barrica.app
```

Para bottles propios (no vinculados) hace falta un motor Wine de código abierto. Homebrew desactivó las casks de WineHQ el 2026-09-01, así que hay que descargarlo de <https://wiki.winehq.org/MacOS> y elegirlo en **Motor → Elegir ejecutable de wine…**. Barrica también detecta Game Porting Toolkit en `/usr/local/opt/game-porting-toolkit`.

## CLI

```sh
barrica list
barrica link-crossover "Counter-Strike 2"     # usar un bottle de CrossOver con su motor
barrica programs "Counter-Strike 2"
barrica run "Counter-Strike 2" ".../Steam/steam.exe" -applaunch 730
barrica snapshot "Counter-Strike 2" "antes de los mods"
barrica help                                  # todos los comandos
```

## Dónde guarda las cosas

| Qué | Dónde |
|---|---|
| Bottles | `~/Library/Application Support/Barrica/Bottles/<nombre>/` (WINEPREFIX + `barrica.toml`) |
| Bottles vinculados | Solo `barrica.toml` con `crossover = "<bottle>"`; los datos siguen en CrossOver |
| Snapshots | `~/Library/Application Support/Barrica/Snapshots/<bottle>/<etiqueta>/` |
| Registros | `~/Library/Application Support/Barrica/Logs/<bottle>/` |
| Ajustes | `~/Library/Application Support/Barrica/settings.toml` |

`BARRICA_HOME` cambia la carpeta raíz, por ejemplo para usar otro disco o para los tests.

## Estructura

- **`src/core.rs`:** toda la lógica (bottles, motores, ejecución, snapshots, CrossOver). No depende de la interfaz y tiene sus tests.
- **`src/gui.rs`:** la app (egui), con tema propio en color vino y Avenir Next del sistema.
- **`src/main.rs`:** sin argumentos abre la app; con argumentos funciona como CLI.
- **`tools/render_icon.py`:** genera `assets/icon.png` y `assets/Barrica.icns`.
- **`bundle.sh`:** empaqueta `target/Barrica.app`.

```sh
cargo test                  # incluye un ciclo completo con un `wine` falso
cargo clippy --all-targets
```

## Relación con CrossOver

Barrica se diseñó a partir de cómo funciona CrossOver por fuera: la estructura de su bundle, sus pantallas y el formato de los bottles (`cxbottle.conf`, `cxmenu.conf`). No incluye ni decompila su código.

La licencia de CrossOver (sección 3) prohíbe la ingeniería inversa de su software. Por eso Barrica nunca ejecuta el Wine interno de CrossOver por su cuenta: los bottles vinculados pasan por el `wine --bottle` oficial, que conserva el control de licencia. La misma licencia (sección 2) sí permite compilar versiones alternativas de su Wine a partir del código que CodeWeavers publica. Ese es el camino para un motor propio; ver [ROADMAP.md](ROADMAP.md).
