//! Das Flussdiagramm: Kanten aus `next`, Ebenen, Anordnung.

mod common;

use std::collections::HashSet;

use common::{FixtureNotion, Ids, client};
use vizu_notion_core::source::{self, ColumnMapping, Source, SourceInput};
use vizu_notion_core::{App, fetch, flow};

/// Die Projekte aus den festgehaltenen Antworten, abgerufen und gespeichert.
fn app_mit_projekten() -> (App, Source) {
    let app = App::in_memory().unwrap();
    let source = source::create(
        app.conn(),
        SourceInput::new(
            "Projekte",
            Ids::load().database("projekte"),
            vec![
                ColumnMapping::new("title", "Name"),
                ColumnMapping::new("next", "Nächstes"),
                ColumnMapping::new("status", "Status"),
                ColumnMapping::new("date", "Start"),
            ],
        ),
    )
    .unwrap();
    let (client, _) = client(FixtureNotion::new());
    let known = fetch::known_titles(app.conn()).unwrap();
    let download = fetch::download(&client, &source, &known).unwrap();
    fetch::store(app.conn(), &download).unwrap();
    (app, source)
}

fn graph(app: &App, source: &Source, hidden: &HashSet<String>) -> flow::FlowGraph {
    flow::build(app.conn(), source.id, hidden, Some("status")).unwrap()
}

#[test]
fn eine_quelle_ohne_next_taugt_nicht_als_fluss() {
    let ohne = SourceInput::new(
        "Ziele",
        "396f66270f5d8034b55cebc685aa5e50",
        vec![ColumnMapping::new("title", "Name")],
    )
    .clean()
    .unwrap();
    let app = App::in_memory().unwrap();
    let ohne = source::create(app.conn(), ohne).unwrap();
    assert!(!flow::eligible(&ohne));

    let (_, mit) = app_mit_projekten();
    assert!(flow::eligible(&mit));
}

#[test]
fn baut_knoten_und_kanten_aus_der_relation() {
    let (app, source) = app_mit_projekten();

    let graph = graph(&app, &source, &HashSet::new());

    assert_eq!(graph.nodes.len(), 12);
    assert_eq!(graph.all_nodes.len(), 12);
    // Zwölf Projekte, neun davon mit Nachfolger, eines mit zweien.
    assert_eq!(graph.edges.len(), 9);
    assert!(graph.width > 0.0 && graph.height > 0.0);

    // Jede Seite ist ein eigener Knoten — auch die zwei „Website".
    let websites: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.title == "Website")
        .collect();
    assert_eq!(websites.len(), 2);
    assert_ne!(websites[0].id, websites[1].id);
}

#[test]
fn die_zweite_zeile_kommt_aus_der_gewaehlten_rolle() {
    let (app, source) = app_mit_projekten();

    let graph = graph(&app, &source, &HashSet::new());
    let launch = graph.nodes.iter().find(|n| n.title == "Launch").unwrap();
    assert_eq!(launch.subtitle, "Done");

    let ohne = flow::build(app.conn(), source.id, &HashSet::new(), None).unwrap();
    assert!(ohne.nodes.iter().all(|n| n.subtitle.is_empty()));

    let datum = flow::build(app.conn(), source.id, &HashSet::new(), Some("date")).unwrap();
    let launch = datum.nodes.iter().find(|n| n.title == "Launch").unwrap();
    assert_eq!(launch.subtitle, "2026-04-01");

    // Titel und Nachfolger stehen schon im Knoten selbst.
    assert_eq!(graph.subtitle_roles, vec!["date", "status"]);
}

#[test]
fn ein_nachfolger_steht_unter_seinem_vorgaenger() {
    let (app, source) = app_mit_projekten();
    let graph = graph(&app, &source, &HashSet::new());

    for edge in &graph.edges {
        let from = graph.nodes.iter().find(|n| n.id == edge.from).unwrap();
        let to = graph.nodes.iter().find(|n| n.id == edge.to).unwrap();
        assert!(
            to.y > from.y,
            "{} ({}) liegt nicht unter {} ({})",
            to.title,
            to.y,
            from.title,
            from.y
        );
    }
}

#[test]
fn ausgeblendete_seiten_nehmen_ihre_kanten_mit() {
    let (app, source) = app_mit_projekten();
    let alle = graph(&app, &source, &HashSet::new());
    let launch = alle.nodes.iter().find(|n| n.title == "Launch").unwrap();
    let hidden: HashSet<String> = [launch.id.clone()].into_iter().collect();

    let gefiltert = graph(&app, &source, &hidden);

    assert_eq!(gefiltert.nodes.len(), 11);
    assert!(
        gefiltert
            .edges
            .iter()
            .all(|e| e.to != launch.id && e.from != launch.id)
    );
    // Die Liste fürs Filterfeld behält alle — sonst käme der Knoten nie zurück.
    assert_eq!(gefiltert.all_nodes.len(), 12);
}

#[test]
fn ein_kreis_bringt_die_anordnung_nicht_zum_stehen() {
    // A → B → A gibt es in Notion durchaus: zwei Seiten, die sich gegenseitig
    // als Nachfolger tragen. Ohne Bremse liefe die Ebenenrechnung ewig.
    let app = App::in_memory().unwrap();
    let source = source::create(
        app.conn(),
        SourceInput::new(
            "Kreis",
            // Der Abruf holt eine echte Antwort; die Seiten ersetzen wir gleich.
            Ids::load().database("projekte"),
            vec![
                ColumnMapping::new("title", "Name"),
                ColumnMapping::new("next", "Nächstes"),
            ],
        ),
    )
    .unwrap();
    let pages: Vec<vizu_notion_core::notion::Page> = [("a", "A", "b"), ("b", "B", "a")]
        .iter()
        .map(|(id, title, next)| {
            serde_json::from_value(serde_json::json!({
                "id": id,
                "properties": {
                    "Name": { "type": "title", "title": [{ "plain_text": title }] },
                    "Nächstes": { "type": "relation", "relation": [{ "id": next }] }
                }
            }))
            .unwrap()
        })
        .collect();
    let (client, _) = client(FixtureNotion::new());
    let known = fetch::known_titles(app.conn()).unwrap();
    let mut download = fetch::download(&client, &source, &known).unwrap();
    download.pages = pages;
    fetch::store(app.conn(), &download).unwrap();

    let graph = flow::build(app.conn(), source.id, &HashSet::new(), None).unwrap();

    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 2);
}
