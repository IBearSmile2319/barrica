#!/bin/sh
# Empaqueta Barrica como app de macOS: ./bundle.sh  →  target/Barrica.app
#   UNIVERSAL=1 ./bundle.sh   binario para Apple Silicon e Intel
#                             (antes: rustup target add aarch64-apple-darwin x86_64-apple-darwin)
#   CODESIGN_IDENTITY="Developer ID Application: …" ./bundle.sh   firma con tu certificado (si no, firma ad hoc)
set -e
cd "$(dirname "$0")"
if [ -n "${UNIVERSAL:-}" ]; then
    cargo build --release --target aarch64-apple-darwin
    cargo build --release --target x86_64-apple-darwin
    BIN=target/barrica-universal
    lipo -create -output "$BIN" target/aarch64-apple-darwin/release/barrica target/x86_64-apple-darwin/release/barrica
else
    cargo build --release
    BIN=target/release/barrica
fi
APP=target/Barrica.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/barrica"
cp assets/Barrica.icns "$APP/Contents/Resources/"  # se regenera con tools/render_icon.py
# Motor propio (engine/build.sh lo deja en engine/dist): va dentro de la app y es el motor por defecto.
[ -d engine/dist ] && cp -Rc engine/dist "$APP/Contents/Resources/engine"
# Licencias: la GPL de Barrica y los avisos de terceros, con la lista de bibliotecas de Rust enlazadas.
cp LICENSE "$APP/Contents/Resources/LICENSE"
tools/notices.sh > "$APP/Contents/Resources/THIRD_PARTY_NOTICES.md"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Barrica</string>
  <key>CFBundleDisplayName</key><string>Barrica</string>
  <key>CFBundleIdentifier</key><string>dev.barrica.app</string>
  <key>CFBundleExecutable</key><string>barrica</string>
  <key>CFBundleIconFile</key><string>Barrica</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>NSHumanReadableCopyright</key><string>© 2026 los autores de Barrica · GPL-3.0-or-later</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSMicrophoneUsageDescription</key><string>Los juegos y programas de Windows usan el micrófono, por ejemplo para el chat de voz.</string>
</dict></plist>
PLIST
# Apple Silicon no abre binarios sin firma: como mínimo, firma ad hoc («-»).
codesign --force --deep --sign "${CODESIGN_IDENTITY:--}" "$APP"
echo "$APP"
