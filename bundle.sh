#!/bin/sh
# Empaqueta Barrica como app de macOS: ./bundle.sh  →  target/Barrica.app
set -e
cd "$(dirname "$0")"
cargo build --release
APP=target/Barrica.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/barrica "$APP/Contents/MacOS/barrica"
cp assets/Barrica.icns "$APP/Contents/Resources/"  # se regenera con tools/render_icon.py
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Barrica</string>
  <key>CFBundleIdentifier</key><string>dev.barrica.app</string>
  <key>CFBundleExecutable</key><string>barrica</string>
  <key>CFBundleIconFile</key><string>Barrica</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
EOF
echo "$APP"
