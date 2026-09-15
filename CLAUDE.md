# Regeln für dieses Projekt

Diese Datei ist für KI-Assistenten gedacht (Claude Code, Cursor, …) und
gleichzeitig als Kurzreferenz für Menschen brauchbar. Sie beschreibt, was in
diesem Projekt gilt und wo die Fallstricke liegen.

---

## Der Stack in einem Satz

Rust mit drei Kisten in einem Arbeitsbereich: `core` (Fachlogik, SQLite
eingebettet), `cli` (clap) und `gui` (egui/eframe). Ein Binary je Schale, keine
Systemabhängigkeit, kein Datenbankserver. **Kein Node, kein Webview, kein
Electron, kein Tauri.**

---

## Festgenagelte Versionen — bitte nicht „aktualisieren"

Alle Versionen stehen in `[workspace.dependencies]` der obersten `Cargo.toml`.
Nur dort. Eine Kiste schreibt `foo.workspace = true`, nie eine eigene Nummer.

| Paket | Version | Grund |
|---|---|---|
| **egui / eframe / egui_kittest** | **0.36.2** | Müssen im Gleichschritt bleiben — `egui_kittest` bindet an genau diese egui-Fassung. Zwischen 0.3x-Ausgaben gibt es Umbenennungen, siehe den nächsten Abschnitt. |
| **rusqlite** | **0.40** mit `bundled` | `bundled` kompiliert SQLite mit ins Binary. **Nicht** auf die Systembibliothek umstellen: dann braucht die ausgelieferte Anwendung `libsqlite3` in passender Fassung auf dem fremden Rechner. |
| **clap** | **4.6** mit `derive`, `env`, `string` | `env` für `--data-dir` aus der Umgebung, `string` weil `clap_mangen` einen zur Laufzeit gebauten Namen braucht. Ohne die beiden Merkmale kompiliert `crates/cli` nicht. |
| **Rust** | **1.98.1** | In `rust-toolchain.toml`. Ein Upgrade ist eine bewusste Entscheidung, damit CI und Arbeitsplatz nie auseinanderlaufen. |

Wer eine Version anheben will: erst `just dupes` prüfen, dann `just check`.

---

## egui 0.36 — vier Eigenheiten

Das ist der Teil, bei dem Sprachmodelle am zuverlässigsten danebenliegen: Die
allermeisten Beispiele im Netz stammen aus egui 0.2x bis 0.31, und dort hießen
die Dinge anders. Was aus dem Gedächtnis kommt, kompiliert hier oft nicht.

1. **`App::ui`, nicht `App::update`.** Die Zeichenfunktion bekommt ein `Ui`,
   keinen `Context`:
   ```rust
   impl eframe::App for Gui {
       fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) { … }   // ✅
       fn update(&mut self, ctx: &egui::Context, …) { … }                      // ❌ existiert nicht
   }
   ```
   Wer vor dem Zeichnen etwas erledigen muss, nimmt das voreingestellte
   `App::logic(&mut self, ctx, frame)`.

2. **`Panel`, nicht `SidePanel`/`TopBottomPanel`.** Die vier Panel-Typen sind zu
   einem zusammengefasst, und alle `show`-Methoden nehmen `&mut Ui`:
   ```rust
   egui::Panel::left("liste").default_size(260.0).show(ui, |ui| …);   // ✅
   egui::SidePanel::left("liste").default_width(260.0).show(ctx, …);  // ❌
   ```
   Die Breite heißt `default_size`, nicht `default_width`.

3. **Stile gibt es zweimal — hell und dunkel.** `ctx.style()` und
   `ctx.style_mut()` existieren nicht mehr; es gibt `style_of(theme)` und
   `style_mut_of(theme)`. Wer eine Einstellung nur für die gerade sichtbare
   Fassung setzt, bekommt beim Umschalten der Systemeinstellung das alte
   Aussehen zurück. Für Farben beim Zeichnen ist `ui.visuals()` richtig — das
   `Ui` weiß, in welcher Fassung es gerade zeichnet, der `Context` nicht.

4. **Schriftzeichen prüfen.** Die mitgelieferte Schrift kennt nicht jedes
   Symbol. `✕` (U+2715) wird als leeres Kästchen gezeichnet, `×` (U+00D7) nicht.
   Nach dem Einbau eines Symbols einmal hinsehen.

Im Zweifel nicht raten, sondern nachlesen — die Quelle liegt lokal:

```bash
grep -rn "pub fn show" ~/.cargo/registry/src/*/egui-0.36.2/src/containers/panel.rs
```

---

## Sprache im Code

* **Bezeichner englisch** — Typen, Funktionen, Felder, Tabellen und Spalten,
  Optionsnamen, Aktionen: `NoteInput::clean()`, `notes.updated_at`,
  `--data-dir`, `Action::SaveDraft`. So heißen sie auch in Rust, und so rät man
  sie.
* **Kommentare, Doku, Oberflächentexte und Testnamen deutsch.**
  `fn ein_titel_aus_leerzeichen_gilt_als_leer()`.

---

## Die eine wichtige Schichtregel

```
crates/core/       →  FACHLOGIK UND PRÜFUNG. Kennt WEDER clap NOCH egui NOCH stdout.
crates/cli/        →  dünn. Argumente parsen, core rufen, Ausgabe formatieren.
crates/gui/        →  dünn. Zustand zeichnen, Aktionen an core schicken.
crates/gui/views/  →  zeichnet nur. Sieht nicht einmal die Anwendung.
```

**`crates/core/tests/layering.rs` schlägt fehl**, sobald in `core` ein
`println!`, ein `clap`, ein `egui` oder ein `std::process::exit` auftaucht —
und ebenso, sobald eine Ansicht `starter_core::App`, `note::create` oder
`conn()` berührt. Den Code verschieben, nicht den Test lockern.

Der Grund ist nicht Sauberkeit: **Alles, was in `core` steht, gilt für die
Kommandozeile und die Oberfläche gleichermaßen.** Eine Prüfung, die in einer
Schale steht, fehlt der anderen. Genau das ist der Fehler, den dieses Kit
verhindern soll.

### Prüfung gehört in die Fachlogik

```rust
// ✅ in crates/core/src/note.rs
pub fn create(conn: &Connection, input: NoteInput) -> Result<Note> {
    let input = input.clean()?;   // trimmt und prüft, wirft Error::Validation
    …
}
```

`Validator` sammelt **alle** fehlerhaften Felder und bricht erst am Ende ab.
Wer beim ersten Fehler zurückkehrt, zwingt den Benutzer, dieselbe Maske
mehrfach abzuschicken.

---

## Zwei Sorten Fehler

| Sorte | Woran erkennbar | Wohin damit |
|---|---|---|
| Eingabefehler | `err.fields()` ist `Some(...)` | CLI: Rückgabewert 4 und `error.fields` im JSON. Oberfläche: Meldung **am Feld**, nicht oben am Fenster. |
| Alles andere | `err.fields()` ist `None` | CLI: Rückgabewert 1. Oberfläche: Meldung oben. |

Deshalb gibt `core` niemals `anyhow::Error` zurück — sonst lässt sich das nicht
mehr unterscheiden. `anyhow` ist für die Schalen.

Die Rückgabewerte der Kommandozeile stehen in `crates/cli/src/exit.rs` und in
`docs/CLI.md`. `2` vergibt clap selbst für einen falschen Aufruf; den bitte
nicht überschreiben.

---

## Der Weg durch die Oberfläche

Eine Ansicht **wünscht**, `app.rs` **tut**:

```
views::show(ui, &mut model)  →  Option<Action>  →  Gui::apply  →  starter_core
```

`ui()` läuft viele Male pro Sekunde. Ein `note::create(…)` in einer
Zeichenfunktion liefe genauso oft. Deshalb sehen die Ansichten nur `&mut Model`
— sie *können* gar nicht speichern.

Eine neue Möglichkeit in der Oberfläche heißt also:

1. Variante zu `Action` in `crates/gui/src/model.rs` hinzufügen,
2. in der Ansicht bei Klick zurückgeben,
3. in `Gui::dispatch` behandeln.

Fachliche Regeln kommen in keinen dieser drei Schritte. Die stehen in `core`.

---

## Kurzkennungen: hinten, nicht vorn

Die Kennungen sind UUIDv7. Die **beginnt mit dem Zeitstempel** — zwei Notizen
aus derselben Sekunde teilen sich die ersten zwölf Zeichen. Zufällig ist erst
das Ende.

Darum zeigt `starter note list` die **letzten** acht Zeichen, und
`note::resolve_id` sucht per Endstück (`LIKE '%…'`). Wer auf ein Präfix
umstellt, bekommt Kennungen, die alle gleich aussehen.

Ist ein Endstück nicht eindeutig, gibt es `Error::Ambiguous` — nie einen
zufälligen Treffer. Bei `rm` wäre „irgendeine davon" die falsche Antwort.

---

## Pfade, Zeit, Daten

* **Die Fallunterscheidung macOS/Linux steht nur in `crates/core/src/paths.rs`.**
  Wer anderswo `#[cfg(target_os = …)]` für einen Pfad schreibt, hat sich
  verlaufen.
* `STARTER_DATA_DIR` und `STARTER_CONFIG_DIR` überschreiben die Verzeichnisse.
  In Tests wird stattdessen `Paths::under(tempdir)` benutzt — Umgebungsvariablen
  gelten für den ganzen Prozess, und `cargo test` läuft nebenläufig.
* **Gespeichert wird in UTC**, als RFC-3339-Text. Umgerechnet wird erst bei der
  Anzeige (`timestamp::to_local`).
* **Eine veröffentlichte Migration wird nie geändert.** Sie ist auf fremden
  Rechnern schon gelaufen. Änderungen kommen als neue Datei mit der nächsten
  Nummer in `crates/core/migrations/`.
* Die Tabellen sind `STRICT`. Ohne das nimmt SQLite in einer `TEXT`-Spalte
  klaglos eine Zahl entgegen.

---

## macOS-Bündel: Groß- und Kleinschreibung

Das Dateisystem unterscheidet in der Voreinstellung **nicht** zwischen groß und
klein. Hieße die Oberfläche im Bündel `Starter`, wäre sie dieselbe Datei wie die
Kommandozeile `starter` — die zweite Kopie überschriebe die erste, und ein
Doppelklick startete die CLI.

Deshalb behalten beide Binaries im Bündel ihren gebauten Namen, und
`CFBundleExecutable` zeigt auf `starter-gui`. Der Anzeigename kommt aus
`CFBundleName`. Siehe `scripts/bundle-macos.sh`.

---

## Was hier NICHT benutzt wird

Bitte nicht „nachrüsten":

| Nicht | Stattdessen | Warum |
|---|---|---|
| Tauri, Electron, Webview | egui | Zweite Sprache, zweiter Build, auf Linux `webkit2gtk` als Systemabhängigkeit |
| iced, Slint, GTK4 | egui | iced bricht die API zwischen Ausgaben, Slint ist eine eigene Sprache mit dünner Datenlage, GTK4 ist auf macOS eine Zumutung |
| Ein Datenbankserver | SQLite `bundled` | Eine Desktop-Anwendung darf kein Docker voraussetzen |
| `cargo-bundle` | `scripts/bundle-macos.sh` | Halb verwaist, und es versteckt genau die Schritte, die man zum Signieren braucht |
| Auto-Update-Framework | Release auf GitHub | Braucht Signaturschlüssel und Serverinfrastruktur |
| `unsafe` | — | `unsafe_code = "forbid"` im Arbeitsbereich |
| Bildvergleichstests der Oberfläche | `egui_kittest` über den Barrierefreiheitsbaum | Bildvergleiche brauchen einen Grafiktreiber und schlagen bei jeder Schriftänderung fehl |

---

## Tests

| Wo | Was |
|---|---|
| `crates/core/tests/note.rs` | Die fachlichen Regeln. **Hier liegt der Schwerpunkt.** |
| `crates/core/tests/config.rs` | Einstellungen lesen, schreiben, ablehnen |
| `crates/core/tests/layering.rs` | Der Schichtwächter |
| `crates/cli/tests/cli.rs` | Ausgabeformat, Rückgabewerte, Verhalten ohne Terminal |
| `crates/gui/tests/gui.rs` | Durchklicken ohne Fenster |

**Eine Regel wird einmal geprüft — in `core`.** Die Schalenprüfungen halten
fest, dass richtig durchgereicht wird, nicht was richtig ist.

Ändert sich ein Ausgabeformat absichtlich, wird der Schnappschuss nicht von Hand
nachgezogen, sondern mit `just snapshots` (`cargo insta review`) angesehen und
angenommen.

Ein Bedienelement in der Oberfläche braucht einen Namen, sonst findet es weder
ein Bildschirmleser noch der Test. Für Symbolknöpfe gibt es
`widgets::labelled(response, "Löschen")`.

---

## Vor jedem Commit

```bash
just check
```

Das ist `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` und
`cargo deny` — genau das, was auch die CI ausführt. Warnungen sind Fehler;
sonst sammeln sie sich an.

---

## Eine neue Ressource anlegen

`note` ist die Vorlage. Der Weg von der Tabelle bis in beide Schalen:

1. `crates/core/migrations/000X_….sql` — neue Datei, nächste Nummer, `STRICT`.
2. `crates/core/src/db.rs` — die Datei in die `MIGRATIONS`-Liste eintragen.
3. `crates/core/src/note.rs` kopieren, umbenennen, Felder und Regeln anpassen.
4. `crates/core/src/lib.rs` — Modul veröffentlichen.
5. `crates/cli/src/args.rs` und `crates/cli/src/commands/` — Unterbefehl,
   **beide** Ausgabefassungen (Mensch und `--json`).
6. `crates/gui/src/model.rs` — `Action`-Varianten; `views/` — Ansicht;
   `app.rs` — `dispatch`.
7. Tests in `crates/core/tests/` zuerst, dann die Schalen.

Ausführlicher mit Codebeispielen: `docs/RECIPES.md`.
