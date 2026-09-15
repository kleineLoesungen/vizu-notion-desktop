# Alle wiederkehrenden Handgriffe an einem Ort.
# `just` ohne Argument zeigt die Liste.
#
# Installation von just:  cargo install just

_default:
    @just --list --unsorted

# --- Erster Start ----------------------------------------------------------

# Einmalige Einrichtung: Werkzeuge nachinstallieren, einmal durchbauen.
setup:
    rustup show
    cargo install --locked cargo-deny cargo-insta || true
    cargo build
    @echo ""
    @echo "Fertig. Weiter mit:  just gui   oder   just cli note list"

# --- Entwicklung -----------------------------------------------------------

# Die Oberfläche starten.
gui:
    cargo run -p starter-gui

# Die Kommandozeile aufrufen:  just cli note add "Titel"
cli *ARGS:
    cargo run -q -p starter-cli -- {{ARGS}}

# Wie `just cli`, aber auf .local/ statt auf den echten Daten.
sandbox *ARGS:
    STARTER_DATA_DIR=.local/data STARTER_CONFIG_DIR=.local/config \
        cargo run -q -p starter-cli -- {{ARGS}}

# Neu bauen, sobald sich etwas ändert (benötigt: cargo install cargo-watch).
watch:
    cargo watch -x 'check --workspace --all-targets'

# --- Prüfen ----------------------------------------------------------------

# Der Torwächter. Läuft genauso in der CI. Muss grün sein, bevor etwas committet wird.
check: fmt-check lint test deny
    @echo ""
    @echo "Alles grün."

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

# Warnungen sind Fehler — sonst sammeln sie sich an.
lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

# Lizenzen und bekannte Sicherheitslücken der Abhängigkeiten.
deny:
    cargo deny check

# Geänderte Schnappschüsse ansehen und annehmen.
snapshots:
    cargo insta review

# Doppelte Kisten im Abhängigkeitsbaum — erste Anlaufstelle bei "zwei inkompatible Typen mit demselben Namen".
dupes:
    cargo tree --duplicates

# --- Ausliefern ------------------------------------------------------------

# macOS: dist/Starter.app, universal (arm64 + x86_64).
bundle-macos:
    ./scripts/bundle-macos.sh

# macOS: nur für diesen Rechner — schneller beim Ausprobieren.
bundle-macos-host:
    ./scripts/bundle-macos.sh --host

# Linux: dist/starter-<version>-<arch>-linux.tar.gz
package-linux:
    ./scripts/package-linux.sh

# Symbole aus assets/icon.svg neu erzeugen (nur macOS).
icons:
    ./scripts/icons.sh

# Handbuchseiten und Shell-Vervollständigungen nach dist/.
docs-cli:
    cargo run -q -p starter-cli -- man --out dist/man
    mkdir -p dist/completions
    for shell in bash zsh fish; do \
        cargo run -q -p starter-cli -- completions $shell > dist/completions/starter.$shell; \
    done
    @echo "dist/man und dist/completions geschrieben."

# --- Aufräumen -------------------------------------------------------------

clean:
    cargo clean
    rm -rf dist .local

# ACHTUNG: löscht die echten lokalen Daten dieser Anwendung.
data-reset:
    @echo "Löscht das Datenverzeichnis:"
    @cargo run -q -p starter-cli -- paths
    @printf "Wirklich? [j/N] " && read a && [ "$a" = "j" ]
    rm -rf "$(cargo run -q -p starter-cli -- paths | awk '/^Daten / {print $2}')"
    @echo "Gelöscht."
