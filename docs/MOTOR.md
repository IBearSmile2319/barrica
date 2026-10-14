# Motor Barrica: un Wine propio afinado para juegos

Documento de diseño para la fase 1 y la fase 2 del [ROADMAP](../ROADMAP.md). El objetivo es dejar de depender de CrossOver para jugar e igualar o superar su rendimiento en juegos como CS2.

Las secciones marcadas **(por verificar)** son hipótesis que hay que comprobar con mediciones antes de darlas por buenas.

## 0. Decisión (análisis del 2026-10-13)

**Máquina:** M5 Pro (5P + 10E), 24 GB, macOS 27.0.1 con Rosetta completo. CrossOver 26.3.0 instalado (Wine 11.0 + D3DMetal 3.0 + DXMT 0.72, todo x86_64 bajo Rosetta). El bottle de CS2 corre con `WINEMSYNC=1` y `WINED3DMETAL=1`.

**Dos motores, no uno:**

| | Motor A: `x86_64` (ahora) | Motor B: `arm64` + FEX (después) |
|---|---|---|
| Qué es | El mismo Wine que CrossOver 26.3, compilado por nosotros | Wine ARM64 nativo; solo el código x86 del juego se emula (FEX, ARM64EC). Wine, DXMT y Metal corren nativos |
| Fuente | `crossover-sources-26.3.0.tar.gz` (ya descargado, ver abajo) | **Todavía no es pública** (2026-10-13): CodeWeavers solo lista 26.3.0; los parches de Wine para macOS (x18, páginas 4K, PAGEZERO, TSO por hilo) se anunciaron en wine-devel el 2026-08-07 y aún no están upstream; su FEX para macOS es un fork privado |
| Requisitos | Rosetta (completo hasta macOS 27; «subconjunto para juegos» desde macOS 28) | macOS 26.5+ y Xcode 26.4+ (ya cumplimos) |
| DX11 | D3DMetal (GPTK 3.0-3, que baja el usuario de Apple) o DXMT 0.80 | DXMT ARM64EC (ya compila en su CI; `winemetal.so` aarch64 sin verificar) |
| DX12 | D3DMetal | Nada aún: D3DMetal es x86_64 y de Apple |
| Para CS2 (DX11) | Paridad con CrossOver desde el día uno | Es el que aprovecha de verdad el Apple Silicon |

**Plan:** compilar A ya (hitos M1-M4) y diseñar todo lo demás (gestor de motores, capas por bottle, perfiles, `barrica bench`) de forma que B sea «otro motor más» cuando salga su código. **Pedir las fuentes del Preview a info@codeweavers.com**: la licencia §2 obliga a entregarlas «cheerfully upon request», y el Wine del Preview es LGPL. Vigilar los MR de Brendan Shanks en gitlab.winehq.org y CrossOver 27 (previsto para principios de 2027).

**Lo que hay en disco (`~/Library/Application Support/Barrica/`):**
- `Backups/CrossOver-26.3.0.app`: clon APFS de la app con firma verificada. Si falta `/Applications/CrossOver.app`, los bottles vinculados usan esta copia (`crossover_app()` en `src/core.rs`); sigue pasando por su `wine` oficial, con su control de licencia. **Nunca va dentro de Barrica.app** (eso sería redistribuirla, licencia §3b) **ni se usa como motor propio**.
- Dónde se engancha D3DMetal en el Wine de CodeWeavers: `dlls/winemac.drv/d3dmetal.c`, `loader/wine.inf.in:739` (DLL falsas), `dlls/ntdll/loader.c:4018` (`__wine_unix_call` exportado para las DLL PE de D3DMetal) y `dlls/setupapi/devinst.c:5282`. En CrossOver el framework vive en `lib64/apple_gptk/external/D3DMetal.framework`; GPTK lo trae en `lib/external/`.

**Compilación de A: `engine/build.sh`** (sin sudo y sin Homebrew x86_64):
- **Trabajo** en `~/Library/Caches/Barrica/engine/` (ruta sin espacios; autotools no los soporta): `src/` (el tarball, sha256 `ac99c8ca…b6872`), `deps/` (prefijo x86_64 propio), `build-wine/`, `build.log`.
- **Herramientas** (Homebrew arm64, nativas): `bison` 3.8 (el de macOS es 2.3), `mingw-w64` (GCC 16), `autoconf`, `automake`, `libtool`, `cmake`.
- **Truco de velocidad:** `CC="clang -arch x86_64"` con `--build=x86_64-apple-darwin`. El compilador corre nativo en arm64 y genera x86_64; solo las pruebas de `configure` corren bajo Rosetta.
- **Dependencias x86_64:** gmp 6.3.0 y nettle 3.10 estáticos dentro de gnutls 3.8.3, y freetype 2.13.3, todos del tarball. El nettle del tarball viene **sin `Makefile.in`**: se usa el oficial de GNU (sha256 fijado en el script), que tiene los mismos 510 `.c`/`.h` byte a byte. gnutls necesita `NETTLE_LIBS`/`HOGWEED_LIBS`/`GMP_LIBS` explícitas con la ruta de los `.a`.
- **MoltenVK 1.4.2 oficial** (Apache-2.0, adelgazado a x86_64). El `win32u/vulkan.c` de CodeWeavers no compila sin Vulkan en macOS. Es más nuevo que el 1.2.10 de CrossOver 26.3, y Wine solo usa sus extensiones públicas.
- **Nombres de las dylibs:** en `deps/` llevan ruta absoluta, porque herramientas del build como `sfnt2fon` enlazan freetype. La copia que va al motor lleva `@rpath/…`. Wine hace `dlopen()` del nombre que detecta `configure`, y lo detecta sin ruta (dyld no mira el rpath), así que se fuerza con `ac_cv_lib_soname_{gnutls,freetype,MoltenVK}=@rpath/…`. Las `.so` llevan rpath `@loader_path/../../` (= `lib/`).
- **Wine:** `--enable-archs=i386,x86_64 --with-mingw --with-coreaudio --with-freetype --with-gnutls --with-vulkan --without-x --without-wayland --without-sdl --without-gstreamer --without-cups --without-ffmpeg --disable-tests`. `--without-opengl` no se usa: en macOS solo quita EGL, y el OpenGL de `winemac.drv` va aparte.
- **Wine Mono 10.4.1 y Gecko 2.47.4** dentro del motor, para que los bottles nuevos no pregunten.
- **Salida:** `engine/dist/`; `bundle.sh` lo mete en `Barrica.app/Contents/Resources/engine/` y es el motor por defecto. Otros motores van en `~/Library/Application Support/Barrica/Engines/<nombre>/bin/wine` y se eligen en Ajustes o por bottle.

**Capas gráficas por bottle, sin copiar DLL** (`graphics_layer()` en `src/core.rs`):
- Wine busca cada DLL builtin primero en su carpeta (`lib/wine/<arch>-windows`) y después en las de `WINEDLLPATH`, y carga la `.so` de la misma carpeta donde encontró la DLL (`find_builtin_dll` en `dlls/ntdll/unix/loader.c`).
- Copiar DLL de capas al `system32` del bottle **no sirve**: DXMT y D3DMetal llevan la marca «Wine builtin DLL», y en ese caso Wine carga la builtin de su carpeta (`load_builtin`, mismo fichero).
- Así que `build.sh` saca del motor las que chocan (`dxgi`, `d3d10core`, `d3d11`, `d3d12`, `d3d12core`) a `lib/wine-layers/wined3d/`, pone DXMT en `lib/wine-layers/dxmt/` y Barrica pone `WINEDLLPATH` a la capa del bottle. Si falta la elegida, WineD3D. `d3d9` se queda en el motor (DXMT no lo cubre).
- `WINEDLLPATH` = capa elegida y **detrás WineD3D**: lo que una capa no trae sale de WineD3D (el `d3d10` de DXMT, por ejemplo). Por eso `d3d10` también sale del motor: D3DMetal trae el suyo.
- `winemetal` (el puente de DXMT a Metal) va con el resto de Wine, no en la capa: su `.so` busca `winemac.so` y `ntdll.so` en `@loader_path`.
- **D3DMetal** (`barrica gptk <volumen «Evaluation environment for Windows games»>`, `install_gptk()` en `src/core.rs`):
  - Se copia su `redist/lib` **tal cual** a `~/Library/Application Support/Barrica/Layers/d3dmetal/` (fuera de la app). Su licencia prohíbe modificarlo; solo permite uso personal para evaluar juegos y redistribución no comercial.
  - Sus `wine/x86_64-unix/*.so` son enlaces a `../../external/libd3dshared.dylib`, así que la estructura `wine/` + `external/` se conserva y la capa es `d3dmetal/wine`.
  - Se renombra `nvngx-on-metalfx` a `nvngx`, como indica el Léeme de Apple. MetalFX se activa con `D3DM_ENABLE_METALFX=1`; `ROSETTA_ADVERTISE_AVX=1` ayuda a los juegos que exigen AVX.
  - No hace falta copiar `nvapi64`/`nvngx` al bottle: el `wine.inf` de CodeWeavers ya crea esas DLL falsas.
- `wineboot` crea en `system32` las DLL de la capa activa (Wine exporta sus rutas a Windows como `WINEDLLDIR0..N`). Por eso Barrica crea los bottles siempre con una capa puesta. Al cambiar de capa, esas DLL falsas cargan la de la capa nueva.

**Resultados (2026-10-13, M5 Pro, macOS 27.0.1):**

| Prueba (`engine/check.sh` y Barrica) | Resultado |
|---|---|
| Compilación completa | 1,0 GB en `engine/dist`; la app con motor ocupa lo mismo |
| Bottle nuevo con Barrica (Mono incluido) | 585 MB |
| Bloc de notas y `winecfg` (hito M1) | Abren; MSync «up and running» |
| HTTPS por schannel | HTTP 200 (gnutls carga por `@rpath`) |
| D3D11 por WineD3D | Nivel 9_3, tanto por OpenGL como por Vulkan: MoltenVK no tiene *geometry shaders*, que hacen falta para el 10 |
| **D3D11 por DXMT** | **Nivel 11_0, Apple M5 Pro, directo a Metal** |
| **D3D11 por D3DMetal** 3.0 y 4.0 beta 3 | **Nivel 11_0**; se anuncia como «AMD Compatibility Mode» (fabricante 0x1002) |
| D3D12 por WineD3D (vkd3d → MoltenVK 1.4.2) | Nivel 11_0 |
| D3D12 con la capa DXMT | Falla (`E_NOINTERFACE`): DXMT es solo DX10/11; por eso «Automático» prefiere D3DMetal |
| **D3D12 por D3DMetal** 3.0 y 4.0 beta 3 | **Nivel 12_2** |

**GPTK 4.0 beta 3** (instalado; la 3.0 queda en `Layers/d3dmetal-3.0/` para comparar o volver atrás):
- D3DMetal 4 encaja con el `winemac.drv` de CrossOver 26.3, al menos al crear dispositivos D3D11 y D3D12.
- Trae `d3d12core` y ya no trae `atidxx64`. Licencia idéntica a la de la 3.0.
- Variables nuevas: `D3DM_MTL4` (backend Metal 4 para DX12, activo por defecto en macOS 27) y `D3DM_MAX_FPS` (limitador).
- Se publican mejoras de un 6 % a un 66 % frente a GPTK 3 según el juego: hay que medirlo nosotros.

**Pendiente:**
- Steam y un juego DX11 (M2), y CS2 con D3DMetal en un clon de su bottle (M3), midiendo 3.0 contra 4.0 beta 3.
- SDL para mandos, GStreamer para vídeos y CUPS.

**Lo que no sabemos (por verificar):** si existe un GPTK 4 (un informe lo menciona); si DXMT 0.80 rinde más o menos que D3DMetal 3.0 en CS2 (no hay benchmark controlado en 2026, solo anécdotas de menos RAM y FPS estables); si el «subconjunto» de Rosetta de macOS 28 cubre Wine.

## 1. Qué es (y qué no es)

- **No es escribir Wine desde cero.** Son unos 5 millones de líneas y 30 años de trabajo.
- **Sí es compilar y distribuir nuestro propio Wine**, con parches propios y con las capas gráficas y los ajustes elegidos para juegos.
- **Base:** `crossover-sources-<versión>.tar.gz`, el código libre que CodeWeavers publica en <https://www.codeweavers.com/crossover/source>. Trae su Wine con los parches para macOS (MSync, mejoras de Rosetta, etc.). Como alternativa queda Wine upstream o wine-staging, para comparar.
- **Legalidad:**
  - **Wine** es LGPL.
  - **La licencia de CrossOver** (sección 2) permite expresamente compilar versiones alternativas de su Wine, y (sección 3) prohíbe la ingeniería inversa de sus binarios. Por eso trabajamos solo con el código fuente publicado, nunca con sus binarios.
  - **D3DMetal** es de Apple (Game Porting Toolkit). Se usa con la copia que el usuario descargue y no se redistribuye.

## 2. Dónde se pierde rendimiento en un Mac con Apple Silicon

Un juego de Windows pasa por varias capas, y cada una es una palanca:

```
juego x86 ─ Rosetta 2 ─ Wine (ntdll, wineserver, winemac.drv) ─ traducción gráfica ─ Metal ─ GPU
```

| Capa | Coste típico | Palanca |
|---|---|---|
| **CPU:** Rosetta 2 traduce x86_64 a ARM | El código del juego corre traducido | Wine x86_64 con WoW64 nuevo. AVX/AVX2 bajo Rosetta en macOS recientes **(por verificar)** para juegos que lo exigen |
| **Sincronización:** eventos y semáforos de Windows | Idas y vueltas al `wineserver` | **MSync** (semáforos Mach, parche de CodeWeavers). Esync y fsync son solo para Linux |
| **Gráficos:** DirectX a Metal | La mayor diferencia entre motores | Elegir por juego: D3DMetal, DXMT o DXVK+MoltenVK (sección 3) |
| **Shaders:** compilación en caliente | Tirones la primera vez | Cachés persistentes por bottle y precalentado (sección 4) |
| **Ventana y entrada:** `winemac.drv` | Latencia del ratón y captura del cursor | Entrada de ratón en crudo, fijación del cursor, pantalla completa nativa |
| **Audio:** `winecoreaudio.drv` | Latencia, artefactos de captura | Ajustar el búfer y arreglar el remuestreo del micrófono (CS2 robótico) |
| **Sistema:** macOS | Prioridades | Activar el **Modo Juego** de macOS (sección 5) |

## 3. Capas gráficas: cuál usar y cuándo

| Capa | Traduce | Licencia | Cuándo |
|---|---|---|---|
| **D3DMetal** (Apple) | DX11 y DX12 → Metal directo | Propietaria; viene del GPTK del usuario | Primera opción para DX11 y DX12 modernos (CS2, la mayoría de AAA) |
| **DXMT** | DX10 y DX11 → Metal directo | Libre | Alternativa libre a D3DMetal para DX11; sin doble traducción |
| **DXVK + MoltenVK** | DX9 a DX11 → Vulkan → Metal | Libre (zlib / Apache-2.0) | DX9 y juegos donde DXMT falla; tiene doble traducción, así que más coste |
| **WineD3D** | DX → OpenGL | LGPL | Último recurso; OpenGL está obsoleto en macOS |
| **vkd3d-proton + MoltenVK** | DX12 → Vulkan → Metal | Libre | Experimental en Mac; MoltenVK no cubre todo lo que pide **(por verificar)** |

Lo que hace falta en Barrica:
- **Por bottle:** instalar y quitar las DLL de cada capa (`d3d11.dll`, `dxgi.dll`, etc. en `system32` y `syswow64`) con sus overrides. Hoy solo ponemos las variables.
- **Perfiles por juego:** capa elegida, variables (`DXVK_ASYNC`, `MTL_HUD_ENABLED`, etc.) y versión de Windows. Es lo que en CrossOver hacen los «CrossTie».
- **DLSS → MetalFX:** CrossOver tiene un interruptor de DLSS (lo vimos en su interfaz) que, por lo que parece, usa la traducción de D3DMetal a MetalFX **(por verificar)**. Ofrecerlo cuando el GPTK lo soporte.

## 4. Shaders y tirones

- **Cachés por bottle**, en `…/Barrica/Caches/<bottle>/`:
  - **DXVK:** `DXVK_STATE_CACHE_PATH`.
  - **D3DMetal y Metal:** la caché de shaders de Metal del sistema; medir si sobrevive entre sesiones **(por verificar)**.
- **Precalentado:** primera partida «de calentamiento» o reproducir cachés compartidas por la comunidad, si la licencia del juego lo permite.
- **No borrar cachés** al cambiar el motor de versión, salvo incompatibilidad.

## 5. macOS a nuestro favor

- **Modo Juego:** macOS lo activa en pantalla completa para apps categorizadas como juego (`LSApplicationCategoryType = public.app-category.games`). Da prioridad de CPU y GPU y reduce la latencia de mandos y audio Bluetooth. El proceso visible es el de Wine, así que hay que lanzarlo desde un bundle con esa categoría **(por verificar cómo lo hace CrossOver)**.
- **Pantalla completa nativa y modo Retina:** Retina apagado (renderizar a 1x) suele rendir más. Exponerlo como «Calidad o Rendimiento» por juego.
- **Energía:** advertir si el Mac está en Modo de bajo consumo.

## 6. Cómo medir (sin datos no hay «alto rendimiento»)

- **Herramientas:**
  - **HUD de Metal** (`MTL_HUD_ENABLED=1`): FPS y tiempo de fotograma en todas las capas.
  - **`DXVK_HUD`:** cuando se use DXVK.
  - **Registro de tiempos de fotograma** a archivo, para comparar.
- **Escena fija:** en CS2, el mapa de la comunidad «FPS Benchmark» del Workshop, con los mismos ajustes gráficos y la misma resolución.
- **Métricas:** FPS medio, 1 % y 0,1 % más bajos (los tirones), tiempo hasta el primer fotograma, uso de CPU y memoria, y latencia percibida del ratón.
- **Línea base:** el mismo juego en CrossOver 26.x sobre el mismo Mac. **Meta:** igualarlo y luego superarlo con nuestros ajustes.
- **Automatizar:** `barrica bench <bottle> <perfil>` para lanzar, medir N minutos y guardar un informe junto a los logs.

## 7. Construcción

**Herramientas:**
- Xcode/clang.
- `mingw-w64`, o llvm-mingw, para las partes PE (las DLL de Windows).
- `bison` 3 o superior (el de macOS es 2.3).
- `freetype` para x86_64.
- MoltenVK y las capas DXVK y DXMT.

**Arquitectura:** el lado Unix en **x86_64** (corre bajo Rosetta); el lado Windows en i386 y x86_64 con `--enable-archs=i386,x86_64` (WoW64 nuevo, sin librerías de 32 bits del sistema).

**Opciones de rendimiento:**
- `-O2` como base. Probar `-O3` y LTO en las piezas calientes (`ntdll`, `win32u`, `winemac.drv`) midiendo, no a ciegas.
- Compilar para `x86-64-v2/v3` solo si Rosetta lo soporta en el macOS mínimo que elijamos **(por verificar)**.
- Sin `--enable-tests`, y con símbolos aparte para depurar.

**Salida:** `engine/dist/` con `bin/`, `lib/wine/…`, MoltenVK, DXVK y DXMT; `bundle.sh` lo mete en `Barrica.app/Contents/Resources/engine/`, así que instalar la app es instalar el motor. Para probar otras versiones sin tocar la app: `~/Library/Application Support/Barrica/Engines/<nombre>/`. Varias conviven y cada bottle elige la suya, así que volver atrás es trivial.

**Automatización:** script `engine/build.sh` escrito por nosotros y reproducible. Más adelante, compilación en CI con macOS para publicar motores firmados.

## 8. Parches propios (lo que CrossOver no nos deja hacer)

1. **Micrófono:** el remuestreo de captura de `winecoreaudio` es el sospechoso del audio robótico en CS2.
2. **Ratón:** entrada en crudo y sensibilidad 1:1 para shooters.
3. **Ajustes por juego** aplicados por el propio motor, sin tocar el registro a mano.
4. **Telemetría local de rendimiento** para `barrica bench`.

## 9. Riesgos

- **Compatibilidad:** el motor puede compilar pero romper juegos. Mitigación: conservar CrossOver vinculado como plan B y hacer snapshots antes de cambiar de motor.
- **Antitrampas:** VAC (CS2) funciona en Wine. Otros sistemas (EAC, BattlEye) dependen de que el desarrollador habilite Wine/Proton; un motor propio no lo resuelve.
- **D3DMetal:** depende de Apple y de su licencia del GPTK; no podemos redistribuirlo ni modificarlo.
- **Mantenimiento:** seguir cada versión de CrossOver y Wine supone trabajo continuo. Automatizar la compilación y las pruebas es obligatorio.

## 10. Hitos

| Hito | Criterio de «hecho» |
|---|---|
| M1: el motor compila | `winecfg` y el Bloc de notas abren en un bottle propio |
| M2: Steam y DX11 libre | Steam instala y un juego DX11 corre con DXMT o DXVK |
| M3: D3DMetal | Con el GPTK del usuario, CS2 abre en un clon de su bottle |
| M4: paridad | En «FPS Benchmark», FPS medio y 1 % bajos a menos de un 5 % de CrossOver |
| M5: superar | Algún parche propio (micrófono o entrada) mejora algo medible frente a CrossOver |
