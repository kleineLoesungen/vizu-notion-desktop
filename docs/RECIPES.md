# Rezepte

Konkrete Handgriffe mit Code zum Abschauen. `note` ist überall die Vorlage.

---

## Eine neue Ressource anlegen

Beispiel: `task` mit Titel, Erledigt-Kennzeichen und Fälligkeitsdatum.

### 1. Migration

`crates/core/migrations/0002_tasks.sql` — neue Datei, **nächste Nummer**, nie
eine bestehende ändern:

```sql
CREATE TABLE tasks (
    id         TEXT    NOT NULL PRIMARY KEY,
    title      TEXT    NOT NULL,
    done       INTEGER NOT NULL DEFAULT 0,
    due_on     TEXT,
    created_at TEXT    NOT NULL,
    updated_at TEXT    NOT NULL
) STRICT;

CREATE INDEX tasks_due_on ON tasks (due_on);
```

`STRICT` nicht weglassen. Ohne das nimmt SQLite in `due_on` klaglos eine Zahl
entgegen.

### 2. Migration eintragen

`crates/core/src/db.rs`:

```rust
static MIGRATIONS: LazyLock<Migrations<'static>> = LazyLock::new(|| {
    Migrations::new(vec![
        M::up(include_str!("../migrations/0001_init.sql")),
        M::up(include_str!("../migrations/0002_tasks.sql")),   // ← neu
    ])
});
```

Reihenfolge = Ausführungsreihenfolge. Der Test `migrationen_sind_gueltig` läuft
bei jedem `just check` mit.

### 3. Modul

`crates/core/src/note.rs` kopieren nach `task.rs`, umbenennen, Regeln anpassen:

```rust
impl TaskInput {
    pub fn clean(self) -> Result<TaskInput> {
        let title = self.title.trim().to_string();

        let mut v = Validator::new();
        v.require(!title.is_empty(), "title", "darf nicht leer sein");
        v.require(
            title.chars().count() <= TITLE_MAX,
            "title",
            format!("darf höchstens {TITLE_MAX} Zeichen lang sein"),
        );
        v.finish()?;      // sammelt ALLE Felder, bricht erst hier ab

        Ok(TaskInput { title, .. })
    }
}
```

Zeichen zählen, nicht Bytes: `"ä"` sind zwei Bytes. `crates/core/src/lib.rs`
ergänzen: `pub mod task;`.

### 4. Tests zuerst

`crates/core/tests/task.rs` — hier liegt der Schwerpunkt, nicht in den Schalen:

```rust
#[test]
fn ein_titel_aus_leerzeichen_gilt_als_leer() {
    let app = App::in_memory().unwrap();
    let err = task::create(app.conn(), TaskInput::new("   ")).unwrap_err();
    assert_eq!(err.fields().unwrap()[0].field, "title");
}
```

### 5. Kommandozeile

`crates/cli/src/args.rs`:

```rust
#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(subcommand)]
    Note(NoteCommand),
    #[command(subcommand)]
    Task(TaskCommand),      // ← neu
    …
}
```

`crates/cli/src/commands/task.rs` — dünn, **beide** Ausgabefassungen:

```rust
TaskCommand::Add(fields) => {
    let created = task::create(app.conn(), TaskInput::new(fields.title))?;
    out.done(format!("Angelegt: {}", short_id(created.id)), &created);
    Ok(())
}
```

`out.done` gibt für Menschen den Satz aus, für `--json` das Objekt.

### 6. Oberfläche

`crates/gui/src/model.rs` — Aktionen ergänzen:

```rust
pub enum Action {
    …
    ToggleTask(Uuid),
}
```

`crates/gui/src/views/tasks.rs` — nur zeichnen, Aktion zurückgeben:

```rust
if ui.checkbox(&mut done, &task.title).changed() {
    action = Some(Action::ToggleTask(task.id));
}
```

`crates/gui/src/app.rs` — anwenden:

```rust
Action::ToggleTask(id) => {
    task::toggle(self.core.conn(), id)?;
    self.dispatch(Action::Reload)
}
```

Fachliche Regeln in keinem dieser drei Schritte. Die stehen in `core`.

---

## Einen Befehl hinzufügen

```rust
// crates/cli/src/args.rs
pub enum NoteCommand {
    …
    /// Notizen nach einem Wort durchsuchen.
    Search {
        /// Suchwort.
        query: String,
    },
}
```

```rust
// crates/cli/src/commands/note.rs
NoteCommand::Search { query } => {
    let found = note::search(app.conn(), &query)?;
    out.notes(&found);
    Ok(())
}
```

Die Suche selbst gehört nach `core` — sonst findet die Oberfläche anders als die
Kommandozeile. Danach `just check`: `befehlsstruktur_ist_gueltig` prüft, dass
clap die Struktur annimmt.

---

## Ein Feld zu den Einstellungen hinzufügen

Vier Stellen, in dieser Reihenfolge:

```rust
// 1. crates/core/src/config.rs
pub struct Config {
    …
    pub font_size: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self { …, font_size: 14.0 }
    }
}

pub fn validate(&self) -> Result<()> {
    let mut v = Validator::new();
    …
    v.require(
        (10.0..=24.0).contains(&self.font_size),
        "font_size",
        "muss zwischen 10 und 24 liegen",
    );
    v.finish()
}
```

```rust
// 2. crates/cli/src/commands/config.rs, fn apply
"font_size" => {
    config.font_size = value
        .parse()
        .map_err(|_| anyhow!("font_size erwartet eine Zahl, nicht {value:?}"))?
}
```

3. `crates/gui/src/views/settings.rs` — Eingabefeld.
4. `crates/gui/src/theme.rs` — anwenden.

Der Test `config_kennt_alle_felder` schlägt fehl, wenn Schritt 2 vergessen wird.

---

## Etwas im Hintergrund erledigen

Die Oberfläche darf nie blockieren. Eine lange Aufgabe läuft in einem Faden und
meldet sich über einen Kanal zurück:

```rust
// in Gui
rx: std::sync::mpsc::Receiver<Meldung>,

// Aufgabe starten
let tx = self.tx.clone();
let ctx = ui.ctx().clone();
std::thread::spawn(move || {
    let ergebnis = etwas_langes();
    let _ = tx.send(Meldung::Fertig(ergebnis));
    // OHNE DIESE ZEILE passiert nichts, bis der Benutzer die Maus bewegt:
    // egui zeichnet nur bei Bedarf neu.
    ctx.request_repaint();
});

// in draw(), vor dem Zeichnen
while let Ok(meldung) = self.rx.try_recv() {
    self.apply(Action::from(meldung));
}
```

`try_recv`, nicht `recv` — `recv` würde die Oberfläche anhalten.

`ctx.request_repaint()` ist die Zeile, die am häufigsten fehlt. Der Fehler sieht
aus wie „das Ergebnis kommt nicht an", ist aber nur ein ausgebliebenes Neuzeichnen.

---

## Ein Ausgabeformat ändern

Nicht den Schnappschuss von Hand nachziehen:

```bash
just snapshots     # cargo insta review
```

`a` nimmt an, `r` verwirft, `d` zeigt den Unterschied. Die Datei unter
`crates/cli/tests/snapshots/` kommt mit in den Commit — sie ist die Zusage an
die Benutzer, wie die Ausgabe aussieht.

Kennungen und Zeitstempel werden vor dem Vergleich ersetzt (siehe `FILTERS` in
`crates/cli/tests/cli.rs`), sonst schlüge jeder Lauf fehl.

---

## Eine egui-API nachschlagen

Nicht raten — die Quelle liegt lokal:

```bash
R=$(echo ~/.cargo/registry/src/*/egui-0.36.2)
grep -rn "pub fn show" $R/src/containers/panel.rs
grep -rn "pub fn set_theme\|pub fn style_of" $R/src/context.rs
grep -rn "pub fn text_edit_singleline" $R/src/ui.rs
```

Der häufigste Grund für Code, der nicht kompiliert: Die Beispiele im Netz
stammen aus einer älteren Ausgabe. Siehe CLAUDE.md, Abschnitt „egui 0.36 — vier
Eigenheiten".

---

## „database is locked"

Sollte durch WAL und `busy_timeout` nicht vorkommen. Wenn doch:

* Läuft noch eine zweite Ausgabe der Anwendung? `pgrep -l starter`
* Liegt das Datenverzeichnis auf einem Netzlaufwerk? SQLite und NFS vertragen
  sich nicht. Dann `STARTER_DATA_DIR` auf eine lokale Platte legen.
* Eine Schreiboperation, die eine offene Leseoperation überdauert? Ergebnisse
  von `query_map` vor dem Schreiben mit `collect()` einsammeln.

---

## Auf einem Wegwerfverzeichnis arbeiten

Ohne die echten Daten anzufassen:

```bash
just sandbox note add "Versuch"
just sandbox note list
rm -rf .local
```

In Tests **nicht** über Umgebungsvariablen, sondern:

```rust
let dir = tempfile::tempdir().unwrap();
let app = App::open(Paths::under(dir.path())).unwrap();
```

`cargo test` läuft nebenläufig, und `std::env::set_var` gilt für den ganzen
Prozess — zwei Tests würden sich das Verzeichnis gegenseitig umstellen.

Für reine Fachlogik genügt `App::in_memory()`: keine Datei, kein Aufräumen.
