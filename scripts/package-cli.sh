#!/usr/bin/env bash
# Baut dist/vizu-notion-cli-<version>-<os>-<arch>.tar.gz — die Kommandozeile zum
# Weitergeben, auf macOS und Linux gleich.
#
# Inhalt: das Binary, Handbuchseiten, Shell-Vervollständigungen und ein
# install.sh. Die Desktop-Anwendung liefert `just bundle` getrennt davon aus:
# Tauri kennt keine zweite ausführbare Datei im Bündel, ohne sie als
# „Sidecar" mit Zielplattform im Namen zu verlangen — das wäre mehr Umweg als
# ein eigener Tarball.
set -euo pipefail
cd "$(dirname "$0")/.."

NAME="vizu-notion"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
STAGE="dist/$NAME-cli-$VERSION-$OS-$ARCH"

rm -rf "$STAGE"
mkdir -p "$STAGE"/{bin,share/man/man1,share/completions}

cargo build --release -p vizu-notion-cli
cp "target/release/$NAME" "$STAGE/bin/"

# Handbuchseiten und Vervollständigungen erzeugt die CLI selbst — so können
# sie nicht veralten.
"target/release/$NAME" man --out "$STAGE/share/man/man1" >/dev/null
for shell in bash zsh fish; do
    "target/release/$NAME" completions "$shell" > "$STAGE/share/completions/$NAME.$shell"
done

cat > "$STAGE/install.sh" <<'INSTALL'
#!/usr/bin/env sh
# Installiert nach ~/.local — ohne Sonderrechte.
set -eu
PREFIX="${PREFIX:-$HOME/.local}"
DIR=$(cd "$(dirname "$0")" && pwd)

mkdir -p "$PREFIX/bin" "$PREFIX/share"
cp "$DIR"/bin/* "$PREFIX/bin/"
cp -r "$DIR"/share/man "$PREFIX/share/"

echo "Installiert nach $PREFIX."
echo "Vervollständigungen liegen in $DIR/share/completions."
case ":$PATH:" in
    *":$PREFIX/bin:"*) ;;
    *) echo "Hinweis: $PREFIX/bin liegt nicht im PATH." ;;
esac
INSTALL
chmod +x "$STAGE/install.sh"

tar -czf "$STAGE.tar.gz" -C dist "$(basename "$STAGE")"
rm -rf "$STAGE"
echo "Fertig: $STAGE.tar.gz"
