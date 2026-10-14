#!/bin/sh
# Imagen de disco para publicar: target/Barrica.app → target/Barrica.dmg (con el acceso a Aplicaciones).
set -e
cd "$(dirname "$0")/.."
STAGE=target/dmg
rm -rf "$STAGE" target/Barrica.dmg
mkdir -p "$STAGE"
cp -Rc target/Barrica.app "$STAGE/" 2>/dev/null || cp -R target/Barrica.app "$STAGE/"
ln -s /Applications "$STAGE/Aplicaciones"
cat > "$STAGE/Léeme.txt" <<TXT
Barrica: arrastra Barrica.app a Aplicaciones.

La primera vez, macOS puede decir que no puede comprobar al desarrollador (Barrica aún no está
notarizada por Apple). Para abrirla, en Terminal:

    xattr -dr com.apple.quarantine /Applications/Barrica.app

Más ayuda: https://ibearsmile2319.github.io/barrica/
TXT
hdiutil create -volname Barrica -srcfolder "$STAGE" -ov -format UDZO -fs HFS+ target/Barrica.dmg
rm -rf "$STAGE"
echo target/Barrica.dmg
