# Starter — Desktop-Anwendungen und Kommandozeile in Rust

Startpunkt für kleine Desktop-Werkzeuge auf **macOS und Linux**: eine
grafische Oberfläche mit **egui**, ein Kommandozeilenwerkzeug mit **clap**, und
dazwischen **eine** Fachlogik, die beide benutzen. Daten in einer
**eingebetteten SQLite**.

Ein Binary je Schale, keine Systemabhängigkeit, kein Node, kein Webview, kein
Datenbankserver. Gemacht für die Arbeit mit KI-Assistenten: klare Regeln in
[CLAUDE.md](CLAUDE.md), fertige Rezepte, ein strenges `just check` — und ein
Test, der die Architektur bewacht.

Die Gegenstücke fürs Web: [starter-rust](https://github.com/kleineLoesungen/starter-rust)
und [starter-php](https://github.com/kleineLoesungen/starter-php).

---

## In fünf Minuten laufen

Voraussetzungen: [rustup](https://rustup.rs),
[just](https://github.com/casey/just) (`cargo install just`).

```bash
just setup    # Werkzeuge nachinstallieren, einmal durchbauen
just gui      # Oberfläche starten
```

Und dieselbe Anwendung auf der Kommandozeile:

```bash
just cli note add "Einkauf" --body "Milch, Brot"
just cli note list
just cli note list --json | jq '.[].title'
```

Beides arbeitet auf **derselben** Datenbank. Wo die liegt, sagt
`just cli paths`.

Ohne `just` geht es auch von Hand:

```bash
cargo run -p starter-gui
cargo run -p starter-cli -- note list
```

---

## Eigenes Projekt aus dem Kit

Das Kit selbst bleibt unverändert. Zwei Wege zu einer eigenen Kopie:

**Über GitHub** — der grüne Knopf **„Use this template"** legt ein eigenes
Repository mit frischer Historie an. Danach klonen und umbenennen:

```bash
git clone git@github.com:<du>/notizbuch.git
cd notizbuch
./scripts/new-project.sh notizbuch "Notizbuch"
just check
```

**Lokal**, ohne Umweg über GitHub:

```bash
git clone <pfad-oder-url-zum-kit> notizbuch
cd notizbuch
./scripts/new-project.sh notizbuch "Notizbuch"
git remote remove origin        # zeigt sonst noch auf das Kit
just check
```

Das Template benennt **nichts** um — das tut erst `new-project.sh`: Kistennamen,
Binaries, Bündelkennung, Umgebungsvariablen, Datenverzeichnis, Fenstertitel.

**Das Skript direkt nach dem Klonen ausführen.** Bis dahin heißt das Projekt
noch „starter" und benutzt dasselbe Datenverzeichnis wie das Kit — zwei
Projekte mit demselben Namen teilen sich unbemerkt eine Datenbank.

### Spätere Verbesserungen aus dem Kit holen

Die Historien sind getrennt, `cherry-pick` arbeitet trotzdem darüber hinweg:

```bash
git remote add kit https://github.com/kleineLoesungen/starter-desktop-cli
git fetch kit
git log --oneline kit/main
git cherry-pick <sha>
```

Ein `merge` wäre der falsche Griff — er zöge die ganze Kit-Geschichte herein.

---

## Was drin ist

| Bereich | Umsetzung |
|---|---|
| Aufbau | Arbeitsbereich mit `core` · `cli` · `gui`, bewacht von einem Test |
| Oberfläche | egui/eframe 0.36 — reines Rust, keine Systembibliothek außer OpenGL |
| Kommandozeile | clap 4, Unterbefehle, Shell-Vervollständigungen, Handbuchseiten |
| Maschinenlesbar | `--json` bei **jedem** Befehl, auch bei Fehlern; eigene Rückgabewerte |
| Daten | SQLite eingebettet, `STRICT`-Tabellen, WAL, nummerierte SQL-Migrationen |
| Kennungen | UUIDv7, in der Bedienung als achtstelliges Endstück |
| Einstellungen | TOML zum Anfassen, von Oberfläche und CLI gemeinsam benutzt |
| Aussehen | Gestaltungsmarken an einer Stelle, hell/dunkel/Systemeinstellung, freie Akzentfarbe |
| Prüfung | Feldgenaue Fehler, in der Oberfläche am Feld, in der API als `error.fields` |
| Zeit | in UTC gespeichert, in Ortszeit angezeigt |
| Protokoll | `tracing` — CLI auf stderr, Oberfläche in eine Datei |
| macOS | `.app`-Bündel von Hand, universal (arm64 + x86_64), Symbol, ad-hoc signiert |
| Linux | Tarball mit `.desktop`, Symbolen, Handbuch, Vervollständigungen, `install.sh` |
| CI | GitHub Actions auf macOS **und** Linux, Release auf Tag |
| Qualität | `cargo fmt`, Clippy als Fehler, `cargo deny`, `unsafe` verboten |
| Tests | 51 Tests: Fachlogik, Schichtwächter, CLI-Schnappschüsse, Oberfläche ohne Fenster |

---

## Wo was liegt

```
crates/
├── core/                  ← FACHLOGIK. Kennt weder clap noch egui noch stdout.
│   ├── migrations/          Nummerierte SQL-Dateien, ins Binary einkompiliert
│   ├── src/note.rs          Beispielressource zum Kopieren
│   ├── src/config.rs        Einstellungen, TOML
│   ├── src/paths.rs         Die einzige Stelle mit macOS/Linux-Unterschieden
│   ├── src/db.rs            Verbindung, Pragmas, Migrationen
│   ├── src/error.rs         Zwei Fehlersorten, Validator
│   └── tests/layering.rs    Der Wächter über die Schichtregel
│
├── cli/                   ← Kommandozeile. Dünn.
│   ├── src/args.rs          Nur die Befehlsstruktur
│   ├── src/commands/        Was die Unterbefehle tun
│   ├── src/output.rs        Mensch oder JSON — jeder Befehl kann beides
│   ├── src/exit.rs          Rückgabewerte und Fehlerausgabe
│   └── tests/snapshots/     Erwartete Ausgabe, verwaltet mit `just snapshots`
│
└── gui/                   ← Oberfläche. Dünn.
    ├── src/model.rs         Zustand und Action — kennt die Anwendung nicht
    ├── src/views/           Zeichnet nur, gibt Action zurück
    ├── src/app.rs           Die einzige Stelle, an der Model auf core trifft
    ├── src/theme.rs         Alle Farben und Abstände
    └── src/widgets.rs       Wiederkehrende Bedienelemente

scripts/
├── new-project.sh         Kit zu eigenem Projekt umbenennen
├── bundle-macos.sh        dist/Starter.app
├── package-linux.sh       dist/starter-<version>-<arch>-linux.tar.gz
└── icons.sh               Symbole aus assets/icon.svg (nur macOS)
```

---

## Ausliefern

```bash
just bundle-macos      # dist/Starter.app, universal
just package-linux     # dist/starter-0.1.0-x86_64-linux.tar.gz
```

Signieren und Notarisieren für die Weitergabe: [docs/RELEASE.md](docs/RELEASE.md).

Ein Tag `v*` löst beides in der CI aus und hängt die Ergebnisse an das Release.

---

## Weiterlesen

| Datei | Inhalt |
|---|---|
| [CLAUDE.md](CLAUDE.md) | Die Regeln. Fallstricke, festgenagelte Versionen, egui-Eigenheiten |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Warum die Schichten so geschnitten sind |
| [docs/CLI.md](docs/CLI.md) | Befehle, Rückgabewerte, JSON-Format |
| [docs/DESIGN.md](docs/DESIGN.md) | Aussehen ändern, Gestaltungsmarken, Symbole |
| [docs/RECIPES.md](docs/RECIPES.md) | Neue Ressource, neuer Befehl, neue Ansicht, neue Migration |
| [docs/RELEASE.md](docs/RELEASE.md) | Bündeln, signieren, notarisieren, veröffentlichen |
