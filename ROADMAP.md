# Hoja de ruta

Dónde está Barrica, qué le falta y el plan para acercarse a lo que hace CrossOver, sin depender de él.

## Estado actual (0.1)

**Funciona y está probado:**
- **Núcleo:** 3 tests, incluido un ciclo completo con un `wine` falso. Cubren crear, lanzar, programas y juegos de Steam, software instalado, snapshots y restaurar, duplicar, renombrar, Papelera, exportar e importar, importar y vincular CrossOver.
- **Bottles vinculados:** Barrica arrancó el motor de CrossOver sobre el bottle real de CS2 y leyó su software instalado.
- **Interfaz:** revisada con capturas reales de la biblioteca con arte de Steam, el banner, las pestañas, el formulario de bottle nuevo y la pantalla vacía.

**Sin probar todavía:**
- Ejecutar programas con un motor propio, porque no hay ninguno instalado.
- Abrir CS2 desde Barrica de principio a fin.

## Lo que falta (limitaciones conocidas)

| # | Falta | Hoy | Impacto |
|---|---|---|---|
| 1 | Motor Wine propio | Sin motor; Homebrew desactivó WineHQ | Los bottles propios no pueden ejecutar nada |
| 2 | Instalar DXVK, DXMT y MoltenVK en el bottle | La opción «Gráficos» solo pone variables y overrides | Los juegos DX10/11 no usan esas capas aunque estén elegidas |
| 3 | D3DMetal propio | Solo vía CrossOver (vinculado) o Game Porting Toolkit | CS2 y DX11/12 dependen de CrossOver |
| 4 | Catálogo de instalación (como CrossTie) | Solo «Ejecutar…» con el instalador | Instalar Steam, Battle.net, etc. es manual |
| 5 | Dependencias (vcredist, .NET, fuentes) | `barrica winetricks` en la CLI | Nada en la interfaz; los vinculados usan CrossOver |
| 6 | Integración con macOS | Nada | Sin launchers en Launchpad/Dock, sin asociaciones de archivos, sin abrir `.exe` desde Finder ni arrastrar y soltar |
| 7 | Canales de log (`WINEDEBUG`) en la interfaz | Solo `BARRICA_DEBUG` en la CLI | Diagnosticar exige terminal |
| 8 | «Ejecutar con opciones» (argumentos, carpeta, variables) | Solo en la CLI | |
| 9 | Micrófono robótico en CS2 | Sin resolver | Ver «Investigaciones abiertas» |
| 10 | Firma, notarización y actualizaciones | Nada | Gatekeeper avisa al compartir la app; sin actualizaciones automáticas |
| 11 | Tests de interfaz | Solo los del núcleo | |
| 12 | Snapshots con el bottle abierto | Solo un aviso de texto | Puede quedar un snapshot inconsistente; habría que bloquearlo |
| 13 | Tamaño de clones | `du` cuenta los bloques compartidos | Un clon APFS aparenta ocupar lo mismo que el original |
| 14 | Idiomas | Solo español | |

## Plan

### Fase 1: motor propio (independencia de CrossOver)
- **Fuente:** `crossover-sources-26.3.0.tar.gz` de codeweavers.com/crossover/source. La licencia de CrossOver, sección 2, permite compilar versiones alternativas de su Wine.
- **Herramientas:** `mingw-w64`, `bison` 3 o superior, y `freetype` compilado para x86_64. Se compila en x86_64 bajo Rosetta con el WoW64 nuevo, que soporta juegos de 32 bits.
- **Script:** `engine/build.sh`, escrito por nosotros, que deja el motor en `~/Library/Application Support/Barrica/Engines/<versión>/`.
- **Gestor de motores en la app:** instalar, elegir por bottle, ver la versión y borrar.
- **Meta:** que el Bloc de notas, `winecfg` y Steam abran en un bottle propio.

### Fase 2: gráficos para juegos
- **Por bottle:** instalar y quitar DXVK y DXMT, con MoltenVK incluido en el motor, al cambiar «Gráficos». Igual que los interruptores de CrossOver: Auto, D3DMetal, DXMT, DXVK o Wine.
- **D3DMetal:** a partir del Game Porting Toolkit que el usuario baje de Apple. Se usa localmente y no se redistribuye.
- **Prueba:** CS2 en un clon de su bottle, para comparar rendimiento y voz con CrossOver.

### Fase 3: instalación guiada (estilo CrossTie)
- **Recetas en TOML:** URL, sha256, dependencias, pasos posteriores y versión de Windows recomendada.
- **Catálogo inicial:** Steam, Battle.net, Epic, GOG Galaxy y EA App, con buscador en la app.
- **Dependencias propias:** vcredist, .NET y fuentes, sin necesitar winetricks.

### Fase 4: integración con macOS
- **Launchers como `.app`:** uno por programa en `~/Applications/Barrica/`, para Dock, Launchpad y Spotlight. Equivale a «cxmenu» de CrossOver.
- **Archivos:** asociaciones (equivale a «cxassoc»), abrir `.exe`, `.msi` y `.lnk` desde Finder (tipos de documento en el Info.plist) y arrastrar y soltar sobre la ventana.

### Fase 5: diagnóstico y soporte
- **Registro:** canales de `WINEDEBUG` con ajustes predefinidos (audio, gráficos, red), como el «Logging Options» de CrossOver, y un visor de logs dentro de la app.
- **Ejecutar con opciones:** argumentos, carpeta de trabajo y variables, y la opción de guardarlo como launcher.
- **Diagnóstico de audio:** dispositivos que Wine fijó en el registro, Bluetooth en modo manos libres, y eco por usar altavoces y micrófono del Mac a la vez.
- **Notas de compatibilidad** por programa.

### Fase 6: distribución
- **Instalación:** firma con Developer ID, notarización y DMG.
- **Actualizaciones:** automáticas mediante un appcast, al estilo Sparkle.
- **Calidad:** traducción al inglés y tests de interfaz (por ejemplo con `egui_kittest`).

## Investigaciones abiertas

**Micrófono robótico en CS2 (con CrossOver).** Pasa también con el micrófono del Mac, así que no es solo por Bluetooth. Sospechosos, por orden:
1. Procesado de voz de Steam (supresión de ruido, cancelación de eco y control de ganancia).
2. Remuestreo en la captura de `winecoreaudio`.
3. MSync o la carga de la CPU.

Además, el bottle tiene la salida fijada al jack de auriculares en `user.reg`. Con un motor propio (fase 1) podríamos depurar y parchear `winecoreaudio.drv`, algo imposible con CrossOver.
