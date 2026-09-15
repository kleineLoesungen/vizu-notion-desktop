#!/usr/bin/env bash
# Baut dist/Starter.app — von Hand, ohne Werkzeug dazwischen.
#
# Warum kein cargo-bundle: Das Bündel ist ein Verzeichnis mit fünf Dateien
# darin. Wer es selbst zusammensetzt, sieht jeden Schritt und kann später
# signieren, notarisieren und eigene Einträge in die Info.plist schreiben,
# ohne gegen die Annahmen eines Werkzeugs anzukämpfen.
#
# Aufruf:
#   ./scripts/bundle-macos.sh            universal (arm64 + x86_64)
#   ./scripts/bundle-macos.sh --host     nur die Architektur dieses Rechners
set -euo pipefail
cd "$(dirname "$0")/.."

NAME="Starter"                       # Anzeigename, steht unter dem Symbol
BIN="starter-gui"                    # Oberfläche, wird CFBundleExecutable
CLI="starter"                        # Kommandozeilenwerkzeug, kommt mit hinein

# ACHTUNG, macOS-Falle: Das Dateisystem unterscheidet in der Voreinstellung
# NICHT zwischen Groß- und Kleinschreibung. Würde die Oberfläche im Bündel
# "Starter" heißen, wäre sie dieselbe Datei wie die CLI "starter" — die zweite
# Kopie überschriebe die erste, und das Bündel startete die Kommandozeile.
# Deshalb behalten beide ihren gebauten Namen, und CFBundleExecutable zeigt
# auf "starter-gui". Der Anzeigename kommt aus CFBundleName.
BUNDLE_ID="de.kleineloesungen.starter"
APP="dist/$NAME.app"

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

if [ "${1:-}" = "--host" ]; then
    echo "Baue für diesen Rechner …"
    cargo build --release
    cp "target/release/$BIN" "$APP/Contents/MacOS/$BIN"
    cp "target/release/$CLI" "$APP/Contents/MacOS/$CLI"
else
    echo "Baue universal (arm64 + x86_64) …"
    for target in aarch64-apple-darwin x86_64-apple-darwin; do
        rustup target add "$target" >/dev/null
        cargo build --release --target "$target"
    done
    # lipo verschmilzt die beiden Binaries zu einer Datei, die auf Apple
    # Silicon und auf Intel läuft.
    lipo -create -output "$APP/Contents/MacOS/$BIN" \
        "target/aarch64-apple-darwin/release/$BIN" \
        "target/x86_64-apple-darwin/release/$BIN"
    lipo -create -output "$APP/Contents/MacOS/$CLI" \
        "target/aarch64-apple-darwin/release/$CLI" \
        "target/x86_64-apple-darwin/release/$CLI"
fi

cp assets/icons/starter.icns "$APP/Contents/Resources/$NAME.icns"

# LSMinimumSystemVersion 11.0: darunter gibt es kein Apple Silicon.
# NSHighResolutionCapable fehlt absichtlich nicht — ohne den Eintrag rendert
# macOS das Fenster hochskaliert und alles wirkt unscharf.
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>              <string>$NAME</string>
    <key>CFBundleDisplayName</key>       <string>$NAME</string>
    <key>CFBundleIdentifier</key>        <string>$BUNDLE_ID</string>
    <key>CFBundleVersion</key>           <string>$VERSION</string>
    <key>CFBundleShortVersionString</key><string>$VERSION</string>
    <key>CFBundleExecutable</key>        <string>$BIN</string>
    <key>CFBundleIconFile</key>          <string>$NAME</string>
    <key>CFBundlePackageType</key>       <string>APPL</string>
    <key>LSMinimumSystemVersion</key>    <string>11.0</string>
    <key>NSHighResolutionCapable</key>   <true/>
</dict>
</plist>
PLIST

# Ohne Signatur startet das Bündel auf dem eigenen Rechner, aber nicht auf
# einem fremden. Die Ad-hoc-Signatur ("-") genügt für den eigenen Gebrauch;
# für die Weitergabe siehe docs/RELEASE.md.
codesign --force --deep --sign - "$APP" 2>/dev/null || \
    echo "Hinweis: codesign nicht verfügbar — Bündel bleibt unsigniert."

echo "Fertig: $APP"
echo "Starten mit:  open $APP"
echo "CLI daneben:  $APP/Contents/MacOS/$CLI"
