//! Von Notion-Seiten zu Zeilen, wie eine Vorlage sie sieht.
//!
//! Arbeitet auf den festgehaltenen Antworten aus `tests/fixtures/notion/`.

mod common;

use std::collections::HashSet;

use common::{FixtureNotion, Ids, client};
use serde_json::json;
use vizu_notion_core::rows::{self, Row};
use vizu_notion_core::source::{self, ColumnMapping, SourceInput};
use vizu_notion_core::{App, fetch};

/// Drei verknüpfte Quellen, abgerufen und gespeichert.
fn app_mit_daten() -> App {
    let app = App::in_memory().unwrap();
    let ids = Ids::load();
    let mapping = |pairs: &[(&str, &str)]| {
        pairs
            .iter()
            .map(|(r, p)| ColumnMapping::new(*r, *p))
            .collect::<Vec<_>>()
    };
    let sources = [
        (
            "ziele",
            "Ziele",
            mapping(&[("title", "Name"), ("note", "Beschreibung")]),
        ),
        (
            "projekte",
            "Projekte",
            mapping(&[
                ("title", "Name"),
                ("date", "Start"),
                ("status", "Status"),
                ("tag", "Tags"),
                ("parent", "Ziel"),
                ("next", "Nächstes"),
                ("phase", "Phase"),
            ]),
        ),
        (
            "aufgaben",
            "Aufgaben",
            mapping(&[
                ("title", "Name"),
                ("parent", "Projekt"),
                ("done", "Erledigt"),
                ("points", "Punkte"),
            ]),
        ),
    ];
    let (client, _) = client(FixtureNotion::new());
    for (label, name, mappings) in sources {
        let source = source::create(
            app.conn(),
            SourceInput::new(name, ids.database(label), mappings),
        )
        .unwrap();
        let known = fetch::known_titles(app.conn()).unwrap();
        let download = fetch::download(&client, &source, &known).unwrap();
        fetch::store(app.conn(), &download).unwrap();
    }
    app
}

fn context(app: &App, names: &[&str], hidden: &HashSet<String>) -> (Vec<Row>, Vec<Row>) {
    let names: Vec<String> = names.iter().map(|n| n.to_string()).collect();
    let (context, _) = rows::context(app.conn(), &names, hidden).unwrap();
    (
        context.get("Projekte").cloned().unwrap_or_default(),
        context.get("Ziele").cloned().unwrap_or_default(),
    )
}

fn find<'a>(rows: &'a [Row], title: &str) -> Vec<&'a Row> {
    rows.iter().filter(|r| r["title"] == title).collect()
}

#[test]
fn macht_aus_jeder_spaltenart_text() {
    let app = app_mit_daten();
    let (projekte, _) = context(&app, &["Projekte"], &HashSet::new());

    let launch = find(&projekte, "Launch")[0];
    assert_eq!(launch["date"], "2026-04-01");
    assert_eq!(
        launch["status"], "Done",
        "status statt leer — anders als die Webapp"
    );
    assert_eq!(launch["tag"], "Web, Mobil", "Mehrfachauswahl als ein Text");
    assert_eq!(launch["phase"], "geplant", "Formel statt leer");
    assert!(launch["id"].len() > 10, "die Seiten-ID ist immer dabei");

    let ohne = find(&projekte, "Ohne Ziel")[0];
    assert_eq!(ohne["date"], "", "kein Datum gesetzt");
    assert_eq!(ohne["tag"], "");
    assert_eq!(ohne["parent"], "", "Relation ohne Ziel");
}

#[test]
fn verbindet_einen_titel_aus_mehreren_textstuecken() {
    // „Reichweite **stärken**" steht in Notion als zwei Stücke. Die Webapp
    // nahm nur das erste und zeigte „Reichweite ".
    let app = app_mit_daten();
    let (_, ziele) = context(&app, &["Ziele"], &HashSet::new());
    assert!(
        ziele.iter().any(|r| r["title"] == "Reichweite stärken"),
        "{:?}",
        ziele.iter().map(|r| &r["title"]).collect::<Vec<_>>()
    );
}

#[test]
fn loest_relationen_zu_titeln_auf() {
    let app = app_mit_daten();
    let (projekte, _) = context(&app, &["Projekte"], &HashSet::new());

    let newsletter = find(&projekte, "Newsletter")[0];
    assert_eq!(newsletter["parent"], "Reichweite stärken");
    assert_eq!(newsletter["next"], "App 🚀");
}

#[test]
fn faechert_mehrfachrelationen_in_mehrere_zeilen_auf() {
    // „Q3" hat drei Ziele und einen Nachfolger → drei Zeilen. Damit ergibt
    // `{{title}} --> {{parent}}` je Ziel eine Kante, ohne Zutun der Vorlage.
    let app = app_mit_daten();
    let (projekte, _) = context(&app, &["Projekte"], &HashSet::new());

    let q3 = find(&projekte, "Q3 \"Review\" & [Plan]");
    assert_eq!(q3.len(), 3);
    let mut ziele: Vec<&str> = q3.iter().map(|r| r["parent"].as_str()).collect();
    ziele.sort_unstable();
    assert_eq!(ziele, ["Kosten senken", "Qualität", "Wachstum"]);
    assert!(q3.iter().all(|r| r["next"] == "Website"));

    // Zwei Nachfolger × ein Ziel = zwei Zeilen.
    assert_eq!(find(&projekte, "Testautomatisierung").len(), 2);
    // Ohne Mehrfachrelation bleibt es bei einer Zeile.
    assert_eq!(find(&projekte, "Launch").len(), 1);
}

#[test]
fn ausgeblendete_seiten_verschwinden_auch_als_relationsziel() {
    let app = app_mit_daten();
    let (_, ziele) = context(&app, &["Ziele"], &HashSet::new());
    let wachstum = find(&ziele, "Wachstum")[0]["id"].clone();
    let hidden: HashSet<String> = [wachstum.clone()].into_iter().collect();

    let (projekte, ziele) = context(&app, &["Projekte", "Ziele"], &hidden);

    assert!(
        !ziele.iter().any(|r| r["id"] == wachstum),
        "eigene Zeile weg"
    );
    let q3 = find(&projekte, "Q3 \"Review\" & [Plan]");
    assert_eq!(q3.len(), 2, "nur noch zwei Ziele");
    assert!(q3.iter().all(|r| r["parent"] != "Wachstum"));
    let app_projekt = find(&projekte, "App 🚀")[0];
    assert_eq!(
        app_projekt["parent"], "Reichweite stärken",
        "das zweite Ziel rückt vor"
    );
}

#[test]
fn die_knotenliste_enthaelt_auch_ausgeblendete_seiten() {
    // Das Filterfeld muss sie wieder einblenden können.
    let app = app_mit_daten();
    let names = vec!["Projekte".to_string()];
    let (_, alle) = rows::context(app.conn(), &names, &HashSet::new()).unwrap();
    let erstes = alle[0].id.clone();

    let hidden: HashSet<String> = [erstes.clone()].into_iter().collect();
    let (context, nodes) = rows::context(app.conn(), &names, &hidden).unwrap();

    assert_eq!(nodes.len(), 12);
    assert!(nodes.iter().any(|n| n.id == erstes));
    // Zwölf Seiten ergeben sechzehn Zeilen (drei Projekte mit
    // Mehrfachrelationen); die ausgeblendete erste Seite ist „Q3" mit ihren
    // dreien.
    assert_eq!(context["Projekte"].len(), 16 - 3);
    assert!(nodes.iter().all(|n| n.source == "Projekte"));
    assert!(
        nodes.iter().any(|n| !n.relations.is_empty()),
        "Relationen für „verwandte Knoten“"
    );
}

#[test]
fn kennt_titel_von_seiten_ausserhalb_der_quellen() {
    // Die Aufgaben zeigen auf Projekte. Ist nur die Aufgaben-Quelle
    // eingerichtet, kommen die Titel aus page_titles, das der Abruf gefüllt hat.
    let app = App::in_memory().unwrap();
    let ids = Ids::load();
    let aufgaben = source::create(
        app.conn(),
        SourceInput::new(
            "Aufgaben",
            ids.database("aufgaben"),
            vec![
                ColumnMapping::new("title", "Name"),
                ColumnMapping::new("parent", "Projekt"),
            ],
        ),
    )
    .unwrap();
    let (client, _) = client(FixtureNotion::new());
    let known = fetch::known_titles(app.conn()).unwrap();
    let download = fetch::download(&client, &aufgaben, &known).unwrap();
    fetch::store(app.conn(), &download).unwrap();

    let names = vec!["Aufgaben".to_string()];
    let (context, _) = rows::context(app.conn(), &names, &HashSet::new()).unwrap();
    let rows = &context["Aufgaben"];

    assert_eq!(rows.len(), 130 + 7, "jede 17. Aufgabe hat zwei Projekte");
    let mit_projekt = rows.iter().find(|r| !r["parent"].is_empty()).unwrap();
    assert!(
        !mit_projekt["parent"].contains('-'),
        "Titel statt Seiten-ID: {}",
        mit_projekt["parent"]
    );
    assert!(
        rows.iter().any(|r| r["parent"].is_empty()),
        "jede 13. ohne Projekt"
    );
}

#[test]
fn ein_zweiter_abruf_holt_bekannte_titel_nicht_noch_einmal() {
    let app = app_mit_daten();
    let projekte =
        source::get(app.conn(), source::resolve(app.conn(), "Projekte").unwrap()).unwrap();
    let (client, _) = client(FixtureNotion::new());

    let known = fetch::known_titles(app.conn()).unwrap();
    let download = fetch::download(&client, &projekte, &known).unwrap();

    assert!(download.titles.is_empty(), "alle Ziele sind schon bekannt");
    assert_eq!(client.requests(), 3, "Datenbank, Schema, ein Stapel");
}

#[test]
fn eine_unbekannte_quelle_ist_ein_eingabefehler() {
    let app = app_mit_daten();
    let err = rows::context(app.conn(), &["Gibtsnicht".to_string()], &HashSet::new()).unwrap_err();
    let fields = err.fields().expect("Eingabefehler");
    assert_eq!(fields[0].field, "sources");
    assert!(
        fields[0].message.contains("Projekte"),
        "{}",
        fields[0].message
    );
}

#[test]
fn einzelne_werte_werden_wie_in_javascript_zu_text() {
    // Feinheiten, an denen sich die Ausgabe der Webapp messen lassen muss.
    assert_eq!(
        rows::text_of(&json!({ "type": "number", "number": 3.0 })),
        "3"
    );
    assert_eq!(
        rows::text_of(&json!({ "type": "number", "number": 3.5 })),
        "3.5"
    );
    assert_eq!(
        rows::text_of(&json!({ "type": "number", "number": null })),
        ""
    );
    assert_eq!(
        rows::text_of(&json!({ "type": "checkbox", "checkbox": true })),
        "true"
    );
    assert_eq!(
        rows::text_of(
            &json!({ "type": "unique_id", "unique_id": { "prefix": "TASK", "number": 7 } })
        ),
        "TASK-7"
    );
    assert_eq!(
        rows::text_of(&json!({ "type": "rollup", "rollup": { "type": "number", "number": 12 } })),
        "12"
    );
    assert_eq!(
        rows::text_of(&json!({ "type": "relation", "relation": [] })),
        ""
    );
}

// --- Feldwerte fürs Filterfeld -------------------------------------------------

#[test]
fn jede_seite_bringt_ihre_feldwerte_fuers_filterfeld_mit() {
    let app = app_mit_daten();
    let (_, nodes) = rows::context(app.conn(), &["Projekte".to_string()], &HashSet::new()).unwrap();

    let seite = |titel: &str| {
        nodes
            .iter()
            .find(|n| n.title == titel)
            .unwrap_or_else(|| panic!("{titel} fehlt"))
    };

    // Eine Mehrfachauswahl bleibt ein Wert — derselbe, aus dem `group` im
    // Diagramm eine Gruppe macht.
    let app_seite = seite("App 🚀");
    assert_eq!(app_seite.fields["tag"], vec!["Mobil, Kunde"]);

    // Eine Relation hat so viele Werte wie Ziele, als Titel.
    let q3 = seite("Q3 \"Review\" & [Plan]");
    assert_eq!(q3.fields["parent"].len(), 3, "{:?}", q3.fields["parent"]);
    assert!(q3.fields["parent"].iter().all(|t| !t.starts_with("0000")));

    // Eine leere Spalte hat keinen Wert, nicht einen leeren.
    let ohne = seite("Ohne Ziel");
    assert!(ohne.fields["parent"].is_empty());
    assert!(ohne.fields["date"].is_empty());
}
