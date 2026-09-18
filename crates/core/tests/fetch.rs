//! Abruf und Zwischenspeicher — gegen die festgehaltenen Notion-Antworten.

mod common;

use common::{FixtureNotion, Ids, client, count_paths, ok};
use serde_json::json;
use vizu_notion_core::notion::Method;
use vizu_notion_core::source::{self, ColumnMapping, Source, SourceInput};
use vizu_notion_core::{App, ErrorCode, fetch};

/// Was die Datenbank schon an Titeln kennt — spart Anfragen beim Abruf.
fn known(app: &App) -> std::collections::HashSet<String> {
    fetch::known_titles(app.conn()).unwrap()
}

fn app() -> App {
    App::in_memory().unwrap()
}

fn add(app: &App, label: &str, mappings: Vec<ColumnMapping>) -> Source {
    let ids = Ids::load();
    source::create(
        app.conn(),
        SourceInput::new(label, ids.database(label), mappings),
    )
    .unwrap()
}

#[test]
fn ruft_alle_seiten_ueber_mehrere_stapel_ab() {
    let app = app();
    let aufgaben = add(&app, "aufgaben", vec![ColumnMapping::new("title", "Name")]);
    let notion = FixtureNotion::new();
    let log = notion.log();
    let (client, _) = client(notion);

    let download = fetch::download(&client, &aufgaben, &known(&app)).unwrap();

    assert_eq!(download.pages.len(), 130);
    assert_eq!(download.database.title(), "vizu Aufgaben");
    // Datenbank, Schema, zwei Stapel — dazu je ein Blick auf die zwölf
    // Projekte, auf die die Aufgaben zeigen.
    assert_eq!(download.requests, 4 + 12);
    assert_eq!(download.titles.len(), 12);
    let queries = log
        .lock()
        .unwrap()
        .iter()
        .filter(|r| r.method == Method::Post)
        .count();
    assert_eq!(queries, 2);
}

#[test]
fn speichert_die_seiten_in_der_reihenfolge_der_abfrage() {
    let app = app();
    let projekte = add(&app, "projekte", vec![ColumnMapping::new("title", "Name")]);
    let (client, _) = client(FixtureNotion::new());
    let download = fetch::download(&client, &projekte, &known(&app)).unwrap();

    let status = fetch::store(app.conn(), &download).unwrap();

    assert_eq!(status.page_count, 12);
    assert_eq!(status.database_title, "vizu Projekte");
    assert!(status.database_url.starts_with("https://"));
    assert_eq!(
        fetch::pages(app.conn(), projekte.id).unwrap(),
        download.pages
    );
    assert_eq!(
        fetch::status(app.conn(), projekte.id).unwrap(),
        Some(status)
    );
}

#[test]
fn ein_neuer_abruf_ersetzt_den_alten_vollstaendig() {
    let app = app();
    let projekte = add(&app, "projekte", vec![]);
    let (client, _) = client(FixtureNotion::new());
    let mut download = fetch::download(&client, &projekte, &known(&app)).unwrap();
    fetch::store(app.conn(), &download).unwrap();

    download.pages.truncate(3);
    let status = fetch::store(app.conn(), &download).unwrap();

    assert_eq!(status.page_count, 3);
    assert_eq!(fetch::pages(app.conn(), projekte.id).unwrap().len(), 3);
}

#[test]
fn noch_nie_abgerufen_hat_keinen_stand() {
    let app = app();
    let projekte = add(&app, "projekte", vec![]);
    assert_eq!(fetch::status(app.conn(), projekte.id).unwrap(), None);
    assert!(fetch::pages(app.conn(), projekte.id).unwrap().is_empty());
}

#[test]
fn eine_unbekannte_spalte_ist_ein_eingabefehler_an_der_rolle() {
    let app = app();
    let projekte = add(
        &app,
        "projekte",
        vec![
            ColumnMapping::new("title", "Name"),
            ColumnMapping::new("next", "Nachfolger"),
            ColumnMapping::new("date", "Datum"),
        ],
    );
    let (client, _) = client(FixtureNotion::new());

    let err = fetch::download(&client, &projekte, &known(&app)).unwrap_err();

    let fields = err.fields().expect("Eingabefehler");
    let names: Vec<&str> = fields.iter().map(|f| f.field.as_str()).collect();
    assert_eq!(names, ["mappings.date", "mappings.next"]);
    // Die vorhandenen Spalten stehen in der Meldung — meist ist es ein Tippfehler.
    assert!(
        fields[1].message.contains("Nächstes"),
        "{}",
        fields[1].message
    );
}

#[test]
fn eine_nicht_geteilte_datenbank_sagt_das_auch() {
    let app = app();
    let fremd = source::create(
        app.conn(),
        SourceInput::new("Fremd", "ffffffffffffffffffffffffffffffff", vec![]),
    )
    .unwrap();
    let (client, _) = client(FixtureNotion::new());

    let err = fetch::download(&client, &fremd, &known(&app)).unwrap_err();

    assert_eq!(err.code(), ErrorCode::NotionNotShared);
    assert!(err.to_string().contains("geteilt"), "{err}");
}

#[test]
fn gekuerzte_relationen_werden_nachgeladen() {
    // Notion liefert im Seitenobjekt höchstens 25 Ziele einer Relation und
    // setzt dann has_more. Die Webapp hat das übersehen.
    let app = app();
    let projekte = add(&app, "projekte", vec![]);
    let ids = Ids::load();
    let ds = ids.data_source("projekte");
    let first_page = common::read_json("2025-09-03/projekte.query.1.json")["results"][0].clone();
    let page_id = first_page["id"].as_str().unwrap().to_string();
    let prop_id = first_page["properties"]["Ziel"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let query_path = format!("/data_sources/{ds}/query");
    let property_path = format!("/pages/{page_id}/properties/{prop_id}");
    let notion = FixtureNotion::new()
        .route(move |r| {
            (r.path == query_path).then(|| {
                let mut batch = common::read_json("2025-09-03/projekte.query.1.json");
                batch["results"][0]["properties"]["Ziel"]["has_more"] = json!(true);
                ok(batch)
            })
        })
        .route(move |r| {
            let path = r.path.split('?').next().unwrap();
            (path == property_path).then(|| {
                if r.path.contains("start_cursor=c2") {
                    ok(json!({ "results": [{ "relation": { "id": "z-3" } }], "has_more": false }))
                } else {
                    ok(json!({
                        "results": [{ "relation": { "id": "z-1" } }, { "relation": { "id": "z-2" } }],
                        "has_more": true, "next_cursor": "c2"
                    }))
                }
            })
        });
    let log = notion.log();
    let (client, _) = client(notion);

    let download = fetch::download(&client, &projekte, &known(&app)).unwrap();

    let ziel = &download.pages[0].properties["Ziel"];
    assert_eq!(ziel["has_more"], json!(false));
    assert_eq!(
        ziel["relation"],
        json!([{ "id": "z-1" }, { "id": "z-2" }, { "id": "z-3" }])
    );
    let counts = count_paths(&log);
    assert_eq!(
        counts.get(&format!("/pages/{page_id}/properties/{prop_id}")),
        Some(&2)
    );
}

#[test]
fn eine_waehrend_des_abrufs_geloeschte_quelle_wird_nicht_wiederbelebt() {
    let app = app();
    let projekte = add(&app, "projekte", vec![]);
    let (client, _) = client(FixtureNotion::new());
    let download = fetch::download(&client, &projekte, &known(&app)).unwrap();

    source::delete(app.conn(), projekte.id).unwrap();

    assert_eq!(
        fetch::store(app.conn(), &download).unwrap_err().code(),
        ErrorCode::NotFound
    );
}

#[test]
fn eine_andere_datenbank_verwirft_den_zwischenspeicher() {
    let app = app();
    let projekte = add(&app, "projekte", vec![]);
    let (client, _) = client(FixtureNotion::new());
    fetch::store(
        app.conn(),
        &fetch::download(&client, &projekte, &known(&app)).unwrap(),
    )
    .unwrap();

    // Nur umbenennen: Abruf bleibt.
    source::update(
        app.conn(),
        projekte.id,
        SourceInput::new("Projekte 2026", &projekte.database_id, vec![]),
    )
    .unwrap();
    assert!(fetch::status(app.conn(), projekte.id).unwrap().is_some());

    // Andere Datenbank: Abruf weg.
    source::update(
        app.conn(),
        projekte.id,
        SourceInput::new("Projekte 2026", Ids::load().database("ziele"), vec![]),
    )
    .unwrap();
    assert_eq!(fetch::status(app.conn(), projekte.id).unwrap(), None);
    assert!(fetch::pages(app.conn(), projekte.id).unwrap().is_empty());
}

#[test]
fn loeschen_nimmt_abruf_und_seiten_mit() {
    let app = app();
    let projekte = add(&app, "projekte", vec![]);
    let (client, _) = client(FixtureNotion::new());
    fetch::store(
        app.conn(),
        &fetch::download(&client, &projekte, &known(&app)).unwrap(),
    )
    .unwrap();

    source::delete(app.conn(), projekte.id).unwrap();

    let rows: i64 = app
        .conn()
        .query_row("SELECT count(*) FROM pages", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 0);
}

// --- Einrichtung: Spalten und Vorschlag --------------------------------------

#[test]
fn sieht_die_spalten_einer_datenbank_ohne_abruf_an() {
    let ids = Ids::load();
    let notion = FixtureNotion::new();
    let log = notion.log();
    let (client, _) = client(notion);

    let schema = fetch::inspect(&client, &ids.database("roadmap")).unwrap();

    assert_eq!(schema.title, "vizu Roadmap");
    let namen: Vec<&str> = schema.properties.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(namen, ["Name", "Nächstes", "Start", "Status", "Tags"]);
    // Zwei Anfragen, keine Seite: Die Einrichtung soll nicht 130 Seiten laden.
    assert_eq!(log.lock().unwrap().len(), 2);
}

#[test]
fn schlaegt_die_zuordnung_aus_den_spalten_vor() {
    let ids = Ids::load();
    let (client, _) = client(FixtureNotion::new());

    let schema = fetch::inspect(&client, &ids.database("roadmap")).unwrap();

    assert_eq!(
        schema.suggestion,
        vec![
            ColumnMapping::new("date", "Start"),
            ColumnMapping::new("next", "Nächstes"),
            ColumnMapping::new("status", "Status"),
            ColumnMapping::new("tag", "Tags"),
            ColumnMapping::new("title", "Name"),
        ]
    );
}

#[test]
fn schlaegt_keine_relation_auf_eine_andere_datenbank_vor() {
    let ids = Ids::load();
    let (client, _) = client(FixtureNotion::new());

    // „vizu Projekte" hat zwei Relationen: „Nächstes" auf sich selbst und
    // „Ziel" auf die Ziele. Nur die erste kann ein Nachfolger sein.
    let schema = fetch::inspect(&client, &ids.database("projekte")).unwrap();

    let rolle = |spalte: &str| {
        schema
            .suggestion
            .iter()
            .find(|m| m.property == spalte)
            .map(|m| m.role.clone())
    };
    assert_eq!(rolle("Nächstes").as_deref(), Some("next"));
    // „Ziel" kommt dazu — aber unter seinem eigenen Namen, nie als `next`
    // oder `parent`.
    assert_eq!(rolle("Ziel").as_deref(), Some("ziel"));
}

#[test]
fn jede_spalte_bekommt_eine_zeile() {
    let ids = Ids::load();
    let (client, _) = client(FixtureNotion::new());

    // „vizu Projekte" hat sieben Spalten. Wer im Dialog „7 Spalten" liest,
    // soll darunter auch sieben Zeilen sehen — nicht nur die Sonderrollen.
    let schema = fetch::inspect(&client, &ids.database("projekte")).unwrap();

    let mut spalten: Vec<&str> = schema.properties.iter().map(|p| p.name.as_str()).collect();
    let mut zugeordnet: Vec<&str> = schema
        .suggestion
        .iter()
        .map(|m| m.property.as_str())
        .collect();
    spalten.sort_unstable();
    zugeordnet.sort_unstable();
    assert_eq!(zugeordnet, spalten, "keine Spalte doppelt, keine vergessen");

    // Die Sonderrollen stehen vorn, die übrigen tragen ihren Namen.
    let rollen: Vec<&str> = schema.suggestion.iter().map(|m| m.role.as_str()).collect();
    assert_eq!(
        rollen,
        ["date", "next", "status", "tag", "title", "phase", "ziel"]
    );

    // Und alles davon besteht die Prüfung einer Quelle.
    SourceInput::new("Projekte", ids.database("projekte"), schema.suggestion)
        .clean()
        .unwrap();
}

#[test]
fn eine_unlesbare_kennung_ist_ein_eingabefehler() {
    let (client, _) = client(FixtureNotion::new());

    let err = fetch::inspect(&client, "keine Kennung").unwrap_err();

    assert_eq!(err.code(), ErrorCode::ValidationFailed);
    assert!(
        err.fields()
            .expect("kein Eingabefehler")
            .iter()
            .any(|f| f.field == "database_id")
    );
}
