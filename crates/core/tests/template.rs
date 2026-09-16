//! Die Vorlagen-Engine gegen die Referenz der Webapp.
//!
//! Jeder Ordner in `tests/fixtures/templates/` ist ein Fall; `expected.mmd`
//! hat die Original-Logik von vizu-notion-local erzeugt (handlebars.js 4.7,
//! gray-matter 4.0). Weicht hier etwas ab, ändern sich Knotenkennungen in
//! Diagrammen, die es schon gibt — siehe README dort und docs/UMSETZUNG.md.

mod common;

use std::collections::{BTreeMap, HashSet};

use serde_json::Value;
use vizu_notion_core::rows::{Context, Row};
use vizu_notion_core::source::{self, ColumnMapping, Source, SourceInput};
use vizu_notion_core::template::{self, TemplateInput};
use vizu_notion_core::{App, ErrorCode};

fn fixtures() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/templates")
}

fn read(relative: &str) -> String {
    let path = fixtures().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Die Zeilen eines Falls, so wie die Webapp sie ihrem Handlebars gegeben hat.
fn context_of(case: &str) -> Context {
    let sources: BTreeMap<String, String> =
        serde_json::from_str(&read(&format!("{case}/sources.json"))).unwrap();
    sources
        .into_iter()
        .map(|(name, file)| {
            let rows: Vec<Row> = serde_json::from_str(&read(&file)).unwrap();
            (name, rows)
        })
        .collect()
}

fn render(case: &str) -> Result<String, String> {
    let body = read(&format!("{case}/template.mmd"));
    template::render_rows(&body, &context_of(case)).map_err(|e| e.to_string())
}

/// Schreibt Zeilen in den Zwischenspeicher — als wären sie abgerufen.
fn insert_rows(app: &App, source: &Source, rows: &[Row]) {
    for (position, row) in rows.iter().enumerate() {
        let properties: serde_json::Map<String, Value> = row
            .iter()
            .filter(|(k, _)| *k != "id")
            .map(|(k, v)| {
                (
                    // Die Spalte heißt in Notion so, wie die Quelle es sagt.
                    source.property(k).unwrap_or(k).to_string(),
                    serde_json::json!({ "type": "rich_text", "rich_text": [{ "plain_text": v }] }),
                )
            })
            .collect();
        app.conn()
            .execute(
                "INSERT INTO pages (source_id, page_id, position, title, url, last_edited_time, properties) \
                 VALUES (?1, ?2, ?3, ?4, '', '', ?5)",
                rusqlite::params![
                    source.id.to_string(),
                    row["id"],
                    position as i64,
                    row.get("title").cloned().unwrap_or_default(),
                    Value::Object(properties).to_string(),
                ],
            )
            .unwrap();
    }
}

/// Die Fälle, die durchlaufen müssen — Beschreibung in der README daneben.
const CASES: [&str; 9] = [
    "basic",
    "crlf",
    "edge",
    "escape",
    "example-fixed",
    "group",
    "hyphen",
    "join",
    "lookup",
];

#[test]
fn trifft_die_referenz_der_webapp_bytegleich() {
    let mut abweichungen = Vec::new();
    for case in CASES {
        let expected = read(&format!("{case}/expected.mmd"));
        match render(case) {
            Ok(actual) if actual == expected => {}
            Ok(actual) => abweichungen.push(format!(
                "--- {case}\nerwartet:\n{expected}\nbekommen:\n{actual}"
            )),
            Err(e) => abweichungen.push(format!("--- {case}\nFehler: {e}")),
        }
    }
    assert!(
        abweichungen.is_empty(),
        "Die Ausgabe weicht von der Webapp ab:\n\n{}",
        abweichungen.join("\n\n")
    );
}

#[test]
fn stile_erzeugen_classdef_und_class_zeilen() {
    // Eigener Fall, weil er als einziger beide Sorten Stil kennt: allgemein
    // (`parent`) und je Quelle (`Ziele.title`).
    let actual = render("styles").unwrap();
    assert_eq!(actual, read("styles/expected.mmd"));
    assert!(actual.contains("classDef cls_Ziele_title fill:#4e79a7,stroke:#2d5a8e"));
    assert!(actual.contains("class ngf8yfq,n3gzlx5,nj669fj cls_Ziele_title"));
}

#[test]
fn der_kopf_zaehlt_nur_am_dateianfang() {
    // `config/mermaid.example` der Webapp beginnt mit einem Kommentar, das
    // Frontmatter gilt dann nicht — die Webapp lehnt die Datei ebenfalls ab.
    let err = render("example").unwrap_err();
    assert!(err.contains("kein Kopf"), "{err}");
    assert!(read("example/expected.mmd").starts_with("FEHLER:"));
}

// --- Speichern und Prüfen ---------------------------------------------------

fn app_mit_quelle() -> App {
    let app = App::in_memory().unwrap();
    source::create(
        app.conn(),
        SourceInput::new(
            "Projekte",
            "396f66270f5d8034b55cebc685aa5e50",
            vec![ColumnMapping::new("title", "Name")],
        ),
    )
    .unwrap();
    app
}

const VORLAGE: &str = "---\ntitle: \"Übersicht\"\nsources:\n  - Projekte\n---\nflowchart TD\n{{#each Projekte}}\n  {{title}}\n{{/each}}\n";

#[test]
fn speichert_eine_vorlage_mit_titel_und_quellen_aus_dem_kopf() {
    let app = app_mit_quelle();
    let created = template::create(app.conn(), TemplateInput::new("uebersicht", VORLAGE)).unwrap();

    assert_eq!(created.slug, "uebersicht");
    assert_eq!(created.title, "Übersicht");
    assert_eq!(created.sources, vec!["Projekte"]);
    assert_eq!(template::list(app.conn()).unwrap(), vec![created.clone()]);

    let id = template::resolve(app.conn(), "UEBERSICHT").unwrap();
    assert_eq!(id, created.id);
}

#[test]
fn lehnt_eine_vorlage_ohne_kopf_ab() {
    let app = app_mit_quelle();
    let err = template::create(
        app.conn(),
        TemplateInput::new("kaputt", "flowchart TD\n  A --> B\n"),
    )
    .unwrap_err();
    assert_eq!(err.fields().unwrap()[0].field, "body");
    assert!(template::list(app.conn()).unwrap().is_empty());
}

#[test]
fn meldet_kurznamen_und_kopf_auf_einmal() {
    let app = app_mit_quelle();
    let err =
        template::create(app.conn(), TemplateInput::new("mit leer", "kein Kopf")).unwrap_err();
    let fields: Vec<&str> = err
        .fields()
        .unwrap()
        .iter()
        .map(|f| f.field.as_str())
        .collect();
    assert_eq!(fields, ["slug", "body"]);
}

#[test]
fn eine_unbekannte_quelle_faellt_beim_speichern_auf_nicht_erst_beim_zeichnen() {
    let app = app_mit_quelle();
    let body = VORLAGE.replace("Projekte", "Gibtsnicht");
    let err = template::create(app.conn(), TemplateInput::new("falsch", body)).unwrap_err();

    assert_eq!(err.code(), ErrorCode::ValidationFailed);
    assert!(err.to_string().contains("vorhanden: Projekte"), "{err}");
}

#[test]
fn import_ersetzt_eine_vorlage_mit_demselben_kurznamen() {
    let app = app_mit_quelle();
    let first = template::import_mmd(app.conn(), "uebersicht", VORLAGE).unwrap();
    let second = template::import_mmd(
        app.conn(),
        "uebersicht",
        &VORLAGE.replace("Übersicht", "Zweiter Titel"),
    )
    .unwrap();

    assert_eq!(first.id, second.id, "dieselbe Vorlage");
    assert_eq!(second.title, "Zweiter Titel");
    assert_eq!(template::list(app.conn()).unwrap().len(), 1);
}

#[test]
fn zeichnet_eine_gespeicherte_vorlage_aus_dem_zwischenspeicher() {
    let app = app_mit_quelle();
    let source = source::get(app.conn(), source::resolve(app.conn(), "Projekte").unwrap()).unwrap();
    let rows: Vec<Row> = ["Website", "App"]
        .iter()
        .enumerate()
        .map(|(i, title)| {
            Row::from([
                ("id".to_string(), format!("p{i}")),
                ("title".to_string(), (*title).to_string()),
            ])
        })
        .collect();
    insert_rows(&app, &source, &rows);

    let template = template::create(app.conn(), TemplateInput::new("uebersicht", VORLAGE)).unwrap();
    let diagram = template::render(app.conn(), &template, &HashSet::new()).unwrap();

    assert_eq!(diagram.title, "Übersicht");
    assert!(diagram.mermaid.starts_with("flowchart TD"));
    assert!(
        diagram.mermaid.contains(r#"["Website"]"#),
        "{}",
        diagram.mermaid
    );
    assert_eq!(diagram.nodes.len(), 2);
    assert_eq!(diagram.nodes[0].source, "Projekte");
}

#[test]
fn ausgeblendete_knoten_fehlen_im_diagramm_aber_nicht_in_der_liste() {
    let app = app_mit_quelle();
    let source = source::get(app.conn(), source::resolve(app.conn(), "Projekte").unwrap()).unwrap();
    insert_rows(
        &app,
        &source,
        &[
            Row::from([
                ("id".into(), "p0".into()),
                ("title".into(), "Website".into()),
            ]),
            Row::from([("id".into(), "p1".into()), ("title".into(), "App".into())]),
        ],
    );
    let template = template::create(app.conn(), TemplateInput::new("uebersicht", VORLAGE)).unwrap();

    let alle = template::render(app.conn(), &template, &HashSet::new()).unwrap();
    let versteckt: HashSet<String> = [alle.nodes[0].id.clone()].into_iter().collect();
    let gefiltert = template::render(app.conn(), &template, &versteckt).unwrap();

    assert!(alle.mermaid.contains("Website"));
    assert!(!gefiltert.mermaid.contains("Website"));
    assert_eq!(gefiltert.nodes.len(), 2, "die Liste zeigt weiterhin beide");
}
