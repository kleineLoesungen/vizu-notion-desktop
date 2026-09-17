//! Gespeicherte Ansichten: anlegen, finden, überschreiben, löschen.

use vizu_notion_core::view::{self, ViewInput};
use vizu_notion_core::{App, ErrorCode};

fn app() -> App {
    App::in_memory().unwrap()
}

#[test]
fn haelt_fest_was_am_diagramm_eingestellt_war() {
    let app = app();

    let saved = view::create(
        app.conn(),
        ViewInput::new("Roadmap ohne Altlasten", "metro", "a")
            .hiding(vec!["p2".into(), "p1".into()]),
    )
    .unwrap();

    assert_eq!(saved.name, "Roadmap ohne Altlasten");
    assert_eq!(saved.kind, "metro");
    assert_eq!(saved.target, "a");
    // Sortiert und ohne Doppelte, damit zwei gleiche Ansichten gleich aussehen.
    assert_eq!(saved.hidden, vec!["p1", "p2"]);
    assert_eq!(saved.subtitle, None);
    assert_eq!(view::get(app.conn(), saved.id).unwrap(), saved);
}

#[test]
fn merkt_sich_die_zweite_zeile_des_flusses() {
    let app = app();

    let saved = view::create(
        app.conn(),
        ViewInput::new("Fluss mit Status", "flow", "a").with_subtitle(Some("status".into())),
    )
    .unwrap();

    assert_eq!(saved.subtitle.as_deref(), Some("status"));
}

#[test]
fn raeumt_die_liste_der_ausgeblendeten_seiten_auf() {
    let app = app();

    let saved = view::create(
        app.conn(),
        ViewInput::new("Aufgeräumt", "template", "t1").hiding(vec![
            " p1 ".into(),
            "".into(),
            "p1".into(),
        ]),
    )
    .unwrap();

    assert_eq!(saved.hidden, vec!["p1"]);
}

#[test]
fn eine_unbekannte_art_wird_abgelehnt() {
    let app = app();

    let err = view::create(app.conn(), ViewInput::new("Gantt", "gantt", "a")).unwrap_err();

    assert_eq!(err.code(), ErrorCode::ValidationFailed);
    assert!(
        err.fields()
            .expect("kein Eingabefehler")
            .iter()
            .any(|f| f.field == "kind")
    );
}

#[test]
fn ein_leerer_name_wird_abgelehnt() {
    let app = app();

    let err = view::create(app.conn(), ViewInput::new("   ", "flow", "a")).unwrap_err();

    assert!(
        err.fields()
            .expect("kein Eingabefehler")
            .iter()
            .any(|f| f.field == "name")
    );
}

#[test]
fn denselben_namen_gibt_es_nur_einmal() {
    let app = app();
    view::create(app.conn(), ViewInput::new("Roadmap", "metro", "a")).unwrap();

    let err = view::create(app.conn(), ViewInput::new("roadmap", "flow", "b")).unwrap_err();

    assert_eq!(err.code(), ErrorCode::ValidationFailed);
    assert!(err.to_string().contains("gibt es schon"), "{err}");
}

#[test]
fn eine_ansicht_laesst_sich_ueberschreiben() {
    let app = app();
    let saved = view::create(
        app.conn(),
        ViewInput::new("Roadmap", "metro", "a").hiding(vec!["p1".into()]),
    )
    .unwrap();

    let changed = view::update(
        app.conn(),
        saved.id,
        ViewInput::new("Roadmap", "metro", "a").hiding(vec!["p2".into()]),
    )
    .unwrap();

    assert_eq!(changed.hidden, vec!["p2"]);
    assert_eq!(changed.created_at, saved.created_at);
    assert_eq!(view::list(app.conn()).unwrap().len(), 1);
}

#[test]
fn findet_eine_ansicht_ueber_namen_kennung_und_endstueck() {
    let app = app();
    let saved = view::create(app.conn(), ViewInput::new("Roadmap", "metro", "a")).unwrap();
    let id = saved.id.to_string();

    assert_eq!(view::resolve(app.conn(), "Roadmap").unwrap(), saved.id);
    assert_eq!(view::resolve(app.conn(), "roadmap").unwrap(), saved.id);
    assert_eq!(view::resolve(app.conn(), &id).unwrap(), saved.id);
    assert_eq!(
        view::resolve(app.conn(), &id[id.len() - 8..]).unwrap(),
        saved.id
    );
}

#[test]
fn eine_ansicht_auf_etwas_geloeschtes_bleibt_stehen() {
    let app = app();
    // Kein Fremdschlüssel: Die Ansicht zeigt ins Leere, das Öffnen meldet es.
    // Sie deshalb stillschweigend zu löschen wäre schlimmer.
    let saved = view::create(app.conn(), ViewInput::new("Alt", "template", "weg")).unwrap();

    assert_eq!(view::list(app.conn()).unwrap(), vec![saved]);
}

#[test]
fn loeschen_meldet_eine_unbekannte_kennung() {
    let app = app();
    let saved = view::create(app.conn(), ViewInput::new("Roadmap", "metro", "a")).unwrap();

    view::delete(app.conn(), saved.id).unwrap();

    assert!(view::list(app.conn()).unwrap().is_empty());
    assert_eq!(
        view::delete(app.conn(), saved.id).unwrap_err().code(),
        ErrorCode::NotFound
    );
}
