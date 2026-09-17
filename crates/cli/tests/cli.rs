//! Abnahmetests der Kommandozeile.
//!
//! Geprüft wird, was ein Benutzer sieht: Ausgabe auf stdout, Meldung auf
//! stderr, Rückgabewert. Die fachlichen Regeln stehen in den Tests von
//! `vizu-notion-core` — hier geht es nur ums Durchreichen und ums Format.
//!
//! Kein Test hier spricht mit Notion oder mit dem Schlüsselbund: Der Token
//! liegt über `VIZU_NOTION_TOKEN_FILE` in einem Wegwerfverzeichnis, und
//! Abrufe gegen festgehaltene Antworten prüft `crates/core/tests/fetch.rs`.
//!
//! # Schnappschüsse
//!
//! Die erwartete Ausgabe steht in `tests/snapshots/`. Ändert sich das Format
//! absichtlich, wird nicht der Test von Hand nachgezogen, sondern:
//!
//! ```text
//! just snapshots        # cargo insta review — Änderung ansehen und annehmen
//! ```
//!
//! Kennungen, Zeitstempel und Pfade sind bei jedem Lauf andere. Sie werden vor
//! dem Vergleich ersetzt (siehe `FILTERS`).

use std::io::Write;
use std::process::{Command, Stdio};

use tempfile::TempDir;

const DB: &str = "396f66270f5d8034b55cebc685aa5e50";
const TOKEN: &str = "ntn_1234567890abcdefghijklmnopqrstuv";

/// Was in jedem Schnappschuss vereinheitlicht wird.
const FILTERS: &[(&str, &str)] = &[
    // Vollständige UUID zuerst, sonst frisst die Kurzform sie stückweise.
    (
        r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
        "[uuid]",
    ),
    (r"\b[0-9a-f]{8}\b", "[id]"),
    (r"\d{2}\.\d{2}\.\d{4} \d{2}:\d{2}", "[datum]"),
    (r"\d{4}-\d{2}-\d{2}T[0-9:.]+Z", "[zeit]"),
    // Wegwerfverzeichnisse heißen bei jedem Lauf anders.
    (r#"/[^\s"]*/vizu-notion-test-[^\s"]*"#, "[tmp]"),
];

struct Ctx {
    dir: TempDir,
}

impl Ctx {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("vizu-notion-test-")
            .tempdir()
            .unwrap();
        Self { dir }
    }

    fn command(&self, args: &[&str]) -> Command {
        // CARGO_BIN_EXE_* setzt cargo für Abnahmetests. Damit wird genau das
        // Binary getestet, das gerade gebaut wurde.
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_vizu-notion"));
        cmd.args(args)
            .arg("--data-dir")
            .arg(self.dir.path().join("data"))
            .arg("--config-dir")
            .arg(self.dir.path().join("config"))
            // Nie der echte Schlüsselbund, nie ein Token aus der Umgebung
            // dessen, der die Tests laufen lässt.
            .env("VIZU_NOTION_TOKEN_FILE", self.dir.path().join("token"))
            .env_remove("VIZU_NOTION_TOKEN");
        cmd
    }

    fn run(&self, args: &[&str]) -> Output {
        // Ohne stdin ist kein Terminal da — Rückfragen werden zum Fehler, wie
        // im Skript auch.
        let out = self
            .command(args)
            .stdin(Stdio::null())
            .output()
            .expect("vizu-notion startbar");
        Output::from(out)
    }

    fn run_with_stdin(&self, args: &[&str], input: &str) -> Output {
        let mut child = self
            .command(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        Output::from(child.wait_with_output().unwrap())
    }

    fn seed(&self) {
        self.run(&[
            "source",
            "add",
            "Projekte",
            "--database",
            DB,
            "--map",
            "title=Name",
            "--map",
            "next=Nächstes",
        ])
        .ok();
        self.run(&[
            "source",
            "add",
            "Aufgaben",
            "--database",
            DB,
            "--map",
            "title=Name",
        ])
        .ok();
    }
}

struct Output {
    stdout: String,
    stderr: String,
    code: i32,
}

impl From<std::process::Output> for Output {
    fn from(out: std::process::Output) -> Self {
        Self {
            stdout: String::from_utf8(out.stdout).expect("stdout ist UTF-8"),
            stderr: String::from_utf8(out.stderr).expect("stderr ist UTF-8"),
            code: out.status.code().expect("kein Signal"),
        }
    }
}

impl Output {
    /// Besteht auf Rückgabewert 0 und reicht die Ausgabe weiter.
    fn ok(self) -> Self {
        assert_eq!(
            self.code, 0,
            "erwartet Rückgabewert 0, bekam {}:\n{}",
            self.code, self.stderr
        );
        self
    }

    fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.stdout).unwrap_or_else(|e| panic!("{e}:\n{}", self.stdout))
    }
}

fn snapshot(name: &str, text: &str) {
    insta::with_settings!({filters => FILTERS.to_vec()}, {
        insta::assert_snapshot!(name, text);
    });
}

// --- Ausgabeformat ---------------------------------------------------------

#[test]
fn hilfe_nennt_alle_befehle() {
    let ctx = Ctx::new();
    snapshot("hilfe", &ctx.run(&["--help"]).ok().stdout);
}

#[test]
fn leere_liste_sagt_wie_es_weitergeht() {
    let ctx = Ctx::new();
    snapshot("liste_leer", &ctx.run(&["source", "list"]).ok().stdout);
}

#[test]
fn liste_ist_eine_tabelle() {
    let ctx = Ctx::new();
    ctx.seed();
    snapshot("liste", &ctx.run(&["source", "list"]).ok().stdout);
}

#[test]
fn liste_als_json_ist_ein_array_mit_abrufstand() {
    let ctx = Ctx::new();
    ctx.seed();
    let out = ctx.run(&["source", "list", "--json"]).ok();
    let list = out.json();
    assert_eq!(list.as_array().unwrap().len(), 2);
    assert_eq!(list[0]["source"]["name"], "Aufgaben");
    assert!(list[0]["fetch"].is_null(), "noch nie abgerufen");
    snapshot("liste_json", &out.stdout);
}

#[test]
fn einzelansicht_zeigt_zuordnung_und_volle_kennungen() {
    let ctx = Ctx::new();
    ctx.seed();
    snapshot(
        "show",
        &ctx.run(&["source", "show", "projekte"]).ok().stdout,
    );
}

// --- Quellen ---------------------------------------------------------------

#[test]
fn edit_haengt_rollen_um_und_laesst_den_rest_stehen() {
    let ctx = Ctx::new();
    ctx.seed();
    ctx.run(&[
        "source",
        "edit",
        "Projekte",
        "--map",
        "next=Nachfolger",
        "--map",
        "date=Start",
        "--unmap",
        "title",
    ])
    .ok();

    let shown = ctx
        .run(&["source", "show", "Projekte", "--json"])
        .ok()
        .json();
    assert_eq!(
        shown["source"]["mappings"],
        serde_json::json!([
            { "role": "date", "property": "Start" },
            { "role": "next", "property": "Nachfolger" }
        ])
    );
    assert_eq!(
        shown["source"]["database_id"],
        "396f6627-0f5d-8034-b55c-ebc685aa5e50"
    );
}

#[test]
fn import_liest_die_sources_json_der_webapp() {
    let ctx = Ctx::new();
    let file = ctx.dir.path().join("sources.json");
    std::fs::write(
        &file,
        format!(
            r#"{{ "sources": [
                {{ "databaseId": "{DB}", "name": "Project Milestones",
                   "columnMappings": {{ "title": "Name", "next": "Next Milestone" }} }}
            ] }}"#
        ),
    )
    .unwrap();

    let out = ctx.run(&["source", "import", file.to_str().unwrap()]).ok();
    assert!(
        out.stdout
            .contains("1 Quellen angelegt: Project Milestones"),
        "{}",
        out.stdout
    );

    let piped = ctx.run_with_stdin(
        &["source", "import", "-", "--json"],
        &format!(r#"{{ "sources": [ {{ "databaseId": "{DB}", "name": "Zweite", "columnMappings": {{}} }} ] }}"#),
    );
    assert_eq!(piped.code, 0, "{}", piped.stderr);
    assert_eq!(piped.json()[0]["name"], "Zweite");
}

#[test]
fn eine_spalte_ohne_gleichheitszeichen_ist_ein_falscher_aufruf() {
    let ctx = Ctx::new();
    let out = ctx.run(&["source", "add", "X", "--database", DB, "--map", "title"]);
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("ROLLE=SPALTE"), "{}", out.stderr);
}

#[test]
fn loeschen_ohne_terminal_verlangt_ausdrueckliche_bestaetigung() {
    let ctx = Ctx::new();
    ctx.seed();
    let out = ctx.run(&["source", "rm", "Projekte"]);
    assert_ne!(out.code, 0);
    assert!(out.stderr.contains("--yes"), "{}", out.stderr);
    ctx.run(&["source", "show", "Projekte"]).ok();

    ctx.run(&["source", "rm", "Projekte", "--yes"]).ok();
    assert_eq!(ctx.run(&["source", "show", "Projekte"]).code, 3);
}

// --- Vorlagen ---------------------------------------------------------------

const VORLAGE: &str = "---\ntitle: \"Übersicht\"\nsources:\n  - Projekte\n---\nflowchart TD\n{{#each Projekte}}\n  {{title}} --> {{next}}\n{{/each}}\n";

#[test]
fn liest_vorlagen_aus_dateien_und_zeichnet_sie() {
    let ctx = Ctx::new();
    ctx.seed();
    let file = ctx.dir.path().join("uebersicht.mmd");
    std::fs::write(&file, VORLAGE).unwrap();

    let imported = ctx
        .run(&["template", "import", file.to_str().unwrap()])
        .ok();
    assert!(
        imported.stdout.contains("uebersicht"),
        "{}",
        imported.stdout
    );

    snapshot("vorlagen", &ctx.run(&["template", "list"]).ok().stdout);

    // Noch nichts abgerufen: Das Diagramm besteht nur aus dem Kopf. Was mit
    // Daten herauskommt, prüfen die Referenzfälle in crates/core.
    let rendered = ctx.run(&["render", "uebersicht"]).ok();
    assert_eq!(rendered.stdout.trim(), "flowchart TD");

    let json = ctx.run(&["render", "uebersicht", "--json"]).ok().json();
    assert_eq!(json["title"], "Übersicht");
    assert_eq!(json["nodes"], serde_json::json!([]));
}

#[test]
fn ein_erneuter_import_ersetzt_die_vorlage() {
    let ctx = Ctx::new();
    ctx.seed();
    let file = ctx.dir.path().join("uebersicht.mmd");
    std::fs::write(&file, VORLAGE).unwrap();
    ctx.run(&["template", "import", file.to_str().unwrap()])
        .ok();
    std::fs::write(&file, VORLAGE.replace("Übersicht", "Neuer Titel")).unwrap();

    ctx.run(&["template", "import", file.to_str().unwrap()])
        .ok();

    let list = ctx.run(&["template", "list", "--json"]).ok().json();
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["title"], "Neuer Titel");
}

#[test]
fn ein_ganzes_verzeichnis_auf_einmal() {
    let ctx = Ctx::new();
    ctx.seed();
    let dir = ctx.dir.path().join("vorlagen");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join("eins.mmd"), VORLAGE).unwrap();
    std::fs::write(dir.join("zwei.mmd"), VORLAGE).unwrap();
    std::fs::write(dir.join("liesmich.txt"), "kein Diagramm").unwrap();

    ctx.run(&["template", "import", dir.to_str().unwrap()]).ok();

    let list = ctx.run(&["template", "list", "--json"]).ok().json();
    assert_eq!(list.as_array().unwrap().len(), 2, "die .txt bleibt liegen");
}

#[test]
fn eine_kaputte_vorlage_ergibt_vier_und_nennt_den_grund() {
    let ctx = Ctx::new();
    ctx.seed();
    let file = ctx.dir.path().join("kaputt.mmd");
    std::fs::write(&file, "flowchart TD\n  A --> B\n").unwrap();

    let out = ctx.run(&["template", "import", file.to_str().unwrap(), "--json"]);

    assert_eq!(out.code, 4);
    let err: serde_json::Value = serde_json::from_str(&out.stderr).unwrap();
    assert_eq!(err["error"]["fields"][0]["field"], "body");
    assert!(ctx.run(&["template", "list", "--json"]).ok().json()[0].is_null());
}

#[test]
fn eine_vorlage_ohne_passende_quelle_wird_nicht_gespeichert() {
    let ctx = Ctx::new();
    let file = ctx.dir.path().join("uebersicht.mmd");
    std::fs::write(&file, VORLAGE).unwrap();

    let out = ctx.run(&["template", "import", file.to_str().unwrap()]);

    assert_eq!(out.code, 4);
    assert!(out.stderr.contains("Projekte"), "{}", out.stderr);
}

#[test]
fn eine_quelle_ohne_next_kann_keinen_fluss() {
    let ctx = Ctx::new();
    ctx.seed();
    // „Aufgaben“ hat nur title.
    let out = ctx.run(&["flow", "Aufgaben", "--json"]);

    assert_eq!(out.code, 4);
    let err: serde_json::Value = serde_json::from_str(&out.stderr).unwrap();
    assert_eq!(err["error"]["fields"][0]["field"], "mappings.next");
}

#[test]
fn der_fluss_ohne_abruf_ist_leer_aber_kein_fehler() {
    let ctx = Ctx::new();
    ctx.seed();
    let out = ctx.run(&["flow", "Projekte"]).ok();
    assert!(out.stdout.contains("vizu-notion fetch"), "{}", out.stdout);

    let graph = ctx.run(&["flow", "Projekte", "--json"]).ok().json();
    assert_eq!(graph["nodes"], serde_json::json!([]));
    assert_eq!(graph["subtitle_roles"], serde_json::json!([]));
}

#[test]
fn eine_quelle_ohne_datum_kann_keine_metro_karte() {
    let ctx = Ctx::new();
    ctx.seed();
    // „Projekte“ hat next, aber kein date.
    let out = ctx.run(&["metro", "Projekte", "--json"]);

    assert_eq!(out.code, 4);
    let err: serde_json::Value = serde_json::from_str(&out.stderr).unwrap();
    assert_eq!(err["error"]["fields"][0]["field"], "mappings.date");
}

// --- Token -----------------------------------------------------------------

#[test]
fn token_kommt_von_stdin_und_wird_nie_ausgegeben() {
    let ctx = Ctx::new();
    let set = ctx.run_with_stdin(&["token", "set"], &format!("{TOKEN}\n"));
    assert_eq!(set.code, 0, "{}", set.stderr);
    assert!(set.stdout.contains("ntn_…stuv"), "{}", set.stdout);

    let status = ctx.run(&["token", "status"]).ok();
    let status_json = ctx.run(&["token", "status", "--json"]).ok();
    for text in [
        &set.stdout,
        &set.stderr,
        &status.stdout,
        &status_json.stdout,
    ] {
        assert!(!text.contains("1234567890"), "Token sichtbar:\n{text}");
    }
    assert_eq!(status_json.json()["origin"], "store");
    snapshot("token_status", &status.stdout);

    ctx.run(&["token", "clear"]).ok();
    assert!(ctx.run(&["token", "status", "--json"]).ok().json()["origin"].is_null());
}

#[test]
fn ein_offensichtlich_falscher_token_wird_nicht_gespeichert() {
    let ctx = Ctx::new();
    let out = ctx.run_with_stdin(&["token", "set", "--json"], "kurz");
    assert_eq!(out.code, 4);
    let err: serde_json::Value = serde_json::from_str(&out.stderr).unwrap();
    assert_eq!(err["error"]["fields"][0]["field"], "token");
    assert!(ctx.run(&["token", "status", "--json"]).ok().json()["origin"].is_null());
}

// --- Abruf -----------------------------------------------------------------

#[test]
fn abruf_ohne_quellen_ist_kein_fehler() {
    let ctx = Ctx::new();
    let out = ctx.run(&["fetch"]).ok();
    assert!(out.stdout.contains("source add"), "{}", out.stdout);
    assert_eq!(
        ctx.run(&["fetch", "--json"]).ok().json(),
        serde_json::json!([])
    );
}

#[test]
fn abruf_ohne_token_ergibt_sieben_und_sagt_wie_es_geht() {
    let ctx = Ctx::new();
    ctx.seed();
    let out = ctx.run(&["fetch", "Projekte", "--json"]);
    assert_eq!(out.code, 7, "{}", out.stderr);
    snapshot("fetch_ohne_token_json", &out.stderr);
}

// --- Rückgabewerte ---------------------------------------------------------

#[test]
fn ungueltige_eingabe_ergibt_vier_mit_allen_feldern() {
    let ctx = Ctx::new();
    let out = ctx.run(&[
        "source",
        "add",
        " ",
        "--database",
        "keine",
        "--map",
        "title=",
    ]);
    assert_eq!(out.code, 4);
    assert!(out.stdout.is_empty(), "Fehler gehören nach stderr");
    snapshot("fehler_eingabe", &out.stderr);
}

#[test]
fn eingabefehler_als_json_nennt_die_felder() {
    let ctx = Ctx::new();
    let out = ctx.run(&["source", "add", " ", "--database", "keine", "--json"]);
    assert_eq!(out.code, 4);
    snapshot("fehler_eingabe_json", &out.stderr);
}

#[test]
fn unbekannte_quelle_ergibt_drei() {
    let ctx = Ctx::new();
    assert_eq!(ctx.run(&["source", "show", "Gibtsnicht"]).code, 3);
    assert_eq!(ctx.run(&["fetch", "Gibtsnicht"]).code, 3);
}

#[test]
fn falscher_aufruf_ergibt_zwei() {
    // Den Rückgabewert 2 vergibt clap selbst. Der Test hält fest, dass wir ihn
    // nicht versehentlich überschreiben.
    let ctx = Ctx::new();
    assert_eq!(ctx.run(&["source", "gibtsnicht"]).code, 2);
}

#[test]
fn mehrdeutige_kennung_ergibt_fuenf_und_zeigt_die_treffer() {
    let ctx = Ctx::new();

    // Ein Hexzeichen hat 16 mögliche Werte. Bei 17 Quellen müssen sich nach
    // dem Schubfachprinzip zwei das letzte Zeichen teilen — damit ist der
    // mehrdeutige Fall garantiert und der Test nicht vom Zufall abhängig.
    for i in 0..17 {
        ctx.run(&["source", "add", &format!("Quelle {i}"), "--database", DB])
            .ok();
    }

    let list = ctx.run(&["source", "list", "--json"]).ok().json();
    let mut nach_endzeichen: std::collections::HashMap<char, usize> = Default::default();
    for s in list.as_array().unwrap() {
        let last = s["source"]["id"]
            .as_str()
            .unwrap()
            .chars()
            .next_back()
            .unwrap();
        *nach_endzeichen.entry(last).or_default() += 1;
    }
    let (zeichen, _) = nach_endzeichen
        .iter()
        .find(|(_, n)| **n > 1)
        .expect("Schubfachprinzip garantiert eine Dopplung");

    let out = ctx.run(&["source", "show", &zeichen.to_string()]);
    assert_eq!(out.code, 5, "{}", out.stderr);
    assert!(
        out.stderr.contains("mehr Zeichen angeben"),
        "{}",
        out.stderr
    );
}

// --- Verhalten -------------------------------------------------------------

#[test]
fn einstellungen_ueberleben_den_programmstart() {
    let ctx = Ctx::new();
    ctx.run(&["config", "set", "theme", "dark"]).ok();

    let config = ctx.run(&["config", "show", "--json"]).ok().json();
    assert_eq!(config["theme"], "dark");
}

#[test]
fn eine_kaputte_einstellung_wird_nicht_uebernommen() {
    let ctx = Ctx::new();
    ctx.run(&["config", "set", "theme", "dark"]).ok();
    assert_eq!(ctx.run(&["config", "set", "accent", "blau"]).code, 4);

    let config = ctx.run(&["config", "show", "--json"]).ok().json();
    assert_eq!(config["accent"], "#3b6ea5", "alter Wert blieb stehen");
    assert_eq!(
        config["theme"], "dark",
        "die gültige Einstellung steht noch"
    );
}

#[test]
fn completions_brauchen_keine_datenbank() {
    // `vizu-notion completions zsh` muss auch dort laufen, wo das
    // Datenverzeichnis nicht beschreibbar ist — sonst scheitert die
    // Installation über ein Paket.
    let out = Command::new(env!("CARGO_BIN_EXE_vizu-notion"))
        .args(["completions", "zsh"])
        .arg("--data-dir")
        .arg("/nicht/beschreibbar")
        .arg("--config-dir")
        .arg("/nicht/beschreibbar")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("#compdef vizu-notion"));
}

// --- Gespeicherte Ansichten --------------------------------------------------

#[test]
fn speichert_eine_ansicht_und_zeichnet_sie_wieder() {
    let ctx = Ctx::new();
    ctx.seed();

    snapshot("ansichten_leer", &ctx.run(&["view", "list"]).ok().stdout);

    let saved = ctx
        .run(&[
            "view",
            "add",
            "Projekte ohne Altlasten",
            "--flow",
            "Projekte",
            "--hide",
            "p1,p2",
            "--sub",
            "status",
        ])
        .ok();
    assert!(saved.stdout.contains("Gespeichert"), "{}", saved.stdout);

    snapshot("ansichten", &ctx.run(&["view", "list"]).ok().stdout);

    // Dasselbe wie `flow Projekte` — nur mit dem, was die Ansicht festhält.
    let json = ctx
        .run(&["view", "show", "Projekte ohne Altlasten", "--json"])
        .ok()
        .json();
    assert_eq!(json["nodes"], serde_json::json!([]));
    assert!(json["subtitle_roles"].is_array(), "{json}");
}

#[test]
fn eine_ansicht_braucht_genau_eine_diagrammart() {
    let ctx = Ctx::new();
    ctx.seed();

    let ohne = ctx.run(&["view", "add", "Leer"]);
    assert_eq!(ohne.code, 1, "{}", ohne.stderr);
    assert!(ohne.stderr.contains("--template"), "{}", ohne.stderr);

    // clap lässt zwei Arten gar nicht erst durch.
    let zwei = ctx.run(&[
        "view", "add", "Zwei", "--flow", "Projekte", "--metro", "Projekte",
    ]);
    assert_eq!(zwei.code, 2, "{}", zwei.stderr);
}

#[test]
fn loescht_eine_ansicht_nur_nach_rueckfrage() {
    let ctx = Ctx::new();
    ctx.seed();
    ctx.run(&["view", "add", "Roadmap", "--metro", "Projekte"])
        .ok();

    // Ohne Terminal gibt es keine Rückfrage — also einen Fehler statt eines
    // stillen Löschens.
    let ohne = ctx.run(&["view", "rm", "Roadmap"]);
    assert_eq!(ohne.code, 1, "{}", ohne.stderr);
    assert!(ohne.stderr.contains("--yes"), "{}", ohne.stderr);

    let weg = ctx.run(&["view", "rm", "Roadmap", "--yes"]).ok();
    assert!(weg.stdout.contains("Gelöscht"), "{}", weg.stdout);
    assert!(
        ctx.run(&["view", "list"]).ok().stdout.contains("Keine"),
        "Ansicht ist noch da"
    );
}
