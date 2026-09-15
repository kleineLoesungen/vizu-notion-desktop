#!/usr/bin/env bash
# Baut dist/starter-<version>-<arch>-linux.tar.gz.
#
# Inhalt: die beiden Binaries, eine .desktop-Datei, Symbole, Handbuchseiten,
# Shell-Vervollständigungen und ein install.sh. Bewusst kein AppImage, kein
# Flatpak, kein .deb — ein Tarball funktioniert überall und ist nachvollziehbar.
# Wer ein Distributionspaket braucht, baut es aus diesem Tarball.
set -euo pipefail
cd "$(dirname "$0")/.."

NAME="starter"
DISPLAY_NAME="Starter"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
ARCH=$(uname -m)
STAGE="dist/$NAME-$VERSION-$ARCH-linux"

rm -rf "$STAGE"
mkdir -p "$STAGE"/{bin,share/applications,share/icons,share/man/man1,share/completions}

cargo build --release
cp "target/release/$NAME" "target/release/$NAME-gui" "$STAGE/bin/"

# Handbuchseiten und Vervollständigungen erzeugt die CLI selbst — so können
# sie nicht veralten.
"target/release/$NAME" man --out "$STAGE/share/man/man1" >/dev/null
for shell in bash zsh fish; do
    "target/release/$NAME" completions "$shell" > "$STAGE/share/completions/$NAME.$shell"
done

for size in 32 64 128 256 512; do
    mkdir -p "$STAGE/share/icons/hicolor/${size}x${size}/apps"
    cp "assets/icons/icon-$size.png" \
       "$STAGE/share/icons/hicolor/${size}x${size}/apps/$NAME.png"
done

# Exec ohne Pfad: install.sh legt das Binary in den PATH.
cat > "$STAGE/share/applications/$NAME.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=$DISPLAY_NAME
Comment=Notizen verwalten
Exec=$NAME-gui
Icon=$NAME
Terminal=false
Categories=Utility;TextEditor;
DESKTOP

cat > "$STAGE/install.sh" <<'INSTALL'
#!/usr/bin/env sh
# Installiert nach ~/.local — ohne Sonderrechte, wie es die XDG-Regeln vorsehen.
set -eu
PREFIX="${PREFIX:-$HOME/.local}"
DIR=$(cd "$(dirname "$0")" && pwd)

mkdir -p "$PREFIX/bin" "$PREFIX/share"
cp "$DIR"/bin/* "$PREFIX/bin/"
cp -r "$DIR"/share/applications "$DIR"/share/icons "$DIR"/share/man "$PREFIX/share/"

# Damit das Startmenü das neue Symbol sofort findet.
command -v update-desktop-database >/dev/null && \
    update-desktop-database "$PREFIX/share/applications" 2>/dev/null || true
command -v gtk-update-icon-cache >/dev/null && \
    gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true

echo "Installiert nach $PREFIX."
case ":$PATH:" in
    *":$PREFIX/bin:"*) ;;
    *) echo "Hinweis: $PREFIX/bin liegt nicht im PATH." ;;
esac
INSTALL
chmod +x "$STAGE/install.sh"

tar -czf "$STAGE.tar.gz" -C dist "$(basename "$STAGE")"
rm -rf "$STAGE"
echo "Fertig: $STAGE.tar.gz"
