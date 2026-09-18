# Vizu Notion — Notion-Datenbanken als Diagramme

Eine Desktop-Anwendung für **macOS und Linux**, die Seiten aus Notion holt und
daraus Diagramme zeichnet: **Mermaid** aus eigenen Vorlagen, ein **Fluss**
entlang der Nachfolger und eine **Metro-Karte** auf einer Zeitachse. Die Daten
liegen danach lokal in SQLite — gezeichnet wird ohne Netz.

Dieselbe Fachlogik bedient ein Kommandozeilenwerkzeug: `vizu-notion render
fahrplan` liefert denselben Mermaid-Text, den das Fenster anzeigt.

Vorhandene `.mmd`-Vorlagen und eine `sources.json` lassen sich einlesen und
laufen unverändert weiter (`template import`, `source import`).

---

## In fünf Minuten laufen

Voraussetzungen: [rustup](https://rustup.rs), [Node.js 22](https://nodejs.org),
[just](https://github.com/casey/just) (`brew install just` oder
`cargo install just`). Unter **Linux** zusätzlich die WebKitGTK-Pakete:

```bash
sudo apt install libwebkit2gtk-4.1-dev libxdo-dev libssl-dev \
    libayatana-appindicator3-dev librsvg2-dev build-essential
```

(Fedora, Arch und andere: [Tauri-Voraussetzungen](https://v2.tauri.app/start/prerequisites/).)

```bash
just setup    # npm-Pakete, Werkzeuge, einmal durchbauen
just dev      # Anwendung starten, lädt bei Änderungen neu
```

Beim ersten Start führt die Anwendung durch die vier Schritte: Token,
Quelle, Abruf, Diagramm.

### Was man in Notion braucht

1. Unter [notion.so/profile/integrations](https://www.notion.so/profile/integrations)
   eine **interne Integration** anlegen. Ein persönlicher Token ist nicht nötig.
2. Die Datenbank in Notion über **… → Verbindungen** mit der Integration teilen.
   Ohne diesen Schritt sieht die Anwendung sie nicht.
3. Den Token in der Anwendung unter *Einstellungen* speichern — er liegt im
   Schlüsselbund des Systems, nicht in einer Datei.

Für die Quelle genügt der **Link zur Datenbank**: in Notion oben rechts
••• → „Link kopieren" und im Quellen-Dialog einfügen. Die Anwendung holt dann
von selbst die Spalten und schlägt die Zuordnung vor. Die Kennung allein geht
auch.

---

## Was die Anwendung kann

| | |
|---|---|
| **Quellen** | Eine Notion-Datenbank unter einem Namen, mit einer Zuordnung: welche Spalte ist `title`, `date`, `next`, `tag` … Der Vorschlag dafür kommt aus den Spalten selbst. |
| **Abrufen** | Alle Seiten samt Relationen, mit Takt und Wiederholung bei 429. Das Ergebnis bleibt in SQLite; Zeichnen braucht danach kein Netz. |
| **Mermaid** | Eigene Vorlagen mit Handlebars, Beispiele und Spickzettel im Editor, Vorschau beim Tippen. |
| **Fluss** | Ohne Vorlage: Knoten und Pfeile entlang der Rolle `next`, angeordnet in core. |
| **Metro-Karte** | Zeitachse aus `date`, Linien entlang `next`, Bänder aus `tag`, mit Abzweigungen und Umsteigestationen. |
| **Filtern** | Einzelne Seiten ausblenden, „nur Verwandte" zeigen, je Quelle aufklappen. |
| **Ansichten** | Das Eingestellte — Diagramm, ausgeblendete Seiten — unter einem Namen sichern und wieder öffnen. |
| **Ausgeben** | SVG speichern; auf der Kommandozeile Mermaid-Text oder JSON. |

---

## Die Kommandozeile

Dieselbe Anwendung, dieselbe Datenbank:

```bash
just cli token set                                    # Token speichern
just cli source add Projekte --database <LINK> --auto # Link aus Notion, Zuordnung vorschlagen lassen
just cli fetch                                        # von Notion holen
just cli render fahrplan                              # Mermaid-Text auf stdout
just cli metro Projekte --json | jq '.lines | length'
just cli view list                                    # gespeicherte Ansichten
```

Wo die Datenbank liegt, sagt `just cli paths`. Ohne die echten Daten
anzufassen: `just dev-sandbox` und `just sandbox source list`.

Alle Befehle, Rückgabewerte und das JSON-Format: [docs/CLI.md](docs/CLI.md).

---

## Wo was liegt

```
crates/
├── core/                    ← FACHLOGIK. Kennt weder clap noch tauri noch stdout.
│   ├── migrations/            Nummerierte SQL-Dateien, ins Binary einkompiliert
│   ├── src/source.rs          Quellen, Zuordnung, Vorschlag aus den Spalten
│   ├── src/notion/            Notion-API: Transport, Takt, Modelle
│   ├── src/fetch.rs           Abrufen, zwischenspeichern, Schema ansehen
│   ├── src/secret.rs          Der Notion-Token
│   ├── src/template/          Mermaid aus Vorlagen, mit Handlebars
│   ├── src/flow.rs            Flussdiagramm: Graph und Anordnung
│   ├── src/metro.rs           Metro-Karte: Ketten, Zeitachse, Bänder
│   ├── src/view.rs            Gespeicherte Ansichten
│   ├── src/hidden.rs          Versteckte Diagramme
│   ├── src/paths.rs           Die einzige Stelle mit macOS/Linux-Unterschieden
│   └── tests/layering.rs      Der Wächter über die Schichtregel (Rust + TypeScript)
│
├── cli/                     ← Kommandozeile `vizu-notion`. Dünn.
│
└── desktop/                 ← Tauri-Schale. Dünn.
    ├── tauri.conf.json        Fenster, CSP, Bündel
    ├── capabilities/          Was das Fenster darf
    ├── src/commands.rs        Die IPC-Befehle
    ├── tests/ipc_contract.rs  Befehle ohne Fenster aufrufen; Rust ↔ api.ts
    └── tests/bindings.rs      Erzeugt ui/src/bindings.ts

ui/src/
├── api.ts                   ← Die EINZIGE Stelle, die mit Rust spricht
├── bindings.ts              Erzeugt aus Rust — nicht von Hand ändern
├── App.tsx                  Zustand, ruft api
├── components/              Zeichnen nur: Diagramme, Listen, Dialoge
├── lib/mermaid.ts           Mermaid, nachgeladen
├── theme.css                Die EINZIGE Datei mit Farben
└── app.css                  Aufbau und Abstände
```

Rechnen tut `core`, zeichnen tut `ui`: Auch die Anordnung von Fluss und
Metro-Karte kommt aus Rust, damit Fenster und Kommandozeile nicht
auseinanderlaufen können.

---

## Ausliefern

```bash
just bundle                   # macOS: .app + .dmg   Linux: .deb + .AppImage
just bundle-macos-universal   # Apple Silicon + Intel in einem Bündel
just package-cli              # die Kommandozeile als Tarball
```

Ergebnisse unter `target/release/bundle/` bzw. `dist/`.

**Fertige Pakete** entstehen von selbst: Ein Versionsschild (`git tag v0.2.0 &&
git push --tags`) lässt GitHub Actions auf macOS und Linux bauen und hängt
`.dmg`, `.deb`, `.AppImage` und den CLI-Tarball an einen Release-Entwurf. Sie
sind **nicht signiert** — macOS meldet deshalb beim ersten Start „beschädigt",
was `xattr -dr com.apple.quarantine "/Applications/Vizu Notion.app"` aufhebt.
Alles dazu: [docs/RELEASE.md](docs/RELEASE.md).

---

## Mitarbeiten

```bash
just check    # rustfmt, Clippy, Biome, tsc, alle Tests, cargo deny, npm audit
```

Warnungen sind Fehler. Die Regeln des Projekts stehen in
[CLAUDE.md](CLAUDE.md) — sie gelten für Menschen wie für KI-Assistenten.

Entstanden aus dem Kit
[vibe-starter-tauri](https://github.com/kleineLoesungen/vibe-starter-tauri).
Verbesserungen von dort holt man einzeln:

```bash
git remote add kit https://github.com/kleineLoesungen/vibe-starter-tauri
git fetch kit && git log --oneline kit/main
git cherry-pick <sha>
```

---

## Weiterlesen

| Datei | Inhalt |
|---|---|
| [CLAUDE.md](CLAUDE.md) | Die Regeln. Tauri-2-Fallstricke, Notion-Eigenheiten, Schichtregel |
| [docs/UMSETZUNG.md](docs/UMSETZUNG.md) | Der Plan: Phasen, Entscheidungen, Herkunft der Vorlagensprache |
| [docs/CLI.md](docs/CLI.md) | Befehle, Rückgabewerte, JSON-Format |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Warum die Schichten so geschnitten sind, wie die IPC-Grenze aussieht |
| [docs/RECIPES.md](docs/RECIPES.md) | npm-Paket einbinden, neuer Befehl, neue Ressource, Plugin, Hintergrundarbeit |
| [docs/DESIGN.md](docs/DESIGN.md) | Aussehen ändern, Farbmarken, Symbole |
| [docs/RELEASE.md](docs/RELEASE.md) | Bündeln, signieren, notarisieren, veröffentlichen |
