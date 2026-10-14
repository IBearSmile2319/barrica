#!/bin/sh
# Prueba de humo de engine/dist: versión, bottle nuevo, HTTPS (gnutls) y D3D11 en cada capa gráfica.
# Usa un bottle temporal; no toca los tuyos.
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
WINE=$HERE/dist/bin/wine
TMP=${TMPDIR:-/tmp}/barrica-check
LAYERS=$HERE/dist/lib/wine-layers
# Como Barrica: siempre con una capa gráfica (d3d11/dxgi no están en lib/wine). MoltenVK, sin su log.
export WINEPREFIX=$TMP/prefix WINEDEBUG=-all WINEDLLPATH=$LAYERS/wined3d MVK_CONFIG_LOG_LEVEL=0
rm -rf "$WINEPREFIX"
mkdir -p "$TMP"
"$(brew --prefix)/bin/x86_64-w64-mingw32-gcc" -O2 -o "$TMP/smoke.exe" "$HERE/smoke.c" -lwinhttp -ld3d11 -ld3d12 -ldxguid -lwinmm

"$WINE" --version
"$WINE" wineboot -u
test -f "$WINEPREFIX/drive_c/windows/system32/d3d11.dll" # wineboot la crea desde la capa (WINEDLLDIR*)
"$WINE" "$TMP/smoke.exe" https
WINEDLLPATH=$LAYERS/wined3d "$WINE" "$TMP/smoke.exe" d3d11
WINEDLLPATH=$LAYERS/wined3d "$WINE" "$TMP/smoke.exe" d3d12 # vkd3d → MoltenVK
WINEDLLPATH=$LAYERS/dxmt:$LAYERS/wined3d "$WINE" "$TMP/smoke.exe" d3d11 # DXMT no hace D3D12
# D3DMetal, si el usuario lo instaló con `barrica gptk` (se anuncia como AMD, fabricante 1002).
GPTK="$HOME/Library/Application Support/Barrica/Layers/d3dmetal/wine"
if [ -d "$GPTK" ]; then
    WINEDLLPATH="$GPTK:$LAYERS/wined3d" "$WINE" "$TMP/smoke.exe" d3d11
    WINEDLLPATH="$GPTK:$LAYERS/wined3d" "$WINE" "$TMP/smoke.exe" d3d12
fi
"$HERE/dist/bin/wineserver" -k || true
echo "OK"
