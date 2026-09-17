//! Versteckte Diagramme: merken, zurückholen, prüfen.

use vizu_notion_core::App;
use vizu_notion_core::hidden::{self, HiddenDiagram};

fn eintrag(kind: &str, target: &str) -> HiddenDiagram {
    HiddenDiagram {
        kind: kind.to_string(),
        target: target.to_string(),
    }
}

#[test]
fn eine_frische_datenbank_versteckt_nichts() {
    let app = App::in_memory().unwrap();
    assert!(hidden::list(app.conn()).unwrap().is_empty());
}

#[test]
fn versteckt_und_holt_zurueck() {
    let app = App::in_memory().unwrap();
    let metro = eintrag("metro", "a");

    hidden::set(app.conn(), &metro, true).unwrap();
    assert_eq!(hidden::list(app.conn()).unwrap(), vec![metro.clone()]);

    hidden::set(app.conn(), &metro, false).unwrap();
    assert!(hidden::list(app.conn()).unwrap().is_empty());
}

#[test]
fn zweimal_verstecken_ist_kein_fehler() {
    let app = App::in_memory().unwrap();
    let flow = eintrag("flow", "a");

    hidden::set(app.conn(), &flow, true).unwrap();
    hidden::set(app.conn(), &flow, true).unwrap();

    assert_eq!(hidden::list(app.conn()).unwrap().len(), 1);
    // Zurückholen, was nie versteckt war, ebenso wenig.
    hidden::set(app.conn(), &eintrag("template", "b"), false).unwrap();
}

#[test]
fn dieselbe_kennung_in_zwei_arten_sind_zwei_eintraege() {
    let app = App::in_memory().unwrap();
    // Fluss und Metro derselben Quelle tragen dieselbe Kennung.
    hidden::set(app.conn(), &eintrag("flow", "a"), true).unwrap();
    hidden::set(app.conn(), &eintrag("metro", "a"), true).unwrap();

    assert_eq!(hidden::list(app.conn()).unwrap().len(), 2);

    hidden::set(app.conn(), &eintrag("flow", "a"), false).unwrap();
    assert_eq!(
        hidden::list(app.conn()).unwrap(),
        vec![eintrag("metro", "a")]
    );
}

#[test]
fn eine_unbekannte_art_wird_abgelehnt() {
    let app = App::in_memory().unwrap();

    let err = hidden::set(app.conn(), &eintrag("gantt", "a"), true).unwrap_err();

    let fields = err.fields().expect("kein Eingabefehler");
    assert!(fields.iter().any(|f| f.field == "kind"), "{fields:?}");
}

#[test]
fn ein_leeres_ziel_wird_abgelehnt() {
    let app = App::in_memory().unwrap();

    let err = hidden::set(app.conn(), &eintrag("metro", "  "), true).unwrap_err();

    assert!(
        err.fields()
            .expect("kein Eingabefehler")
            .iter()
            .any(|f| f.field == "target")
    );
}
