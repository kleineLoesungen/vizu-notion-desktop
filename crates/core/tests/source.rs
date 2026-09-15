//! Fachliche Regeln der Quellen.

use vizu_notion_core::source::{self, ColumnMapping, SourceInput};
use vizu_notion_core::{App, ErrorCode};

const DB: &str = "396f66270f5d8034b55cebc685aa5e50";
const DB_DASHED: &str = "396f6627-0f5d-8034-b55c-ebc685aa5e50";

fn app() -> App {
    App::in_memory().expect("Datenbank im Arbeitsspeicher")
}

fn input(name: &str) -> SourceInput {
    SourceInput::new(
        name,
        DB,
        vec![
            ColumnMapping::new("title", "Name"),
            ColumnMapping::new("next", "Nächstes"),
        ],
    )
}

fn fields(err: &vizu_notion_core::Error) -> Vec<String> {
    err.fields()
        .expect("Eingabefehler")
        .iter()
        .map(|f| f.field.clone())
        .collect()
}

#[test]
fn legt_eine_quelle_an_und_findet_sie_wieder() {
    let app = app();
    let created = source::create(app.conn(), input("Projekte")).unwrap();

    assert_eq!(created.name, "Projekte");
    assert_eq!(created.database_id, DB_DASHED);
    assert_eq!(created.property("title"), Some("Name"));
    assert_eq!(source::get(app.conn(), created.id).unwrap(), created);
    assert_eq!(source::list(app.conn()).unwrap(), vec![created]);
}

#[test]
fn findet_ueber_namen_kennung_und_endstueck() {
    let app = app();
    let created = source::create(app.conn(), input("Projekte")).unwrap();
    let full = created.id.to_string();

    for needle in [
        "Projekte",
        "projekte",
        " PROJEKTE ",
        &full,
        &full[full.len() - 8..],
    ] {
        assert_eq!(
            source::resolve(app.conn(), needle).unwrap(),
            created.id,
            "{needle}"
        );
    }
    let err = source::resolve(app.conn(), "Aufgaben").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotFound);
}

#[test]
fn nimmt_die_adresse_der_datenbank_statt_der_kennung() {
    let app = app();
    let mut i = input("Projekte");
    i.database_id =
        format!("https://www.notion.so/team/Projekte-{DB}?v=0123456789abcdef0123456789abcdef");
    let created = source::create(app.conn(), i).unwrap();
    assert_eq!(created.database_id, DB_DASHED);
}

#[test]
fn raeumt_auf_und_sortiert_die_zuordnung() {
    let app = app();
    let created = source::create(
        app.conn(),
        SourceInput::new(
            "  Projekte ",
            DB,
            vec![
                ColumnMapping::new(" title ", " Name "),
                ColumnMapping::new("date", "Start"),
            ],
        ),
    )
    .unwrap();
    assert_eq!(created.name, "Projekte");
    assert_eq!(
        created.mappings,
        vec![
            ColumnMapping::new("date", "Start"),
            ColumnMapping::new("title", "Name")
        ]
    );
}

#[test]
fn meldet_alle_fehlerhaften_felder_auf_einmal() {
    let app = app();
    let err = source::create(
        app.conn(),
        SourceInput::new(
            " ",
            "keine-kennung",
            vec![
                ColumnMapping::new("next-step", "Nächstes"),
                ColumnMapping::new("title", ""),
                ColumnMapping::new("id", "ID"),
            ],
        ),
    )
    .unwrap_err();
    assert_eq!(
        fields(&err),
        [
            "name",
            "database_id",
            "mappings",
            "mappings.title",
            "mappings.id"
        ]
    );
}

#[test]
fn eine_rolle_darf_nur_einmal_vorkommen() {
    let app = app();
    let mut i = input("Projekte");
    i.mappings.push(ColumnMapping::new("title", "Titel"));
    let err = source::create(app.conn(), i).unwrap_err();
    assert_eq!(fields(&err), ["mappings.title"]);
}

#[test]
fn der_name_ist_ohne_gross_und_kleinschreibung_eindeutig() {
    let app = app();
    source::create(app.conn(), input("Projekte")).unwrap();
    let err = source::create(app.conn(), input("PROJEKTE")).unwrap_err();
    assert_eq!(fields(&err), ["name"]);
}

#[test]
fn umbenennen_in_den_eigenen_namen_ist_erlaubt() {
    let app = app();
    let created = source::create(app.conn(), input("Projekte")).unwrap();
    let updated = source::update(app.conn(), created.id, input("projekte")).unwrap();
    assert_eq!(updated.name, "projekte");
}

#[test]
fn aendern_ersetzt_die_ganze_zuordnung() {
    let app = app();
    let created = source::create(app.conn(), input("Projekte")).unwrap();
    let updated = source::update(
        app.conn(),
        created.id,
        SourceInput::new("Projekte", DB, vec![ColumnMapping::new("title", "Titel")]),
    )
    .unwrap();
    assert_eq!(updated.mappings, vec![ColumnMapping::new("title", "Titel")]);
    assert!(updated.updated_at >= created.updated_at);
}

#[test]
fn loeschen_und_danach_nicht_mehr_gefunden() {
    let app = app();
    let created = source::create(app.conn(), input("Projekte")).unwrap();
    source::delete(app.conn(), created.id).unwrap();
    assert_eq!(
        source::get(app.conn(), created.id).unwrap_err().code(),
        ErrorCode::NotFound
    );
    assert_eq!(
        source::delete(app.conn(), created.id).unwrap_err().code(),
        ErrorCode::NotFound
    );
    assert_eq!(source::count(app.conn()).unwrap(), 0);
}

// --- Import aus der Webapp -------------------------------------------------

#[test]
fn importiert_die_sources_json_der_webapp() {
    let app = app();
    let text = r#"{
      "sources": [
        { "databaseId": "396f66270f5d8034b55cebc685aa5e50", "name": "Project Milestones",
          "columnMappings": { "title": "Name", "date": "Due Date", "next": "Next Milestone" } },
        { "databaseId": "3dcf6627-0f5d-8038-86a1-f2481b0968ac", "name": "Task Flow",
          "columnMappings": { "title": "Task Name", "next": "Blocked By" } }
      ]
    }"#;
    let imported = source::import_webapp_json(app.conn(), text).unwrap();
    assert_eq!(imported.len(), 2);
    assert_eq!(imported[0].name, "Project Milestones");
    assert_eq!(imported[0].property("date"), Some("Due Date"));
    assert_eq!(
        imported[1].database_id,
        "3dcf6627-0f5d-8038-86a1-f2481b0968ac"
    );
}

#[test]
fn import_ist_alles_oder_nichts() {
    let app = app();
    let text = r#"{ "sources": [
        { "databaseId": "396f66270f5d8034b55cebc685aa5e50", "name": "Gut", "columnMappings": {} },
        { "databaseId": "kaputt", "name": "", "columnMappings": { "title": "" } }
    ] }"#;
    let err = source::import_webapp_json(app.conn(), text).unwrap_err();
    assert_eq!(
        fields(&err),
        [
            "sources[1].name",
            "sources[1].database_id",
            "sources[1].mappings.title"
        ]
    );
    assert_eq!(
        source::count(app.conn()).unwrap(),
        0,
        "auch „Gut“ wurde nicht angelegt"
    );
}

#[test]
fn import_lehnt_vorhandene_und_doppelte_namen_ab() {
    let app = app();
    source::create(app.conn(), input("Projekte")).unwrap();
    let text = r#"{ "sources": [
        { "databaseId": "396f66270f5d8034b55cebc685aa5e50", "name": "projekte", "columnMappings": {} },
        { "databaseId": "396f66270f5d8034b55cebc685aa5e50", "name": "Aufgaben", "columnMappings": {} },
        { "databaseId": "396f66270f5d8034b55cebc685aa5e50", "name": "AUFGABEN", "columnMappings": {} }
    ] }"#;
    let err = source::import_webapp_json(app.conn(), text).unwrap_err();
    assert_eq!(fields(&err), ["sources[0].name", "sources[2].name"]);
}

#[test]
fn import_meldet_unbekannte_felder_statt_sie_zu_ueberlesen() {
    // Die Webapp prüft mit additionalProperties: false — ein Tippfehler im
    // Schlüssel soll auffallen.
    let app = app();
    let text = r#"{ "sources": [ { "databaseID": "x", "name": "A", "columnMappings": {} } ] }"#;
    let err = source::import_webapp_json(app.conn(), text).unwrap_err();
    assert_eq!(fields(&err), ["file"]);
}
