#!/usr/bin/env bash
# Erzeugt alle Symbolgrößen aus assets/icon.svg.
#
# Braucht macOS (qlmanage, sips, iconutil). Die Ergebnisse liegen mit im
# Verwaltungssystem — ein Bau unter Linux ruft dieses Skript nicht auf.
set -euo pipefail
cd "$(dirname "$0")/.."

SRC=assets/icon.svg
OUT=assets/icons
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

command -v qlmanage >/dev/null || { echo "Braucht macOS (qlmanage fehlt)." >&2; exit 1; }

mkdir -p "$OUT"

# SVG einmal groß rastern, danach nur noch verkleinern — jede Verkleinerung
# aus dem 1024er ist schärfer als eine eigene Rasterung.
qlmanage -t -s 1024 -o "$TMP" "$SRC" >/dev/null 2>&1
BASE="$TMP/$(basename "$SRC").png"
[ -f "$BASE" ] || { echo "Rastern fehlgeschlagen." >&2; exit 1; }

for size in 16 32 64 128 256 512 1024; do
    sips -z "$size" "$size" "$BASE" --out "$OUT/icon-$size.png" >/dev/null
done

# .icns für das macOS-Bündel.
ICONSET="$TMP/starter.iconset"
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$BASE" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z "$double" "$double" "$BASE" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$OUT/starter.icns"

echo "Symbole erzeugt in $OUT"
