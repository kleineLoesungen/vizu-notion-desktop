//! Eine Vorlage aus einer Auswahl bauen — der Assistent im Editor.
//!
//! Statt Handlebars-Muster zu kennen, sagt man, was man sehen will:
//! Diagrammart, Quellen, und je Art ein paar Rollen („Pfeile entlang `next`",
//! „ein Stück je Wert von `status`"). Heraus kommt eine gewöhnliche Vorlage,
//! die man danach von Hand weiterbearbeiten kann.
//!
//! **Die Auswahl steht im Kopf der Vorlage** (`assistant: {…}`, als JSON —
//! das ist gültiges YAML). So öffnet „Im Assistenten ändern" sie wieder, und
//! [`assistant`] merkt, ob die Vorlage seither von Hand geändert wurde.
//!
//! Jede Knotenkennung nennt ihre Quelle ausdrücklich (`{{node title "A"}}`),
//! statt sie dem Umschreiber zu überlassen: Der kennt die Quelle innerhalb von
//! `join-rows` nicht, und Pfeile zwischen zwei Quellen trafen sonst Knoten,
//! die es nicht gibt. Jede Kombination wird in `crates/core/tests/compose.rs`
//! mit echten Daten gezeichnet und von mermaid.js gelesen.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use super::helpers::label_text;
use super::parse;
use crate::error::{Result, Validator};

/// Die Diagrammarten des Assistenten.
pub const KINDS: [&str; 4] = ["flowchart", "pie", "gantt", "mindmap"];

/// Was im Assistenten gewählt wurde. Welche Felder zählen, hängt an `kind`;
/// die übrigen bleiben leer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Spec {
    /// `flowchart`, `pie`, `gantt` oder `mindmap`.
    pub kind: String,
    pub title: String,
    /// Eine oder mehrere Quellen, in der gewählten Reihenfolge.
    pub sources: Vec<SourcePart>,
    /// Rahmen (Flowchart), Stück (Pie), Abschnitt (Gantt), Zweig (Mindmap)
    /// je Wert dieser Rolle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Flowchart: ein Rahmen je Quelle. Mit `group` zusammen: je Quelle, darin
    /// je Wert.
    #[serde(default, skip_serializing_if = "is_false")]
    pub by_source: bool,
    /// Flowchart: eine Farbe je Wert dieser Rolle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Flowchart: eine Farbe je Quelle.
    #[serde(default, skip_serializing_if = "is_false")]
    pub color_by_source: bool,
    /// Gantt: das Datum — Anfang, bei einem Zeitraum auch das Ende.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// Pie: die Summe dieses Zahlenfelds statt der Anzahl der Seiten.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sum: Option<String>,
}

/// Eine Quelle im Assistenten.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SourcePart {
    pub name: String,
    /// Flowchart: Pfeile entlang dieser Rolle …
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// … zu den Seiten dieser Quelle. Leer: dieselbe Quelle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_to: Option<String>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl SourcePart {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }
}

/// Die Auswahl einer Vorlage, falls sie aus dem Assistenten stammt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct AssistantState {
    pub spec: Spec,
    /// Wurde die Vorlage seither von Hand geändert? Dann ersetzt ein neuer Lauf
    /// des Assistenten diese Änderungen.
    pub edited: bool,
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

/// Die Auswahl aus dem Kopf einer Vorlage — `None`, wenn sie nicht aus dem
/// Assistenten stammt.
pub fn assistant(body: &str) -> Option<AssistantState> {
    let (yaml, _) = parse::split(body);
    let data: Value = serde_saphyr::from_str(yaml?).ok()?;
    let spec: Spec = serde_json::from_value(data.get("assistant")?.clone()).ok()?;
    let edited = match compose(&spec) {
        Ok(fresh) => fresh.trim_end() != body.trim_end(),
        Err(_) => true,
    };
    Some(AssistantState { spec, edited })
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
    let title = label_text(&spec.title);
    v.require(!title.is_empty(), "title", "darf nicht leer sein");

    let names: Vec<&str> = spec.sources.iter().map(|s| s.name.trim()).collect();
    v.require(!names.is_empty(), "sources", "mindestens eine Quelle");
    for (i, name) in names.iter().enumerate() {
        if !usable_source_name(name) {
            v.add(
                "sources",
                format!(
                    "„{name}\u{201c} taugt nicht für Vorlagen — nur Buchstaben, Ziffern und _. \
                     Die Quelle lässt sich unter „Quellen\u{201c} umbenennen."
                ),
            );
        } else if names[..i].contains(name) {
            v.add("sources", format!("„{name}\u{201c} ist zweimal gewählt"));
        }
    }

    // Rollen landen als Bezeichner in der Vorlage — was keiner ist, wäre dort
    // Syntax statt Feld.
    let mut roles = vec![
        ("group", &spec.group),
        ("color", &spec.color),
        ("date", &spec.date),
        ("sum", &spec.sum),
    ];
    for part in &spec.sources {
        roles.push(("link", &part.link));
    }
    for (field, role) in roles {
        if let Some(role) = chosen(role)
            && !identifier(role)
        {
            v.add(field, format!("„{role}\u{201c} ist keine Rolle"));
        }
    }
    for part in &spec.sources {
        if let Some(target) = chosen(&part.link_to)
            && !names.contains(&target)
        {
            v.add(
                "link",
                format!(
                    "Pfeile von „{}\u{201c} zu „{target}\u{201c}: „{target}\u{201c} muss als Quelle dabei sein",
                    part.name
                ),
            );
        }
    }

    match kind {
        "pie" => v.require(
            names.len() > 1 || chosen(&spec.group).is_some(),
            "group",
            "ein Kreisdiagramm aus einer Quelle braucht ein Feld, dessen Werte die Stücke sind",
        ),
        "gantt" => v.require(
            chosen(&spec.date).is_some(),
            "date",
            "ein Gantt-Diagramm braucht ein Datum",
        ),
        _ => {}
    }
    v.finish()?;

    // Die Auswahl als JSON in den Kopf — JSON ist gültiges YAML.
    let recorded = serde_json::to_string(spec).expect("Spec ist serialisierbar");
    let mut head = format!("---\ntitle: \"{title}\"\nsources:\n");
    for name in &names {
        head.push_str(&format!("  - {name}\n"));
    }
    head.push_str(&format!("assistant: {recorded}\n---\n"));

    let body = match kind {
        "flowchart" => flowchart(spec, &names),
        "pie" => pie(spec, &names, &title),
        "gantt" => gantt(spec, &names, &title),
        _ => mindmap(spec, &names, &title),
    };
    Ok(head + &body)
}

fn flowchart(spec: &Spec, names: &[&str]) -> String {
    let mut out = String::from("flowchart LR\n");
    let class = |source: &str| -> String {
        if spec.color_by_source {
            format!(":::{{{{valueClass \"{source}\"}}}}")
        } else if let Some(c) = chosen(&spec.color) {
            format!(":::{{{{valueClass {c}}}}}")
        } else {
            String::new()
        }
    };

    for &source in names {
        let class = class(source);
        if spec.by_source {
            out.push_str(&format!("  subgraph q-{source} [\"{source}\"]\n"));
        }
        match chosen(&spec.group) {
            Some(g) => out.push_str(&format!(
                "{{{{#each (group-pages {source} \"{g}\")}}}}\n  subgraph {{{{valueClass {g}}}}}-{source} [\"{{{{label {g} \"(leer)\"}}}}\"]\n  {{{{#group-item}}}}\n    {{{{node title \"{source}\"}}}}{class}\n  {{{{/group-item}}}}\n  end\n{{{{/each}}}}\n"
            )),
            None => out.push_str(&format!(
                "{{{{#each (unique {source})}}}}\n  {{{{node title \"{source}\"}}}}{class}\n{{{{/each}}}}\n"
            )),
        }
        if spec.by_source {
            out.push_str("  end\n");
        }
    }

    // Pfeile zuletzt: Jede Kennung nennt ihre Quelle, also treffen sie die
    // Kästen von oben — auch in Rahmen und über Quellen hinweg.
    for part in &spec.sources {
        let Some(link) = chosen(&part.link) else {
            continue;
        };
        let source = part.name.trim();
        match chosen(&part.link_to).filter(|t| *t != source) {
            None => out.push_str(&format!(
                "{{{{#each {source}}}}}\n  {{{{#if {link}}}}}{{{{node title \"{source}\"}}}} --> {{{{node {link} \"{source}\"}}}}{{{{/if}}}}\n{{{{/each}}}}\n"
            )),
            Some(target) => out.push_str(&format!(
                "{{{{#each (join-rows {source} \"{link}\" {target} \"title\" \"z\")}}}}\n{{{{#if z_title}}}}  {{{{node title \"{source}\"}}}} --> {{{{node z_title \"{target}\"}}}}\n{{{{/if}}}}\n{{{{/each}}}}\n"
            )),
        }
    }
    out
}

fn pie(spec: &Spec, names: &[&str], title: &str) -> String {
    let mut out = format!("pie showData\n  title {title}\n");
    let several = names.len() > 1;
    for &source in names {
        match chosen(&spec.group) {
            Some(g) => {
                let size = match chosen(&spec.sum) {
                    Some(f) => format!("{{{{sum items \"{f}\"}}}}"),
                    None => "{{len items}}".to_string(),
                };
                // Mit mehreren Quellen trägt jedes Stück seine Quelle im Namen —
                // sonst stünden zwei „Done" nebeneinander.
                let suffix = if several {
                    format!(" · {source}")
                } else {
                    String::new()
                };
                out.push_str(&format!(
                    "{{{{#each (group-pages {source} \"{g}\")}}}}\n  \"{{{{label {g} \"(leer)\"}}}}{suffix}\" : {size}\n{{{{/each}}}}\n"
                ));
            }
            None => {
                let size = match chosen(&spec.sum) {
                    Some(f) => format!("{{{{sum {source} \"{f}\"}}}}"),
                    None => format!("{{{{len (unique {source})}}}}"),
                };
                out.push_str(&format!("  \"{source}\" : {size}\n"));
            }
        }
    }
    out
}

fn gantt(spec: &Spec, names: &[&str], title: &str) -> String {
    let date = chosen(&spec.date).unwrap_or("date");
    // Ein Zeitraum wird ein Balken, ein einzelner Tag ein Balken von einem Tag.
    let task = format!(
        "{{{{#if {date}}}}}  {{{{label title \"(ohne Titel)\"}}}} : {{{{day {date}}}}}, {{{{#if {date}_end}}}}{{{{day {date}_end}}}}{{{{else}}}}1d{{{{/if}}}}\n{{{{/if}}}}"
    );
    let mut out =
        format!("gantt\n  title {title}\n  dateFormat YYYY-MM-DD\n  axisFormat %d.%m.%Y\n");
    let several = names.len() > 1;
    for &source in names {
        match chosen(&spec.group) {
            Some(g) => {
                let suffix = if several {
                    format!(" · {source}")
                } else {
                    String::new()
                };
                out.push_str(&format!(
                    "{{{{#each (group-pages {source} \"{g}\")}}}}\n  section {{{{label {g} \"(ohne)\"}}}}{suffix}\n{{{{#group-item}}}}\n{task}\n{{{{/group-item}}}}\n{{{{/each}}}}\n"
                ));
            }
            None => out.push_str(&format!(
                "  section {source}\n{{{{#each (unique {source})}}}}\n{task}\n{{{{/each}}}}\n"
            )),
        }
    }
    out
}

fn mindmap(spec: &Spec, names: &[&str], title: &str) -> String {
    let mut out = format!("mindmap\n  root(({title}))\n");
    // Mit mehreren Quellen ist jede Quelle ein eigener Zweig — eine Ebene tiefer.
    let several = names.len() > 1;
    let indent = |level: usize| " ".repeat(2 + 2 * level + if several { 2 } else { 0 });
    for &source in names {
        if several {
            out.push_str(&format!("    {source}\n"));
        }
        let (branch, leaf) = (indent(1), indent(2));
        match chosen(&spec.group) {
            Some(g) => out.push_str(&format!(
                "{{{{#each (group-pages {source} \"{g}\")}}}}\n{branch}{{{{label {g} \"(leer)\"}}}}\n{{{{#group-item}}}}\n{leaf}{{{{label title \"(ohne Titel)\"}}}}\n{{{{/group-item}}}}\n{{{{/each}}}}\n"
            )),
            None => out.push_str(&format!(
                "{{{{#each (unique {source})}}}}\n{branch}{{{{label title \"(ohne Titel)\"}}}}\n{{{{/each}}}}\n"
            )),
        }
    }
    out
}
