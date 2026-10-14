# Brag plan: Barrica 0.2, «nuestro propio Wine»

Segundo vídeo. El primero (site/video/barrica.mp4) presenta Descubrir, instalar con un clic y la pila del motor.
Este cuenta otra historia: **lo compilamos nosotros, y se nota**. Todo lo que sale en pantalla es real:
líneas del log de `engine/build.sh`, resultados de `engine/check.sh`, la ventana de Barrica con CS2 en marcha
y los dos parches propios.

## Rubric

- **Qué es:** Barrica, una app libre para macOS que abre juegos y programas de Windows, ahora con su propio motor Wine.
- **Para quién:** quien juega en un Mac con Apple Silicon y no quiere depender de una licencia de pago.
- **Qué la distingue:** el motor se compila desde el código libre de CodeWeavers, con capas gráficas a elegir por bottle (WineD3D, DXMT, D3DMetal) y arreglos propios.
- **Lo más impresionante:** D3D12 nivel 12_2 sobre Metal con D3DMetal 4 beta; CS2 abriendo con el motor propio.
- **Gancho visual:** una terminal que compila el motor y termina en «Motor listo · 1.0 GB».
- **UI real:** la ventana de Barrica con «CS2 motor Barrica · En ejecución» (captura del usuario, recortada a la ventana; fuera quedan los ficheros del escritorio).
- **Tono:** `cinematic` contenido. Tráiler técnico, frases cortas, sin humo.
- **Frase para compartir:** «Barrica 0.2 trae su propio Wine: lo compilamos desde el código libre de CodeWeavers y ya abre Counter-Strike 2 en un Mac.»

Sin datos personales: no aparece el nombre de usuario de Steam ni nada del escritorio.
No se afirma «más FPS que…»: no está medido.

## Identidad

Colores del primer vídeo y de la web: vino `#b3264f` / `#d23a6a`, fondo `#120e14`, texto `#f4eef6`.
Tipografía del sistema (SF Pro como -apple-system), SF Mono para la terminal. 1920×1080, 30 fps.

## Storyboard (110 BPM, un tiempo = 0,545 s; los cortes caen en tiempos)

| # | Escena | Inicio–fin | Texto en pantalla | Movimiento | Sonido |
|---|---|---|---|---|---|
| 1 | **Gancho:** la terminal compila | 0,00–2,73 | `$ ./engine/build.sh` y el log real; cierra con «Motor listo · 1.0 GB» | Tecleo, líneas que suben rápido, la última se ilumina | Tecleo suave, «tick» en cada línea |
| 2 | **Revelación** | 2,73–5,45 | «Barrica 0.2» / «Ahora con su propio Wine» | Icono que cae con rebote, título que sube | Golpe grave al entrar el icono |
| 3 | **Capas gráficas** | 5,45–9,82 | «DirectX, directo a Metal» y tres tarjetas: WineD3D · D3D11 nivel 9_3 / DXMT 0.80 · D3D11 nivel 11_0 / D3DMetal 4 beta · D3D12 nivel 12_2 | Tarjetas que entran una a una; la última late | «Ding» afinado por tarjeta |
| 4 | **CS2 en marcha** | 9,82–14,18 | Ventana real de Barrica; rótulo «Counter-Strike 2, con nuestro motor» | Zoom lento hacia «En ejecución» | Barrido al entrar |
| 5 | **Arreglos propios** | 14,18–17,45 | «Parches propios»: ✓ El micro ya no congela el juego · ✓ La interfaz de Steam ya no sale negra | Líneas tipo prueba que pasan | Dos «ding» |
| 6 | **Cierre** | 17,45–20,73 | Icono · «Tu Wine. Tu Mac. Tus juegos.» · ibearsmile2319.github.io/barrica · «Gratis y de código abierto · GPL-3.0» | Entrada escalonada y luego quieto | Acorde final que se apaga |

Duración total: 20,73 s.

## Sonido

Música original sintetizada (sin licencias de terceros), 110 BPM, La menor: Am–F–C–G.
Bombo suave, hi-hats a contratiempo, bajo, acordes de pad y un arpegio; entra a pleno en la escena 2.
Los efectos van en la misma tonalidad (los «ding» en notas del acorde) y por debajo de la música.
