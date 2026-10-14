# Licencias y avisos de terceros

Barrica es software libre. Este archivo dice bajo qué licencia va cada parte y dónde está su código fuente. Viaja dentro de `Barrica.app` (`Contents/Resources/`) y del motor (`engine/share/licenses/`), junto con los textos completos de cada licencia.

## Barrica

| Parte | Licencia |
|---|---|
| La app (`src/`), los scripts (`bundle.sh`, `engine/*.sh`, `tools/`), el catálogo (`catalog/`) y la web (`site/`) | [GPL-3.0-or-later](LICENSE) |
| Los parches del motor (`engine/patches/`) | [LGPL-2.1-or-later](engine/COPYING.LIB), la misma de Wine, porque modifican Wine |

Copyright © 2026 los autores de Barrica.

## El motor (`Barrica.app/Contents/Resources/engine/`)

El motor es Wine compilado por Barrica a partir del código fuente que CodeWeavers publica de CrossOver. No es CrossOver ni incluye ninguna parte cerrada de CrossOver. Lo compila `engine/build.sh` y cada versión publicada de Barrica adjunta su código fuente completo (`barrica-engine-sources-<versión>.tar.gz`) en la misma página de descarga.

| Componente | Versión | Licencia | Código fuente |
|---|---|---|---|
| Wine (de las fuentes de CrossOver) | 26.3.0 | LGPL-2.1-or-later | <https://www.codeweavers.com/crossover/source> y la descarga de cada versión de Barrica |
| GnuTLS | 3.8.3 | LGPL-2.1-or-later | Dentro de las fuentes de CrossOver |
| GMP | 6.3.0 | LGPL-3.0-or-later o GPL-2.0-or-later | Dentro de las fuentes de CrossOver |
| Nettle | 3.10 | LGPL-3.0-or-later o GPL-2.0-or-later | <https://ftp.gnu.org/gnu/nettle/> |
| FreeType | 2.13.3 | FreeType License (FTL) | Dentro de las fuentes de CrossOver |
| MoltenVK | 1.4.2 | Apache-2.0 | <https://github.com/KhronosGroup/MoltenVK> |
| DXMT | 0.80 | MIT | <https://github.com/3Shain/dxmt> |
| Wine Mono | 10.4.1 | MIT y otras (detalle en su código) | <https://dl.winehq.org/wine/wine-mono/> |
| Wine Gecko | 2.47.4 | MPL-2.0 | <https://dl.winehq.org/wine/wine-gecko/> |

Portions of this software are copyright © 2024 The FreeType Project (www.freetype.org). All rights reserved.

Cómo ejercer tus derechos de la LGPL: el motor se compila de cero con `engine/build.sh`, y Wine carga GnuTLS, FreeType y MoltenVK como bibliotecas dinámicas (`engine/lib/*.dylib`), así que puedes sustituirlas por versiones modificadas.

## Lo que Barrica no incluye

- **D3DMetal** (Game Porting Toolkit) es de Apple. Barrica no lo distribuye: lo instala cada usuario desde su propia copia con `barrica gptk`, sin modificarlo, y su licencia se copia junto a él.
- **CrossOver** es de CodeWeavers. Los bottles vinculados se abren con el CrossOver que tenga instalado el usuario y su licencia.
- **Los instaladores del catálogo** se descargan de cada fabricante o de las URL que publica winget-pkgs, en el momento de instalar.

## Datos que la app descarga

- **Catálogo:** `catalog/catalog.toml` de este repositorio (GPL-3.0-or-later).
- **Versiones de las apps:** [winget-pkgs](https://github.com/microsoft/winget-pkgs), de Microsoft y la comunidad (MIT).
- **Logos:** [Simple Icons](https://simpleicons.org) (CC0-1.0). Los logos y nombres son marcas de sus dueños y se usan solo para identificar cada app. Barrica no está afiliada a ninguna de ellas, ni a CodeWeavers ni a Apple.

## Dependencias de Rust

La app enlaza unas 450 bibliotecas de Rust, todas compatibles con la GPL-3.0: en su mayoría MIT o Apache-2.0, y algunas Zlib, BSD, ISC, Unicode-3.0, BSL-1.0 y OFL-1.1 (fuentes de egui). La lista completa con la licencia de cada una se genera al empaquetar y va al final de la copia de este archivo que viaja en `Barrica.app`.
