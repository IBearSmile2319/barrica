#!/bin/sh
# Código fuente completo del motor publicado (lo exige la LGPL de Wine):
#   target/barrica-engine-sources-<versión>.tar.gz
# Lleva las fuentes de CrossOver y de Nettle tal como las usó engine/build.sh, los scripts y los parches.
set -e
cd "$(dirname "$0")/.."
WORK=${BARRICA_ENGINE_WORK:-$HOME/Library/Caches/Barrica/engine}
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
NAME=barrica-engine-sources-$VERSION
STAGE=target/$NAME
rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$WORK"/crossover-sources-*.tar.gz "$STAGE/"
cp "$WORK"/addons/nettle-*.tar.gz "$STAGE/"
cp -R engine "$STAGE/engine"
rm -rf "$STAGE/engine/dist"
cp LICENSE THIRD_PARTY_NOTICES.md "$STAGE/"
cat > "$STAGE/LEEME.md" <<TXT
# Fuentes del motor de Barrica $VERSION

- crossover-sources-*.tar.gz: el código fuente de CrossOver que publica CodeWeavers (Wine, GnuTLS, GMP, FreeType…).
- nettle-*.tar.gz: Nettle, de ftp.gnu.org.
- engine/: build.sh (cómo se compila), patches/ (los cambios de Barrica a Wine, LGPL-2.1-or-later) y la prueba de humo.

Compilar: copia los tarballs a ~/Library/Caches/Barrica/engine/ (nettle en addons/) y ejecuta engine/build.sh.
MoltenVK, DXMT, Wine Mono y Wine Gecko se descargan de sus publicaciones oficiales; las URL y los
sha256 están en engine/build.sh y THIRD_PARTY_NOTICES.md.
TXT
tar -czf "target/$NAME.tar.gz" -C target "$NAME"
rm -rf "$STAGE"
echo "target/$NAME.tar.gz"
