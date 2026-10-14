# Barrica

Gestor de *bottles* de Wine para macOS, escrito en Rust. Es un proyecto nuevo, inspirado en CrossOver, que no usa código de CrossOver ni de GitHub.

Un **bottle** es un Windows aparte (un `WINEPREFIX`) donde instalas programas y juegos. Barrica los crea, los organiza y los abre, y añade cosas que CrossOver no trae: snapshots instantáneos, un log por ejecución y una biblioteca con el arte de tus juegos de Steam.

## Qué hace hoy

- **Descubrir:** un catálogo de apps y tiendas de Windows (Steam, Battle.net, Epic, GOG Galaxy, EA app, 7-Zip, Notepad++…) con sus logos, una ficha por app y su progreso de instalación. Cada app se instala en su propio bottle, preparado como ella necesita. El catálogo se actualiza solo, sin publicar versiones de Barrica: ver [catalog/README.md](catalog/README.md).
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

Para bottles propios (no vinculados), Barrica trae su propio motor: el Wine 11 de CrossOver 26.3 compilado desde el código libre de CodeWeavers, con MSync, MoltenVK y DXMT. Se compila una vez (sin sudo; unos 30 minutos) y `bundle.sh` lo mete dentro de la app:

```sh
brew install bison mingw-w64 autoconf automake libtool cmake
engine/build.sh             # deja el motor en engine/dist (detalles en docs/MOTOR.md)
engine/check.sh             # prueba de humo: HTTPS y D3D11 por WineD3D, DXMT y D3DMetal
./bundle.sh
```

D3DMetal (DirectX 11/12, lo que usa CrossOver para CS2) es de Apple y no viene con Barrica. Baja el Game Porting Toolkit de developer.apple.com, monta su imagen «Evaluation environment for Windows games» y ejecuta `barrica gptk "/Volumes/Evaluation environment for Windows games 3.0"`. Se instala en `~/Library/Application Support/Barrica/Layers/d3dmetal/` sin modificarlo, porque su licencia lo prohíbe.

También sirven otros motores: Wine de <https://wiki.winehq.org/MacOS> (Homebrew desactivó sus casks el 2026-09-01), motores en `~/Library/Application Support/Barrica/Engines/<nombre>/` o Game Porting Toolkit en `/usr/local/opt/game-porting-toolkit`. Se eligen en **Motor → Elegir ejecutable de wine…**.

## CLI

```sh
barrica list
barrica link-crossover "Counter-Strike 2"     # usar un bottle de CrossOver con su motor
barrica programs "Counter-Strike 2"
barrica run "Counter-Strike 2" ".../Steam/steam.exe" -applaunch 730
barrica snapshot "Counter-Strike 2" "antes de los mods"
barrica catalog tiendas                       # apps que Barrica sabe instalar
barrica install steam                         # la instala en un bottle nuevo llamado «Steam"
barrica help                                  # todos los comandos
```

## Dónde guarda las cosas

| Qué | Dónde |
|---|---|
| Bottles | `~/Library/Application Support/Barrica/Bottles/<nombre>/` (WINEPREFIX + `barrica.toml`) |
| Bottles vinculados | Solo `barrica.toml` con `crossover = "<bottle>"`; los datos siguen en CrossOver |
| Respaldo de CrossOver | `~/Library/Application Support/Barrica/Backups/CrossOver-<versión>.app`; si falta `/Applications/CrossOver.app`, los vinculados usan este |
| Snapshots | `~/Library/Application Support/Barrica/Snapshots/<bottle>/<etiqueta>/` |
| Registros | `~/Library/Application Support/Barrica/Logs/<bottle>/` |
| Ajustes | `~/Library/Application Support/Barrica/settings.toml` |
| Catálogo y logos | `~/Library/Application Support/Barrica/Catalog/` (caché; se baja solo) |
| Instaladores | `~/Library/Application Support/Barrica/Downloads/<app>/` (se borran al terminar) |

`BARRICA_HOME` cambia la carpeta raíz, por ejemplo para usar otro disco o para los tests.

## Estructura

- **`src/core.rs`:** toda la lógica (bottles, motores, ejecución, snapshots, CrossOver). No depende de la interfaz y tiene sus tests.
- **`src/catalog.rs`:** el catálogo de instalación: recetas, versiones de winget-pkgs, descargas verificadas e instalación.
- **`src/gui.rs`:** la app (egui), con tema propio en color vino y Avenir Next del sistema. **`src/gui/discover.rs`** es la pantalla Descubrir.
- **`catalog/catalog.toml`:** las apps del catálogo (la app descarga la versión más nueva).
- **`src/main.rs`:** sin argumentos abre la app; con argumentos funciona como CLI.
- **`tools/render_icon.py`:** genera `assets/icon.png` y `assets/Barrica.icns`.
- **`bundle.sh`:** empaqueta `target/Barrica.app`, con el motor de `engine/dist` dentro si está compilado.
- **`engine/`:** `build.sh` compila el motor, y `check.sh` con `smoke.c` lo prueba.

```sh
cargo test                  # incluye un ciclo completo con un `wine` falso
cargo clippy --all-targets
```

## Relación con CrossOver

Barrica se diseñó a partir de cómo funciona CrossOver por fuera: la estructura de su bundle, sus pantallas y el formato de los bottles (`cxbottle.conf`, `cxmenu.conf`). No incluye ni decompila su código.

La licencia de CrossOver (sección 3) prohíbe la ingeniería inversa de su software. Por eso Barrica nunca ejecuta el Wine interno de CrossOver por su cuenta: los bottles vinculados pasan por el `wine --bottle` oficial, que conserva el control de licencia. La misma licencia (sección 2) sí permite compilar versiones alternativas de su Wine a partir del código que CodeWeavers publica. Ese es el camino para un motor propio; ver [ROADMAP.md](ROADMAP.md).
