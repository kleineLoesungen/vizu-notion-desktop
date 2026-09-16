//! Mermaid-Vorlagen: `.mmd`-Text mit Handlebars, gefüllt aus Notion-Daten.
//!
//! ```text
//! Vorlage (Kopf + Rumpf)
//!   │  rewrite:  {{title}} → {{nodeId "title" title source="Projekte"}}
//!   │  Kontext:  rows::context(conn, sources, hidden)
//!   ▼
//! Handlebars  →  Mermaid-Text  →  classDef- und class-Zeilen angehängt
//! ```
//!
//! **Die Ausgabe ist bytegleich zu der der Webapp vizu-notion-local.** Das ist
//! kein Zufall, sondern geprüft: `crates/core/tests/template.rs` vergleicht mit
//! Dateien, die die Original-Logik (handlebars.js 4.7) erzeugt hat. Wer hier
//! etwas ändert, ändert die Knotenkennungen in bestehenden Diagrammen —
//! Hintergrund in docs/UMSETZUNG.md, Abschnitt 3.
//!
//! Gezeichnet wird nicht hier: `core` erzeugt den Text, die Oberfläche gibt
//! ihn an mermaid.js, die Kommandozeile schreibt ihn auf stdout.

pub mod examples;
mod helpers;
mod parse;
mod rewrite;
mod store;
mod style;

use std::collections::HashSet;

use handlebars::Handlebars;
use rusqlite::Connection;
use serde::Serialize;
use serde_json::{Map, Value};
use ts_rs::TS;

pub use examples::{Example, Hint};
pub use parse::Meta;
pub use store::{Template, TemplateInput, create, delete, get, import_mmd, list, resolve, update};

use crate::error::{Result, Validator};
use crate::rows::{self, Context, NodeInfo};

/// Ein fertiges Diagramm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct Diagram {
    pub title: String,
    /// Der Mermaid-Text, so wie ihn auch `vizu-notion render` ausgibt.
    pub mermaid: String,
    /// Jede Seite der beteiligten Quellen — auch die ausgeblendeten, damit das
    /// Filterfeld sie wieder einblenden kann.
    pub nodes: Vec<NodeInfo>,
}

/// Zeichnet eine gespeicherte Vorlage.
pub fn render(conn: &Connection, template: &Template, hidden: &HashSet<String>) -> Result<Diagram> {
    render_body(conn, &template.body, hidden)
}

/// Zeichnet einen Vorlagentext, der noch nicht gespeichert ist.
///
/// Der Editor der Oberfläche benutzt das für seine Vorschau — sie läuft
/// dieselbe Rechnung wie das fertige Diagramm und kann deshalb nicht davon
/// abweichen.
pub fn render_body(conn: &Connection, body: &str, hidden: &HashSet<String>) -> Result<Diagram> {
    let meta = parse::meta(body)?;
    let (context, nodes) = rows::context(conn, &meta.sources, hidden)?;
    let mermaid = to_mermaid(parse::body(body), &meta, &context)?;
    Ok(Diagram {
        title: meta.title,
        mermaid,
        nodes,
    })
}

/// Zeichnet mit ausdrücklich übergebenen Zeilen, ohne Datenbank.
///
/// Damit vergleichen die Referenzfälle in `tests/fixtures/templates/` genau
/// das, was auch die Webapp bekommen hat.
pub fn render_rows(body: &str, context: &Context) -> Result<String> {
    let meta = parse::meta(body)?;
    to_mermaid(parse::body(body), &meta, context)
}

/// Der eigentliche Lauf: umschreiben, füllen, Stile anhängen.
fn to_mermaid(body: &str, meta: &Meta, context: &Context) -> Result<String> {
    let classes = helpers::ClassAssignments::default();
    let mut hb = Handlebars::new();
    helpers::register(&mut hb, classes.clone());

    let rewritten = rewrite::rewrite(body, &meta.styles, &meta.sources);
    hb.register_template_string("diagram", &rewritten)
        .map_err(|e| template_error(format!("Vorlage lässt sich nicht lesen: {e}")))?;

    let data = Value::Object(
        context
            .iter()
            .map(|(name, rows)| {
                let rows: Vec<Value> = rows
                    .iter()
                    .map(|row| {
                        Value::Object(
                            row.iter()
                                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                                .collect::<Map<_, _>>(),
                        )
                    })
                    .collect();
                (name.clone(), Value::Array(rows))
            })
            .collect::<Map<_, _>>(),
    );

    let mut diagram = hb
        .render("diagram", &data)
        .map_err(|e| template_error(format!("Vorlage scheitert beim Zeichnen: {e}")))?;

    // Die classDef-Zeilen kommen hinter die erste Zeile mit Inhalt — also
    // hinter `flowchart TD`, wo Mermaid sie erwartet.
    let defs = style::class_defs(&meta.styles);
    if !defs.is_empty() {
        let mut lines: Vec<&str> = diagram.split('\n').collect();
        let first = lines
            .iter()
            .position(|l| !l.trim().is_empty())
            .map_or(0, |i| i + 1);
        lines.insert(first, &defs);
        diagram = lines.join("\n");
    }

    // Und zum Schluss `class …`-Zeilen für jeden eingefärbten Knoten. Das
    // wirkt auch dann, wenn der Knoten zuerst ohne `:::klasse` auftauchte.
    let assignments = classes.lock().unwrap_or_else(|e| e.into_inner());
    if !assignments.is_empty() {
        let mut by_class: Vec<(String, Vec<String>)> = Vec::new();
        for (node, class) in assignments.iter() {
            match by_class.iter_mut().find(|(c, _)| c == class) {
                Some(entry) => entry.1.push(node.clone()),
                None => by_class.push((class.clone(), vec![node.clone()])),
            }
        }
        let lines: Vec<String> = by_class
            .iter()
            .map(|(class, nodes)| format!("class {} {class}", nodes.join(",")))
            .collect();
        diagram = format!("{}\n{}", diagram.trim_end(), lines.join("\n"));
    }
    Ok(diagram)
}

/// Ein Fehler in der Vorlage ist ein Eingabefehler — die Oberfläche zeigt ihn
/// am Textfeld, die Kommandozeile gibt Rückgabewert 4.
fn template_error(message: String) -> crate::Error {
    let mut v = Validator::new();
    v.add("body", message);
    v.finish().unwrap_err()
}
