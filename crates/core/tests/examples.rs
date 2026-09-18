//! Die Beispiele aus dem Editor — gezeichnet, nicht nur angesehen.
//!
//! Jedes Beispiel wird mit echten Daten aus den Fixtures gerendert und mit der
//! festgehaltenen Ausgabe in `tests/fixtures/examples/` verglichen. Ein Fehler
//! in einer Vorlage fällt damit hier auf und nicht erst im Fenster.
//!
//! Ob Mermaid den erzeugten Text auch **versteht**, prüft
//! `ui/src/lib/examples.test.ts` — dieselben Dateien, gelesen von mermaid.js.
//!
//! Ändert sich ein Beispiel absichtlich: `UPDATE_EXAMPLES=1 cargo test
//! -p vizu-notion-core --test examples` schreibt die Dateien neu.

mod common;

use std::collections::HashSet;
use std::path::PathBuf;

use common::{FixtureNotion, Ids, client};
use vizu_notion_core::source::{self, ColumnMapping, SourceInput};
use vizu_notion_core::template::{self, examples};
use vizu_notion_core::{App, fetch};

/// Zwei Quellen mit allen Rollen, die die Beispiele benutzen: „Projekte" hat
/// `parent` (die Relation „Ziel"), und die Ziele sind die zweite Quelle —
/// sonst fände das Beispiel mit `join-rows` nichts zum Verbinden.
fn app_mit_quellen() -> App {
    let app = App::in_memory().unwrap();
    let ids = Ids::load();
    let (client, _) = client(FixtureNotion::new());

    for (name, fixture, mappings) in [
        ("Ziele", "ziele", vec![ColumnMapping::new("title", "Name")]),
        (
            "Projekte",
            "projekte",
            vec![
                ColumnMapping::new("title", "Name"),
                ColumnMapping::new("next", "Nächstes"),
                ColumnMapping::new("date", "Start"),
                ColumnMapping::new("tag", "Tags"),
                ColumnMapping::new("status", "Status"),
                // „Ziel" zeigt auf eine andere Datenbank — für die Beispiele
                // mit `parent` ist das genau die Überordnung.
                ColumnMapping::new("parent", "Ziel"),
            ],
        ),
    ] {
        let source = source::create(
            app.conn(),
            SourceInput::new(name, ids.database(fixture), mappings),
        )
        .unwrap();
        let known = fetch::known_titles(app.conn()).unwrap();
        let download = fetch::download(&client, &source, &known).unwrap();
        fetch::store(app.conn(), &download).unwrap();
    }
    app
}

fn fixture_path(name: &str) -> PathBuf {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'ä' => 'a',
            'ö' => 'o',
            'ü' => 'u',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        })
        .collect();
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/examples")
        .join(format!("{}.mmd", slug.trim_matches('-')))
}

#[test]
fn jedes_beispiel_zeichnet_und_bleibt_wie_festgehalten() {
    let app = app_mit_quellen();
    let update = std::env::var_os("UPDATE_EXAMPLES").is_some();
    let all = examples::examples();
    assert!(all.len() >= 10, "nur {} Beispiele", all.len());

    for example in all {
        // Im Editor setzt die Oberfläche die Namen der eingerichteten Quellen
        // ein; hier sind es die beiden aus den Fixtures.
        let body = example
            .body
            .replace("QUELLE", "Projekte")
            .replace("ZWEITE", "Ziele");
        let diagram = template::render_body(app.conn(), &body, &HashSet::new())
            .unwrap_or_else(|e| panic!("„{}“ zeichnet nicht: {e}", example.name));

        assert!(
            diagram.mermaid.lines().count() > 1,
            "„{}“ ergibt nur eine Zeile: {:?}",
            example.name,
            diagram.mermaid
        );

        let path = fixture_path(&example.name);
        if update {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &diagram.mermaid).unwrap();
            continue;
        }
        let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "{}: {e} — neu schreiben mit UPDATE_EXAMPLES=1",
                path.display()
            )
        });
        assert_eq!(
            diagram.mermaid,
            expected,
            "„{}“ zeichnet anders als festgehalten ({})",
            example.name,
            path.display()
        );
    }
}

/// Welche Rolle ein Baustein im Test für `FELD` bekommt — eine, bei der er in
/// den Fixtures auch etwas zeichnet.
fn feld_fuer(baustein: &str) -> &'static str {
    match baustein {
        "Pfeile entlang eines Felds" => "next",
        "Zwei Quellen verbinden" => "parent",
        "Nur Seiten mit gefülltem Feld" => "date",
        _ => "status",
    }
}

#[test]
fn jeder_baustein_zeichnet_und_bleibt_wie_festgehalten() {
    let app = app_mit_quellen();
    let update = std::env::var_os("UPDATE_EXAMPLES").is_some();
    let all = examples::blocks();
    assert!(all.len() >= 6, "nur {} Bausteine", all.len());

    for block in all {
        let fragment = block
            .body
            .replace("QUELLE", "Projekte")
            .replace("ZWEITE", "Ziele")
            .replace("FELD", feld_fuer(&block.name));
        // Ein Baustein ist ein Stück, keine Vorlage: Er bekommt den Kopf, den er
        // im Editor unter sich hätte. Das Gerüst bringt seinen eigenen mit.
        let body = if fragment.starts_with("---") {
            fragment
        } else {
            format!(
                "---\ntitle: \"Baustein\"\nsources:\n  - Projekte\n  - Ziele\n---\nflowchart LR\n{fragment}"
            )
        };
        let diagram = template::render_body(app.conn(), &body, &HashSet::new())
            .unwrap_or_else(|e| panic!("Baustein „{}“ zeichnet nicht: {e}", block.name));
        assert!(
            diagram.mermaid.lines().count() > 1,
            "Baustein „{}“ ergibt nur eine Zeile: {:?}",
            block.name,
            diagram.mermaid
        );

        // Neben den Beispielen, damit ui/src/lib/examples.test.ts sie
        // ebenfalls von mermaid.js lesen lässt.
        let path = fixture_path(&format!("baustein {}", block.name));
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
        assert_eq!(diagram.mermaid, expected, "Baustein „{}“", block.name);
    }
}
