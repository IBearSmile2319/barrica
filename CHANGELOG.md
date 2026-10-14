# Cambios

## 0.2.0 (2026-10-14)

- **Motor propio:** Wine 11 compilado desde el código libre de CrossOver 26.3, con MSync, MoltenVK 1.4.2 y DXMT 0.80, dentro de Barrica.app. Capas gráficas por bottle (DXMT, D3DMetal y WineD3D) y `barrica gptk` para instalar D3DMetal.
- **Parches del motor:** el micrófono ya no congela el juego (0001) y la interfaz de Steam se ve (0002).
- **Descubrir:** catálogo de 20 apps y tiendas con logos, ficha, progreso y aviso al terminar. Se actualiza solo desde el repositorio y winget-pkgs. `barrica catalog` y `barrica install`.
- **Acerca de:** versión y commit, componentes del motor, D3DMetal, sistema, aviso de versiones nuevas, licencias y créditos. `barrica version`.
- **Publicación:** licencia GPL-3.0-or-later, avisos de terceros, `.dmg` universal (Apple Silicon e Intel) y Releases compiladas en GitHub Actions, con el código fuente del motor.
- «En ejecución» solo si el wineserver sigue vivo de verdad.

## 0.1.0

- Primera versión: bottles, biblioteca con arte de Steam, snapshots APFS, registros por ejecución, herramientas de Windows, ajustes, terminal, CLI y bottles de CrossOver vinculados o copiados.
