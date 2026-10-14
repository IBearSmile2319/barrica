# Catálogo de instalación

`catalog.toml` es la lista de apps que Barrica sabe instalar: lo que muestra la pantalla **Descubrir** y lo que usan `barrica catalog` y `barrica install`.

## Por qué no hace falta publicar versiones de Barrica

| Qué cambia | Quién lo mantiene | Cómo llega a la app |
|---|---|---|
| Versión, URL y sha256 de las apps con `winget` | La comunidad de [winget-pkgs](https://github.com/microsoft/winget-pkgs) y los propios fabricantes, a diario | Barrica busca la versión más nueva al instalar |
| Apps con `url` | El fabricante: su instalador fijo siempre baja la última versión, y después la app se actualiza sola | Nada que hacer |
| Este archivo (apps nuevas, ajustes para Wine) | Quien lo edite en el repositorio | Barrica lo descarga al abrirse y se queda con el más nuevo (`updated`) entre ese y la copia que trae |
| Logos | [Simple Icons](https://simpleicons.org) (CC0) | Se descargan una vez y quedan en caché |

Solo hay que tocar este archivo para añadir una app o cambiar cómo se prepara su bottle. Al hacerlo, sube `updated` a la fecha del día: si no, las copias que ya tienen la versión anterior no lo reemplazan.

La app lo descarga de `catalog_url` en `settings.toml`. Por defecto es la copia de este archivo en la rama `master` del repositorio (vía `raw.githubusercontent.com`), así que **el repositorio tiene que ser público**. Si no lo es, apunta `catalog_url` a cualquier otra URL pública (por ejemplo un Gist) y Barrica usará esa.

## Formato

```toml
format = 1                 # versión del formato; Barrica ignora catálogos con otro
updated = "2026-10-14"     # gana el más nuevo entre el descargado y el incluido

[[category]]
id = "tiendas"
name = "Tiendas de juegos"

[[app]]
id = "steam"                       # único y estable: los bottles y la caché lo usan
name = "Steam"
publisher = "Valve"
category = "tiendas"
summary = "Una línea para la tarjeta."
description = "Un párrafo para la ficha (opcional)."
icon = "steam"                     # nombre en Simple Icons, o URL a un .svg/.png; vacío = iniciales
color = "#1B2838"                  # color de marca para el icono y la cabecera
featured = true                    # sale en «Destacadas»
homepage = "https://store.steampowered.com"

# De dónde sale el instalador: uno de los dos.
url = "https://cdn.akamai.steamstatic.com/client/installer/SteamSetup.exe"
# winget = "Valve.Steam"           # el identificador de winget-pkgs

file = "SteamSetup.exe"            # solo si la URL no trae el nombre del archivo
sha256 = "…"                       # solo con `url` fijo a una versión; con winget viene solo
args = []                          # argumentos para el instalador

# Cómo se prepara el bottle nuevo.
windows = "win10"                  # los de `barrica set <bottle> windows`
arch = "win64"                     # o "win32"
graphics = "auto"                  # auto, d3dmetal, dxvk o wined3d
deps = ["vcrun2022"]               # verbos de winetricks
env = { DXVK_ASYNC = "1" }

# Para saber si ya está instalada y abrirla. Relativo a drive_c; `users/*/` vale por cualquier usuario.
exe = ["Program Files (x86)/Steam/steam.exe"]

notes = "Aviso que se muestra en la ficha (opcional)."
```

### ¿`url` o `winget`?

- **`url`** cuando el fabricante publica un instalador fijo que siempre descarga la última versión (Steam, Battle.net, Ubisoft Connect…). No lleves `sha256`: el archivo cambia con cada versión.
- **`winget`** cuando cada versión tiene su propia URL (7-Zip, Notepad++, EA app…). Barrica elige el instalador `.exe` o `.msi` de la arquitectura del bottle (y en español, si hay), y comprueba el sha256 publicado. Los `.msix`, `.appx` y `.zip` no sirven para Wine y se saltan.

Para encontrar el identificador de winget, busca la carpeta del programa en [winget-pkgs/manifests](https://github.com/microsoft/winget-pkgs/tree/master/manifests): `manifests/n/Notepad++/Notepad++` es `Notepad++.Notepad++`.

### Compatibilidad

Una app con un campo que esta versión de Barrica no entiende (por ejemplo un `graphics` nuevo) se salta y el resto del catálogo se carga igual. Los campos desconocidos se ignoran. `cargo test` comprueba que todas las apps de este archivo se leen bien.
