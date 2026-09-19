//! Eine Vorlage aus einer Auswahl bauen — der Assistent im Editor.
//!
//! Statt Handlebars-Muster zu kennen, sagt man, was man sehen will:
//! Diagrammart, Quelle, und je Art ein paar Rollen („Pfeile entlang `next`",
//! „ein Stück je Wert von `status`"). Heraus kommt eine gewöhnliche Vorlage,
//! die man danach von Hand weiterbearbeiten kann.
//!
//! Die Vorlagen benutzen die Helfer aus [`super::helpers`], die die Webapp
//! nicht kannte — `node`, `valueClass`, `label`, `day`, `sum` —, weil sie
//! sonst an Rahmen oder Sonderzeichen scheiterten. Jede Kombination wird in
//! `crates/core/tests/compose.rs` mit echten Daten gezeichnet und von
//! mermaid.js gelesen.

use serde::Deserialize;
use ts_rs::TS;

use crate::error::{Result, Validator};

/// Die Diagrammarten des Assistenten.
pub const KINDS: [&str; 4] = ["flowchart", "pie", "gantt", "mindmap"];

/// Was im Assistenten gewählt wurde. Welche Felder zählen, hängt an `kind`;
/// die übrigen bleiben leer.
#[derive(Debug, Clone, Default, Deserialize, TS)]
pub struct Spec {
    /// `flowchart`, `pie`, `gantt` oder `mindmap`.
    pub kind: String,
    pub title: String,
    pub source: String,
    /// Flowchart: Pfeile entlang dieser Rolle — einer Relation wie `next`.
    #[serde(default)]
    pub link: Option<String>,
    /// Flowchart: ein Rahmen je Wert. Pie: ein Stück je Wert. Gantt: ein
    /// Abschnitt je Wert. Mindmap: ein Zweig je Wert.
    #[serde(default)]
    pub group: Option<String>,
    /// Pie: die Summe dieses Zahlenfelds statt der Anzahl der Seiten.
    #[serde(default)]
    pub sum: Option<String>,
    /// Flowchart: eine Farbe je Wert dieser Rolle, aus der Palette.
    #[serde(default)]
    pub color: Option<String>,
    /// Gantt: das Datum — Anfang, bei einem Zeitraum auch das Ende.
    #[serde(default)]
    pub date: Option<String>,
}

/// Taugt ein Quellname für eine Vorlage?
///
/// `{{#each Name}}` erkennt nur Buchstaben, Ziffern und `_`, vorn keine
/// Ziffer — so war es in der Webapp, und daran hängt, dass vorhandene
/// Vorlagen dieselben Kennungen ergeben. „vizu Roadmap" mit Leerzeichen
/// taugt deshalb für Fluss und Metro, aber nicht für eine Vorlage.
pub fn usable_source_name(name: &str) -> bool {
    identifier(name)
}

fn identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Eine Rolle aus dem Assistenten: leer heißt „nicht gewählt".
fn chosen(role: &Option<String>) -> Option<&str> {
    role.as_deref().map(str::trim).filter(|r| !r.is_empty())
}

/// Baut die Vorlage. Was nicht passt, ist ein Eingabefehler am Feld.
pub fn compose(spec: &Spec) -> Result<String> {
    let mut v = Validator::new();

    let kind = spec.kind.trim();
    v.require(
        KINDS.contains(&kind),
        "kind",
        format!("unbekannte Diagrammart — erlaubt sind {}", KINDS.join(", ")),
    );
    let title = super::helpers::label_text(&spec.title);
    v.require(!title.is_empty(), "title", "darf nicht leer sein");

    let source = spec.source.trim();
    if source.is_empty() {
        v.add("source", "fehlt");
    } else if !usable_source_name(source) {
        v.add(
            "source",
            format!(
                "„{source}\u{201c} taugt nicht für Vorlagen — nur Buchstaben, Ziffern und _. \
                 Die Quelle lässt sich unter „Quellen\u{201c} umbenennen."
            ),
        );
    }

    // Rollen landen als Bezeichner in der Vorlage — was keiner ist, wäre dort
    // Syntax statt Feld.
    for (field, role) in [
        ("link", &spec.link),
        ("group", &spec.group),
        ("sum", &spec.sum),
        ("color", &spec.color),
        ("date", &spec.date),
    ] {
        if let Some(role) = chosen(role)
            && !identifier(role)
        {
            v.add(field, format!("„{role}\u{201c} ist keine Rolle"));
        }
    }

    match kind {
        "pie" => v.require(
            chosen(&spec.group).is_some(),
            "group",
            "ein Kreisdiagramm braucht ein Feld, dessen Werte die Stücke sind",
        ),
        "gantt" => v.require(
            chosen(&spec.date).is_some(),
            "date",
            "ein Gantt-Diagramm braucht ein Datum",
        ),
        _ => {}
    }
    v.finish()?;

    let head = format!("---\ntitle: \"{title}\"\nsources:\n  - {source}\n---\n");
    let body = match kind {
        "flowchart" => flowchart(spec, source),
        "pie" => pie(spec, source, &title),
        "gantt" => gantt(spec, source, &title),
        _ => mindmap(spec, source, &title),
    };
    Ok(head + &body)
}

fn flowchart(spec: &Spec, source: &str) -> String {
    let mut out = String::from("flowchart LR\n");
    let color = chosen(&spec.color);
    let class = color
        .map(|c| format!(":::{{{{valueClass {c}}}}}"))
        .unwrap_or_default();

    if let Some(c) = color {
        out.push_str(&format!(
            "{{{{#each (group {source} \"{c}\")}}}}\n  classDef {{{{valueClass {c}}}}} fill:{{{{palette @index}}}},color:#fff\n{{{{/each}}}}\n"
        ));
    }

    match chosen(&spec.group) {
        Some(g) => out.push_str(&format!(
            "{{{{#each (group-pages {source} \"{g}\")}}}}\n  subgraph {{{{valueClass {g}}}}} [\"{{{{label {g} \"(leer)\"}}}}\"]\n  {{{{#group-item}}}}\n    {{{{node title \"{source}\"}}}}{class}\n  {{{{/group-item}}}}\n  end\n{{{{/each}}}}\n"
        )),
        None => out.push_str(&format!(
            "{{{{#each {source}}}}}\n  {{{{title}}}}{class}\n{{{{/each}}}}\n"
        )),
    }

    if let Some(l) = chosen(&spec.link) {
        out.push_str(&format!(
            "{{{{#each {source}}}}}\n  {{{{#if {l}}}}}{{{{title}}}} --> {{{{{l}}}}}{{{{/if}}}}\n{{{{/each}}}}\n"
        ));
    }
    out
}

fn pie(spec: &Spec, source: &str, title: &str) -> String {
    let group = chosen(&spec.group).unwrap_or("status");
    let size = match chosen(&spec.sum) {
        Some(f) => format!("{{{{sum items \"{f}\"}}}}"),
        None => "{{len items}}".to_string(),
    };
    format!(
        "pie showData\n  title {title}\n{{{{#each (group-pages {source} \"{group}\")}}}}\n  \"{{{{label {group} \"(leer)\"}}}}\" : {size}\n{{{{/each}}}}\n"
    )
}

fn gantt(spec: &Spec, source: &str, title: &str) -> String {
    let date = chosen(&spec.date).unwrap_or("date");
    // Ein Zeitraum wird ein Balken, ein einzelner Tag ein Balken von einem Tag.
    let task = format!(
        "{{{{#if {date}}}}}  {{{{label title \"(ohne Titel)\"}}}} : {{{{day {date}}}}}, {{{{#if {date}_end}}}}{{{{day {date}_end}}}}{{{{else}}}}1d{{{{/if}}}}\n{{{{/if}}}}"
    );
    let mut out =
        format!("gantt\n  title {title}\n  dateFormat YYYY-MM-DD\n  axisFormat %d.%m.%Y\n");
    match chosen(&spec.group) {
        Some(g) => out.push_str(&format!(
            "{{{{#each (group-pages {source} \"{g}\")}}}}\n  section {{{{label {g} \"(ohne)\"}}}}\n{{{{#group-item}}}}\n{task}\n{{{{/group-item}}}}\n{{{{/each}}}}\n"
        )),
        None => out.push_str(&format!(
            "  section {source}\n{{{{#each (unique {source})}}}}\n{task}\n{{{{/each}}}}\n"
        )),
    }
    out
}

fn mindmap(spec: &Spec, source: &str, title: &str) -> String {
    let mut out = format!("mindmap\n  root(({title}))\n");
    match chosen(&spec.group) {
        Some(g) => out.push_str(&format!(
            "{{{{#each (group-pages {source} \"{g}\")}}}}\n    {{{{label {g} \"(leer)\"}}}}\n{{{{#group-item}}}}\n      {{{{label title \"(ohne Titel)\"}}}}\n{{{{/group-item}}}}\n{{{{/each}}}}\n"
        )),
        None => out.push_str(&format!(
            "{{{{#each (unique {source})}}}}\n    {{{{label title \"(ohne Titel)\"}}}}\n{{{{/each}}}}\n"
        )),
    }
    out
}
