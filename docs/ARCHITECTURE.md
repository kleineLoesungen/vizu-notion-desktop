# Aufbau

## Das Problem, das dieses Kit löst

Eine Desktop-Anwendung mit Weboberfläche hat eine typische
Verfallsgeschichte: Die Oberfläche entsteht zuerst, und weil JavaScript gerade
offen ist, landen die Regeln dort — „Titel darf nicht leer sein" als `if` in
einer Komponente, eine SQL-Abfrage über ein Plugin direkt aus dem Webview.
Später kommt eine Kommandozeile dazu und bekommt eine eigene, leicht
abweichende Fassung derselben Regeln. Von da an gibt es jede Prüfung zweimal,
in zwei Sprachen.

Dazu kommt eine zweite Bruchstelle, die es in einer reinen Rust-Anwendung
nicht gibt: die **IPC-Grenze**. Ein umbenanntes Feld in Rust kompiliert, das
TypeScript kompiliert auch — und im Fenster steht `undefined`.

Dagegen hilft keine Ermahnung, sondern eine Anordnung, in der die Fehler
auffallen, bevor jemand klickt.

```
                          ┌──────────────────────┐
                          │   crates/core        │
      ┌──────────────────▶│   Fachlogik          │◀──────────────────┐
      │                   │   Prüfung · SQLite   │                   │
      │                   └──────────────────────┘                   │
      │                                                              │
┌─────┴──────────────┐                              ┌────────────────┴─────┐
│  crates/cli        │                              │  crates/desktop      │
│  clap              │                              │  Tauri-Befehle       │
│  Mensch │ --json   │                              │  ApiError            │
└────────────────────┘                              └──────────▲───────────┘
                                                               │ IPC (JSON)
                                                    ┌──────────┴───────────┐
                                                    │  ui/  (Webview)      │
                                                    │  api.ts · App.tsx    │
                                                    │  components · lib    │
                                                    └──────────────────────┘
```

Die Pfeile gehen nur in eine Richtung. `core` weiß nicht, dass es Schalen gibt.

## Was wo hingehört

| Frage | Antwort |
|---|---|
| Darf ein Titel leer sein? | `core` |
| Wird ein leerer Titel rot umrandet? | `ui/src/components` |
| Welchen Rückgabewert liefert ein leerer Titel auf der Kommandozeile? | `cli` |
| Wie heißt der IPC-Befehl, welche Argumente hat er? | `desktop/src/commands.rs` + `ui/src/api.ts` |
| Wie heißt die Spalte in der Datenbank? | `core` |
| Wie wird Markdown zu HTML, wie ein Diagramm gezeichnet? | `ui/src/lib` |
| Wie wird der Zeitstempel angezeigt? | die jeweilige Schale |
| Wie wird der Zeitstempel gespeichert? | `core` — immer UTC |

Die Faustregel: **Was auf beiden Wegen gleich sein muss, steht in `core`.**
Was nur mit Darstellung zu tun hat — Markdown, Mermaid, Farben —, steht in
`ui/`. Die Kommandozeile gibt den Markdown-Text roh aus; sie muss ihn nicht
verstehen.

## Die IPC-Grenze

```
ui/src/api.ts                         crates/desktop/src/commands.rs
─────────────                         ──────────────────────────────
api.sources.fetch(id)
  └▶ invoke("source_fetch",    ──▶    #[tauri::command]
            { id })                   async fn source_fetch(state, id: Uuid)
                                        └▶ Quelle + Token holen (kurz gesperrt)
                                        └▶ fetch::download(…) in spawn_blocking, ohne Sperre
                                        └▶ fetch::store(app.conn(), &download)
  ◀── FetchStatus              ◀──    Ok(FetchStatus)     → JSON
  ◀── throw ApiError           ◀──    Err(ApiError)       → abgelehntes Promise
```

Vier Riegel halten diese Grenze zusammen:

1. **Typen aus Rust.** `#[derive(TS)]` an `Note`, `NoteInput`, `Config`,
   `ApiError` … erzeugt `ui/src/bindings.ts`. `tests/bindings.rs` schlägt
   fehl, wenn die Datei veraltet ist. Danach zeigt `tsc`, wo die Oberfläche
   nicht mehr passt.
2. **Eine Befehlsliste.** `tests/ipc_contract.rs` liest `generate_handler![…]`
   aus `lib.rs` und die `call<…>("…")` aus `api.ts` und vergleicht.
3. **Echte Aufrufe ohne Fenster.** Derselbe Test ruft jeden Befehl über
   `tauri::test::MockRuntime` mit JSON-Argumenten auf — so, wie das Webview es
   tut. Ein falscher Argumentname fällt dort auf.
4. **Nur `api.ts` spricht mit Rust.** Der Schichtwächter verbietet
   `@tauri-apps/api/…` und `invoke(` überall sonst. Damit steht jeder Befehl
   genau einmal im Frontend, und Riegel 2 kann ihn finden.

### Warum `api.ts` Argumente einwortig hält

Tauri übersetzt Rust-Argumentnamen in camelCase: `source_id` erwartete im JSON
`sourceId`. Das ist dokumentiert, wird aber zuverlässig vergessen — von Menschen
und von Sprachmodellen. Einwortige Namen (`id`, `input`, `order`) machen die
Frage gegenstandslos.

### Warum eine eigene Fehlerhülle

`vizu_notion_core::Error` enthält `rusqlite::Error` und Pfade — nicht
serialisierbar und nicht für die Oberfläche gedacht. `ApiError` hat genau die
Form von `error` im JSON der Kommandozeile:

```json
{ "code": "validation_failed", "message": "title: darf nicht leer sein",
  "fields": [{ "field": "title", "message": "darf nicht leer sein" }] }
```

Damit behandeln beide Schalen Fehler auf dieselbe Art: mit `fields` ans Feld
(CLI: Rückgabewert 4), ohne `fields` als allgemeine Meldung (CLI: 1).

### Wenn der Start scheitert

Ist `config.toml` kaputt, kann `App::open` nicht öffnen. Statt dass die
Anwendung beim Doppelklick kommentarlos verschwindet, öffnet sich das Fenster
trotzdem: `AppState::failed` hält den Fehler, jeder Befehl liefert ihn, und
die Oberfläche zeigt ihn oben an.

## Die Oberfläche: App.tsx und Komponenten

React zeichnet bei jeder Zustandsänderung neu. Der häufigste Fehler in einer
wachsenden Oberfläche ist ein `invoke` in einem `useEffect` tief in einer
Komponente — dann weiß niemand mehr, wann was gespeichert wird.

Deshalb:

```
components/NoteEditor.tsx   Props rein: draft, fieldMessage, dark …
                            Rückrufe raus: onChange, onSave, onDelete …
App.tsx                     hält den Zustand, ruft api, verteilt Fehler
```

Komponenten *können* nicht speichern — sie importieren `api` nicht, der
Schichtwächter besteht darauf. Das Gegenstück zu `Model`/`Action` im
egui-Kit, nur in der Sprache von React.

## Vom Vorlagentext zum Bild

```
Vorlage (.mmd)  ──core::template──▶  Mermaid-Text  ──▶  ui/lib/mermaid.ts
   Handlebars + Notion-Daten             │              import("mermaid") (einmal)
                                         │              parse → render → SVG
                                         └──▶  vizu-notion render (stdout)
```

* **Den Text erzeugt `core`, nicht die Oberfläche.** Deshalb liefert
  `vizu-notion render` genau das, was im Fenster steht.
* **Mermaid wird nachgeladen.** Vite legt es als eigene Stücke ins Bündel.
  Wer nur Fluss und Metro-Karte ansieht, lädt es nie; die Anwendung startet
  ohne Wartezeit und ohne Netz.
* **Fluss und Metro-Karte gehen ohne Mermaid.** Ihre Koordinaten kommen aus
  `core::flow` und `core::metro`, gezeichnet wird eigenes SVG.
* **Wird aus Benutzertext je HTML, dann nur über `ui/src/lib/markdown.ts`.**
  Dort bereinigt DOMPurify. Im Webview hätte ein eingeschleustes Skript
  Zugriff auf jeden IPC-Befehl — und damit auf die Daten jeder Quelle.
* **Veraltete Darstellungen werden verworfen.** Wer tippt, erzeugt alle
  150 ms eine neue; eine langsame alte darf eine schnelle neue nicht
  überschreiben (`isCurrent` in `lib/mermaid.ts`).

## Daten

* **SQLite eingebettet** (`rusqlite` mit `bundled`). Kein Serverprozess.
* **WAL**, damit Desktop-Anwendung und Kommandozeile gleichzeitig laufen.
* **`STRICT`-Tabellen.**
* **Migrationen als nummerierte `.sql`-Dateien**, per `include_str!` im Binary.
* **UUIDv7 als Schlüssel** — siehe CLAUDE.md, „Kurzkennungen".
* **Kein SQL aus dem Webview.** `tauri-plugin-sql` wäre bequem und würde die
  Prüfung in `core` umgehen.

## Warum Tauri

| Statt | Grund gegen |
|---|---|
| egui (das Schwester-Kit) | Kein Zugang zu Browserpaketen wie mermaid, KaTeX, Chart.js, Monaco |
| Electron | Liefert Chromium und Node mit — 150 MB je Anwendung, zweite Laufzeit |
| Wails (Go) | Zweite Sprache neben der vorhandenen Rust-Fachlogik |
| Eine lokale Webanwendung im Browser | Kein Dock-Symbol, kein Dateisystem ohne Server, Port-Konflikte |

Der Preis ist ehrlich zu nennen:

* **Zwei Sprachen, zwei Builds.** npm und Cargo, Biome und Clippy.
* **Zwei Webviews.** WebKit auf macOS, WebKitGTK auf Linux. Fast immer
  gleich, aber nicht garantiert — auf Linux einmal ansehen, bevor etwas
  ausgeliefert wird. Chrome-only-APIs gibt es nicht.
* **Linux braucht `libwebkit2gtk-4.1`** auf dem Zielrechner. Das `.deb`
  zieht es als Abhängigkeit, das `.AppImage` bringt es mit.
* **Nicht nativ.** Die Oberfläche sieht aus wie eine gute Webseite, nicht wie
  AppKit. Für Werkzeuge der richtige Tausch.
