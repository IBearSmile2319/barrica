# Motor Barrica: un Wine propio afinado para juegos

Documento de diseño para la fase 1 y la fase 2 del [ROADMAP](../ROADMAP.md). El objetivo es dejar de depender de CrossOver para jugar e igualar o superar su rendimiento en juegos como CS2.

Las secciones marcadas **(por verificar)** son hipótesis que hay que comprobar con mediciones antes de darlas por buenas.

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

**Salida:** `~/Library/Application Support/Barrica/Engines/<versión>/` con `bin/`, `lib/wine/…`, MoltenVK, DXVK y DXMT. Varias versiones conviven y cada bottle elige la suya, así que volver atrás es trivial.

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
