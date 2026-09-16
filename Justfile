# Alle wiederkehrenden Handgriffe an einem Ort.
# `just` ohne Argument zeigt die Liste.
#
# Installation von just:  cargo install just   (oder brew install just)

# Absolute Pfade für das Wegwerfverzeichnis: `tauri dev` startet die Anwendung
# aus crates/desktop heraus, ein relativer Pfad landete dort.
local := justfile_directory() / ".local"

_default:
    @just --list --unsorted

# --- Erster Start ----------------------------------------------------------

# Einmalige Einrichtung: npm-Pakete, Werkzeuge, einmal durchbauen.
setup:
    rustup show
    npm ci
    cargo install --locked cargo-deny cargo-insta || true
    npm run build:ui
    cargo build
    @echo ""
    @echo "Fertig. Weiter mit:  just dev   oder   just cli source list"

# --- Entwicklung -----------------------------------------------------------

# Die Desktop-Anwendung starten. Oberfläche lädt bei jeder Änderung neu,
# Rust wird bei jeder Änderung neu gebaut.
dev:
    npm run tauri dev

# Wie `just dev`, aber auf .local/ statt auf den echten Daten.
dev-sandbox:
    VIZU_NOTION_DATA_DIR={{local}}/data VIZU_NOTION_CONFIG_DIR={{local}}/config npm run tauri dev

# Die Kommandozeile aufrufen:  just cli source list
cli *ARGS:
    cargo run -q -p vizu-notion-cli -- {{ARGS}}

# Wie `just cli`, aber auf .local/ — dieselben Daten wie `just dev-sandbox`.
sandbox *ARGS:
    VIZU_NOTION_DATA_DIR={{local}}/data VIZU_NOTION_CONFIG_DIR={{local}}/config \
        cargo run -q -p vizu-notion-cli -- {{ARGS}}

# TypeScript-Typen aus den Rust-Typen neu erzeugen (ui/src/bindings.ts).
bindings:
    VIZU_NOTION_UPDATE_BINDINGS=1 cargo test -q -p vizu-notion-desktop --test bindings
    @echo "ui/src/bindings.ts geschrieben. Weiter mit:  npm run build:ui"

# --- Prüfen ----------------------------------------------------------------

# Der Torwächter. Muss grün sein, bevor etwas committet wird.
check: fmt-check lint test deny
    @echo ""
    @echo "Alles grün."

# Formatieren: Rust und TypeScript.
fmt:
    cargo fmt --all
    npm run fmt:ui

fmt-check:
    cargo fmt --all --check

# Warnungen sind Fehler — sonst sammeln sie sich an.
# Biome prüft Format und Regeln der Oberfläche, tsc die Typen.
lint:
    cargo clippy --workspace --all-targets -- -D warnings
    npm run lint:ui
    npx tsc --noEmit

# Rust-Tests brauchen ui/dist: tauri::generate_context! bettet es ein.
test:
    npm run build:ui
    cargo test --workspace
    npm run test:ui

# Lizenzen und bekannte Sicherheitslücken der Abhängigkeiten — Rust und npm.
deny:
    cargo deny check
    npm audit --omit=dev --audit-level=high

# Geänderte Schnappschüsse der CLI ansehen und annehmen.
snapshots:
    cargo insta review

# Doppelte Kisten im Abhängigkeitsbaum — erste Anlaufstelle bei "zwei inkompatible Typen mit demselben Namen".
dupes:
    cargo tree --duplicates

# --- Ausliefern ------------------------------------------------------------

# Bündel für diesen Rechner: .app + .dmg auf macOS, .deb + .AppImage auf Linux.
bundle:
    npm run tauri build

# macOS: ein Bündel für Apple Silicon und Intel.
bundle-macos-universal:
    npm run tauri build -- --target universal-apple-darwin

# Kommandozeile mit Handbuch und Vervollständigungen: dist/vizu-notion-cli-<version>-<os>-<arch>.tar.gz
package-cli:
    ./scripts/package-cli.sh

# Symbole aus assets/icon.svg neu erzeugen.
icons:
    npx tauri icon assets/icon.svg -o crates/desktop/icons
    cd crates/desktop/icons && rm -rf android ios Square*.png StoreLogo.png icon.ico 64x64.png

# --- Aufräumen -------------------------------------------------------------

clean:
    cargo clean
    rm -rf dist .local ui/dist

# ACHTUNG: löscht die echten lokalen Daten dieser Anwendung.
data-reset:
    @echo "Löscht das Datenverzeichnis:"
    @cargo run -q -p vizu-notion-cli -- paths
    @printf "Wirklich? [j/N] " && read a && [ "$a" = "j" ]
    rm -rf "$(cargo run -q -p vizu-notion-cli -- paths --json | sed -n 's/.*"data_dir": "\(.*\)".*/\1/p')"
    @echo "Gelöscht."
