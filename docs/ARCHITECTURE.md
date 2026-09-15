# Aufbau

## Das Problem, das dieses Kit löst

Eine Anwendung mit Oberfläche **und** Kommandozeile hat eine typische
Verfallsgeschichte: Die Oberfläche entsteht zuerst, die Regeln landen in den
Zeichenfunktionen. Später kommt die Kommandozeile dazu und bekommt eine eigene,
leicht abweichende Fassung derselben Regeln. Von da an gibt es jede Prüfung
zweimal, und irgendwann erlaubt die eine, was die andere verbietet.

Dagegen hilft keine Ermahnung, sondern eine Anordnung, in der der Fehler nicht
kompiliert.

```
                        ┌──────────────────────┐
                        │   crates/core        │
    ┌──────────────────▶│   Fachlogik          │◀──────────────────┐
    │                   │   Prüfung            │                   │
    │                   │   SQLite             │                   │
    │                   └──────────────────────┘                   │
    │                                                              │
┌───┴────────────────┐                              ┌──────────────┴───┐
│  crates/cli        │                              │  crates/gui      │
│  clap              │                              │  egui / eframe   │
│  Mensch │ --json   │                              │  Model + Action  │
└────────────────────┘                              └──────────────────┘
```

Die Pfeile gehen nur in eine Richtung. `core` weiß nicht, dass es Schalen gibt.

## Was wo hingehört

| Frage | Antwort |
|---|---|
| Darf ein Titel leer sein? | `core` |
| Wird ein leerer Titel rot unterlegt oder als Meldung gezeigt? | `gui` |
| Welchen Rückgabewert liefert ein leerer Titel? | `cli` |
| Wie heißt die Spalte in der Datenbank? | `core` |
| Wie wird der Zeitstempel angezeigt? | die jeweilige Schale |
| Wie wird der Zeitstempel gespeichert? | `core` — immer UTC |

Die Faustregel: **Was auf beiden Wegen gleich sein muss, steht in `core`.**

## Der Wächter

`crates/core/tests/layering.rs` durchsucht den Quelltext nach Zeichenfolgen, die
an der jeweiligen Stelle nichts zu suchen haben, und prüft zusätzlich die
Abhängigkeiten in den `Cargo.toml`-Dateien. Schlägt er an, sieht die Meldung so
aus:

```
Die Schichtregel aus CLAUDE.md ist verletzt:

  crates/gui/src/views/notes.rs:131
    enthält "starter_core::App" — eine Ansicht kennt die Anwendung nicht
    > fn verstoss(app: &starter_core::App) { let _ = app.conn(); }

Den Code verschieben, nicht diesen Test ändern.
```

Zwei Riegel, weil einer umgehbar wäre: Der Textscan lässt sich mit `use egui as
ui;` austricksen — ohne Eintrag in der `Cargo.toml` kompiliert davon aber
nichts.

Kommentare werden vor dem Scan abgeschnitten. Sonst könnte diese Datei die
Regel nicht beschreiben, ohne sie zu brechen.

## Die Oberfläche: Model und Action

Der Sofortmodus von egui ist ungewohnt: `ui()` läuft viele Male pro Sekunde,
und was darin steht, läuft genauso oft. Der häufigste Fehler ist ein
Schreibzugriff mitten im Zeichnen.

Deshalb gibt es zwei getrennte Typen:

```rust
pub struct Model { … }   // Zustand. Kennt weder Datenbank noch Dateisystem.
pub struct Gui {         // Model + starter_core::App
    core: starter_core::App,
    model: Model,
}
```

Die Ansichten in `views/` bekommen **nur** `&mut Model`. Sie *können* nichts
speichern — die Signatur gibt es nicht her. Was geschehen soll, geben sie als
`Action` zurück:

```
views::show(ui, &mut model)  →  Option<Action>  →  Gui::apply  →  starter_core
```

`Gui::apply` ist die einzige Stelle im ganzen Programm, an der `Model` und
`starter_core::App` gleichzeitig sichtbar sind.

Eine Ansicht gibt höchstens **eine** Aktion je Bild zurück. Mehr ist nicht
nötig — niemand klickt zweimal in derselben sechzehntel Sekunde — und eine
Liste würde die Reihenfolge zur offenen Frage machen.

## Die Kommandozeile: zwei Ausgaben, eine Logik

`crates/cli/src/output.rs` kapselt beide Fassungen. **Jeder Befehl kann beides**
— ein `--json`, das bei manchen Befehlen nichts tut, ist schlimmer als keins,
weil ein Skript es nicht vorhersehen kann.

Protokoll geht auf **stderr**, niemals auf stdout. Sonst stolpert `jq` über die
Protokollzeile.

## Fehler

```rust
pub enum Error {
    NotFound,
    Validation(ValidationError),   // hat Feldfehler
    Ambiguous { prefix, matches },
    Db(rusqlite::Error),
    Io { path, source },
    …
}
```

`Error::fields()` liefert die Feldfehler oder `None`. Damit muss keine Schale
`match` auf die Variante schreiben, und beide behandeln Eingabefehler anders als
Programmfehler — die Oberfläche schreibt sie ans Feld, die Kommandozeile gibt
Rückgabewert 4.

`core` gibt **nie** `anyhow::Error` zurück. Dann ginge die Unterscheidung
verloren. `anyhow` ist für die Schalen, wo sie nicht mehr gebraucht wird.

## Daten

* **SQLite eingebettet** (`rusqlite` mit `bundled`). Kein Serverprozess, keine
  Systembibliothek, ein Binary.
* **WAL**, damit Oberfläche und Kommandozeile gleichzeitig laufen können, ohne
  einander mit „database is locked" zu blockieren.
* **`STRICT`-Tabellen.** Ohne das nimmt SQLite in einer `TEXT`-Spalte klaglos
  eine Zahl entgegen.
* **Migrationen als nummerierte `.sql`-Dateien**, über `include_str!` ins Binary
  einkompiliert — eine ausgelieferte Desktop-Anwendung hat kein Verzeichnis, aus
  dem sie sie lesen könnte.
* **UUIDv7 als Schlüssel.** Trägt die Zeit im Präfix, ist damit aufsteigend und
  kommt SQLite beim Einfügen entgegen. Die Kehrseite: der Anfang ist nicht
  zufällig — siehe CLAUDE.md, Abschnitt „Kurzkennungen".

## Warum egui

| Statt | Grund gegen |
|---|---|
| Tauri, Electron | Zweite Sprache über IPC, zweiter Build, auf Linux `webkit2gtk` mit eigenen Renderfehlern |
| iced | Starke API-Brüche zwischen Ausgaben — Sprachmodelle erzeugen fast immer Code, der nicht kompiliert |
| Slint | Eigene Sprache mit dünner Trainingsdatenlage |
| GTK4 | Auf macOS eine Zumutung, fette Systemabhängigkeit |

Der Preis ist ehrlich zu nennen: **egui sieht nicht nach macOS aus.** Kein
natives Menü in der Menüleiste, eigenes Textfeldverhalten, eingeschränkte
Eingabemethoden für Sprachen mit IME. Für Werkzeuge und Übersichten ist das der
richtige Tausch gegen „ein Binary, keine Abhängigkeiten, eine Sprache". Für eine
Anwendung, die sich wie eine macOS-Anwendung anfühlen soll, ist es der falsche.
