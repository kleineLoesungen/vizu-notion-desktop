//! Der Assistent: aus einer Auswahl eine Vorlage, die zeichnet.
//!
//! Jede Kombination wird mit echten Daten gerendert und neben den Beispielen
//! festgehalten (`tests/fixtures/examples/assistent-*.mmd`) — dort liest
//! `ui/src/lib/examples.test.ts` sie mit mermaid.js.
//!
//! Neu schreiben: `UPDATE_EXAMPLES=1 cargo test -p vizu-notion-core --test compose`.

mod common;

use std::collections::HashSet;
use std::path::PathBuf;

use common::{FixtureNotion, Ids, client};
use vizu_notion_core::source::{self, ColumnMapping, SourceInput};
use vizu_notion_core::template::{self, SourcePart, Spec, assistant, compose};
use vizu_notion_core::{App, ErrorCode, fetch};

fn app() -> App {
    let app = App::in_memory().unwrap();
    let ids = Ids::load();
    let (client, _) = client(FixtureNotion::new());
    let projekte = source::create(
        app.conn(),
        SourceInput::new(
            "Projekte",
            ids.database("projekte"),
            vec![
                ColumnMapping::new("title", "Name"),
                ColumnMapping::new("next", "Nächstes"),
                ColumnMapping::new("date", "Start"),
                ColumnMapping::new("tag", "Tags"),
                ColumnMapping::new("status", "Status"),
                ColumnMapping::new("parent", "Ziel"),
            ],
        ),
    )
    .unwrap();
    let aufgaben = source::create(
        app.conn(),
        SourceInput::new(
            "Aufgaben",
            ids.database("aufgaben"),
            vec![
                ColumnMapping::new("title", "Name"),
                ColumnMapping::new("projekt", "Projekt"),
                ColumnMapping::new("punkte", "Punkte"),
                ColumnMapping::new("erledigt", "Erledigt"),
            ],
        ),
    )
    .unwrap();
    for s in [projekte, aufgaben] {
        let known = fetch::known_titles(app.conn()).unwrap();
        let download = fetch::download(&client, &s, &known).unwrap();
        fetch::store(app.conn(), &download).unwrap();
    }
    app
}

fn spec(kind: &str, source: &str) -> Spec {
    Spec {
        kind: kind.to_string(),
        title: format!("{kind} aus {source}"),
        sources: vec![SourcePart::new(source)],
        ..Spec::default()
    }
}

/// Dieselbe Auswahl mit mehreren Quellen.
fn specs(kind: &str, sources: &[&str]) -> Spec {
    Spec {
        kind: kind.to_string(),
        title: format!("{kind} aus {}", sources.join(" und ")),
        sources: sources.iter().map(|s| SourcePart::new(*s)).collect(),
        ..Spec::default()
    }
}

/// Pfeile entlang `link` für die Quelle an Stelle `i`, optional zu `to`.
fn link(mut spec: Spec, i: usize, role: &str, to: Option<&str>) -> Spec {
    spec.sources[i].link = Some(role.to_string());
    spec.sources[i].link_to = to.map(str::to_string);
    spec
}

fn with(mut spec: Spec, set: impl FnOnce(&mut Spec)) -> Spec {
    set(&mut spec);
    spec
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/examples")
        .join(format!("assistent-{name}.mmd"))
}

#[test]
fn jede_kombination_zeichnet_und_bleibt_wie_festgehalten() {
    let app = app();
    let update = std::env::var_os("UPDATE_EXAMPLES").is_some();
    let some = |s: &str| Some(s.to_string());

    let faelle: Vec<(&str, Spec)> = vec![
        ("flowchart-knoten", spec("flowchart", "Projekte")),
        (
            "flowchart-pfeile-farbe",
            with(link(spec("flowchart", "Projekte"), 0, "next", None), |s| {
                s.color = some("status");
            }),
        ),
        // Rahmen und Pfeile zugleich — ging mit `nodeId` nicht, weil der
        // Knoten im Rahmen eine andere Kennung bekam.
        (
            "flowchart-rahmen-pfeile-farbe",
            with(link(spec("flowchart", "Projekte"), 0, "next", None), |s| {
                s.group = some("status");
                s.color = some("tag");
            }),
        ),
        (
            "pie-anzahl",
            with(spec("pie", "Projekte"), |s| s.group = some("status")),
        ),
        (
            "pie-summe",
            with(spec("pie", "Aufgaben"), |s| {
                s.group = some("erledigt");
                s.sum = some("punkte");
            }),
        ),
        (
            "gantt",
            with(spec("gantt", "Projekte"), |s| s.date = some("date")),
        ),
        (
            "gantt-abschnitte",
            with(spec("gantt", "Projekte"), |s| {
                s.date = some("date");
                s.group = some("status");
            }),
        ),
        ("mindmap", spec("mindmap", "Projekte")),
        (
            "mindmap-zweige",
            with(spec("mindmap", "Projekte"), |s| s.group = some("parent")),
        ),
        // Mehrere Quellen: Pfeile von Aufgaben zu ihrem Projekt, ein Rahmen
        // und eine Farbe je Quelle.
        (
            "flowchart-zwei-quellen",
            with(
                link(
                    link(
                        specs("flowchart", &["Aufgaben", "Projekte"]),
                        0,
                        "projekt",
                        Some("Projekte"),
                    ),
                    1,
                    "next",
                    None,
                ),
                |s| {
                    s.by_source = true;
                    s.color_by_source = true;
                },
            ),
        ),
        ("pie-je-quelle", specs("pie", &["Aufgaben", "Projekte"])),
        (
            "gantt-zwei-quellen",
            with(specs("gantt", &["Projekte", "Aufgaben"]), |s| {
                s.date = some("date");
            }),
        ),
        (
            "mindmap-zwei-quellen",
            with(specs("mindmap", &["Projekte", "Aufgaben"]), |s| {
                s.group = some("status");
            }),
        ),
    ];

    for (name, spec) in faelle {
        let body = compose(&spec).unwrap_or_else(|e| panic!("{name}: {e}"));
        let diagram = template::render_body(app.conn(), &body, &HashSet::new())
            .unwrap_or_else(|e| panic!("{name} zeichnet nicht: {e}\n{body}"));
        assert!(
            diagram.mermaid.lines().count() > 2,
            "{name} ist fast leer: {:?}",
            diagram.mermaid
        );

        let path = fixture_path(name);
        if update {
            std::fs::write(&path, &diagram.mermaid).unwrap();
            continue;
        }
        let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "{}: {e} — neu schreiben mit UPDATE_EXAMPLES=1",
                path.display()
            )
        });
        assert_eq!(diagram.mermaid, expected, "{name}");
    }
}

#[test]
fn rahmen_und_pfeile_treffen_dieselben_knoten() {
    let app = app();
    let body = compose(&with(
        link(spec("flowchart", "Projekte"), 0, "next", None),
        |s| {
            s.group = Some("status".into());
        },
    ))
    .unwrap();
    let mermaid = template::render_body(app.conn(), &body, &HashSet::new())
        .unwrap()
        .mermaid;

    // Jede Kennung, auf die ein Pfeil zeigt, steht auch in einem Rahmen —
    // sonst hinge der Pfeil an einem zweiten, losen Knoten.
    let in_frames: HashSet<&str> = mermaid
        .lines()
        .filter(|l| l.starts_with("    n"))
        .filter_map(|l| l.trim().split('[').next())
        .collect();
    let targets: Vec<&str> = mermaid
        .lines()
        .filter_map(|l| l.split(" --> ").nth(1))
        .filter_map(|t| t.split('[').next())
        .collect();
    assert!(!targets.is_empty(), "keine Pfeile:\n{mermaid}");
    for target in targets {
        assert!(
            in_frames.contains(target),
            "{target} steht in keinem Rahmen:\n{mermaid}"
        );
    }
}

#[test]
fn ein_quellname_mit_leerzeichen_wird_vorher_abgelehnt() {
    let err = compose(&spec("flowchart", "vizu Roadmap")).unwrap_err();

    assert_eq!(err.code(), ErrorCode::ValidationFailed);
    let fields = err.fields().expect("kein Eingabefehler");
    let source = fields
        .iter()
        .find(|f| f.field == "sources")
        .expect("sources");
    assert!(source.message.contains("umbenennen"), "{}", source.message);
    assert!(!template::usable_source_name("vizu Roadmap"));
    assert!(template::usable_source_name("vizu_Roadmap"));
}

#[test]
fn was_eine_art_braucht_fehlt_am_feld() {
    let pie = compose(&spec("pie", "Projekte")).unwrap_err();
    assert!(pie.fields().unwrap().iter().any(|f| f.field == "group"));

    let gantt = compose(&spec("gantt", "Projekte")).unwrap_err();
    assert!(gantt.fields().unwrap().iter().any(|f| f.field == "date"));

    let unbekannt = compose(&spec("sequence", "Projekte")).unwrap_err();
    assert!(
        unbekannt
            .fields()
            .unwrap()
            .iter()
            .any(|f| f.field == "kind")
    );
}

#[test]
fn eine_rolle_ist_ein_bezeichner_und_keine_syntax() {
    let err = compose(&link(
        spec("flowchart", "Projekte"),
        0,
        "next}}{{evil",
        None,
    ))
    .unwrap_err();
    assert!(err.fields().unwrap().iter().any(|f| f.field == "link"));
}

#[test]
fn der_titel_zerbricht_die_zeile_nicht() {
    let body = compose(&with(spec("pie", "Projekte"), |s| {
        s.group = Some("status".into());
        s.title = "Status: \"alle\" (2026)".into();
    }))
    .unwrap();
    assert!(body.contains("title Status alle 2026\n"), "{body}");
}

#[test]
fn das_kreisdiagramm_zaehlt_seiten_nicht_zeilen() {
    let app = app();
    let body = compose(&with(spec("pie", "Projekte"), |s| {
        s.group = Some("status".into());
    }))
    .unwrap();
    let mermaid = template::render_body(app.conn(), &body, &HashSet::new())
        .unwrap()
        .mermaid;

    // Zwölf Projekte. „Q3 Review & Plan" hat drei Ziele und stünde ohne
    // `group-pages` dreimal im Kontext — die Summe der Stücke wäre dann 14.
    let total: u32 = mermaid
        .lines()
        .filter_map(|l| l.rsplit(" : ").next()?.trim().parse::<u32>().ok())
        .sum();
    assert_eq!(total, 12, "{mermaid}");
}

#[test]
fn ein_gantt_zeigt_jede_seite_einmal() {
    let app = app();
    let body = compose(&with(spec("gantt", "Projekte"), |s| {
        s.date = Some("date".into());
    }))
    .unwrap();
    let mermaid = template::render_body(app.conn(), &body, &HashSet::new())
        .unwrap()
        .mermaid;

    assert_eq!(mermaid.matches("Q3 Review & Plan").count(), 1, "{mermaid}");
}

#[test]
fn pfeile_zwischen_quellen_treffen_die_kaesten_beider_quellen() {
    let app = app();
    let body = compose(&link(
        specs("flowchart", &["Aufgaben", "Projekte"]),
        0,
        "projekt",
        Some("Projekte"),
    ))
    .unwrap();
    let mermaid = template::render_body(app.conn(), &body, &HashSet::new())
        .unwrap()
        .mermaid;

    // Jeder Kasten steht einmal als eigene Zeile; jeder Pfeil muss von einem
    // Aufgaben-Kasten zu einem Projekte-Kasten gehen, nicht zu einem losen.
    let boxes: HashSet<&str> = mermaid
        .lines()
        .filter(|l| !l.contains("-->"))
        .filter_map(|l| l.trim().split('[').next())
        .filter(|id| id.starts_with('n'))
        .collect();
    let edges: Vec<(&str, &str)> = mermaid
        .lines()
        .filter_map(|l| l.split_once(" --> "))
        .filter_map(|(a, b)| Some((a.trim().split('[').next()?, b.split('[').next()?)))
        .collect();
    assert!(!edges.is_empty(), "keine Pfeile:\n{mermaid}");
    for (from, to) in edges {
        assert!(boxes.contains(from), "{from} ist kein Kasten:\n{mermaid}");
        assert!(boxes.contains(to), "{to} ist kein Kasten:\n{mermaid}");
    }
}

#[test]
fn derselbe_wert_hat_in_jeder_quelle_dieselbe_farbe() {
    let app = app();
    // „status" gibt es nur in Projekte; mit zwei Quellen und Farbe je Quelle
    // bekommt jede Quelle genau eine Farbe, einmal definiert.
    let body = compose(&with(specs("flowchart", &["Aufgaben", "Projekte"]), |s| {
        s.color_by_source = true;
    }))
    .unwrap();
    let mermaid = template::render_body(app.conn(), &body, &HashSet::new())
        .unwrap()
        .mermaid;

    let defs: Vec<&str> = mermaid
        .lines()
        .filter(|l| l.trim().starts_with("classDef v-"))
        .collect();
    assert_eq!(defs.len(), 2, "{mermaid}");
    let farbe = |line: &str| line.trim().split(' ').nth(2).map(str::to_string);
    assert_ne!(farbe(defs[0]), farbe(defs[1]), "zwei Quellen, zwei Farben");
}

#[test]
fn die_auswahl_steht_im_kopf_und_kommt_zurueck() {
    let auswahl = with(link(spec("flowchart", "Projekte"), 0, "next", None), |s| {
        s.group = Some("status".into());
        s.color = Some("tag".into());
    });
    let body = compose(&auswahl).unwrap();

    let state = assistant(&body).expect("keine Auswahl im Kopf");
    assert_eq!(state.spec, auswahl);
    assert!(!state.edited, "frisch gebaut gilt nicht als geändert");

    // Der Kopf bleibt eine gültige Vorlage — `assistant` stört ihn nicht.
    let meta = template::parse_meta(&body).unwrap();
    assert_eq!(meta.sources, vec!["Projekte"]);
}

#[test]
fn eine_von_hand_geaenderte_vorlage_wird_erkannt() {
    let body = compose(&spec("mindmap", "Projekte")).unwrap();
    let geaendert = format!("{body}    Nachtrag\n");

    assert!(assistant(&geaendert).unwrap().edited);
    // Eine Vorlage ohne Assistent hat keine Auswahl.
    assert!(assistant("---\ntitle: \"x\"\nsources:\n  - Projekte\n---\nflowchart TD\n").is_none());
}

#[test]
fn pfeile_zu_einer_nicht_gewaehlten_quelle_werden_abgelehnt() {
    let err = compose(&link(
        spec("flowchart", "Aufgaben"),
        0,
        "projekt",
        Some("Projekte"),
    ))
    .unwrap_err();
    assert!(err.fields().unwrap().iter().any(|f| f.field == "link"));
}
