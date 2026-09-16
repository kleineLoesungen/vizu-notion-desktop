//! `{{title}}` → `{{nodeId "title" title source="Projekte"}}`.
//!
//! Der Umschreiber läuft **vor** Handlebars und ist der Grund, warum eine
//! Vorlage nie eine Knotenkennung von Hand vergeben muss: Aus einem
//! Attributnamen wird ein vollständiger Mermaid-Knoten `nXXXXXX["Wert"]`.
//!
//! Er arbeitet zeilenweise und merkt sich, in welchem `{{#each Quelle}}` er
//! gerade steht — daran hängt, welche Quelle in die Knotenkennung einfließt
//! und welcher `styles`-Eintrag gilt.
//!
//! Die Muster sind dieselben wie in `server/utils/templates.ts` der Webapp,
//! **einschließlich ihrer Eigenheiten**: Ein Quellname mit Bindestrich passt
//! nicht auf `[a-zA-Z_][a-zA-Z0-9_]*` und wird deshalb nicht als Quelle
//! erkannt. Das ist nicht schön, aber Bedingung dafür, dass vorhandene
//! Vorlagen dieselben Kennungen ergeben (docs/UMSETZUNG.md, 3.1).

use std::sync::LazyLock;

use regex::{Captures, Regex};
use serde_json::{Map, Value};

use super::style;

static EACH_OPEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\{\{#each\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\}\}").unwrap());
static EACH_SUBEXPR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\{\{#each\s+\((?:lookup-by|group)\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap()
});
static LOOKUP_OR_GROUP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\((lookup-by|group)\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap());
static JOIN_ROWS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\(join-rows\s+([a-zA-Z_][a-zA-Z0-9_]*)(\s+"(?:[^"\\]|\\.)*"\s+)([a-zA-Z_][a-zA-Z0-9_]*)"#)
        .unwrap()
});
static BARE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\}\}").unwrap());

/// Wörter, die Handlebars selbst gehören.
const KEYWORDS: [&str; 3] = ["else", "this", "log"];

pub fn rewrite(body: &str, styles: &Map<String, Value>, sources: &[String]) -> String {
    let known = |name: &str| sources.iter().any(|s| s == name);
    let mut current = String::new();
    let mut stack: Vec<String> = Vec::new();

    body.split('\n')
        .map(|line| {
            let trimmed = line.trim_start();

            if let Some(c) = EACH_OPEN.captures(trimmed) {
                stack.push(std::mem::replace(&mut current, c[1].to_string()));
                return line.to_string();
            }
            // Bei `{{#each (group Quelle …)}}` geht es weiter: Die Zeile
            // braucht unten noch die Ersetzung auf @root.
            if let Some(c) = EACH_SUBEXPR.captures(trimmed)
                && known(&c[1])
            {
                stack.push(std::mem::replace(&mut current, c[1].to_string()));
            }
            if trimmed.starts_with("{{/each}}") {
                current = stack.pop().unwrap_or_default();
                return line.to_string();
            }
            // In diesen Zeilen steht Mermaid-Syntax, keine Bindung.
            if trimmed.starts_with("classDef ") || trimmed.starts_with("subgraph ") {
                return line.to_string();
            }

            let line = if sources.is_empty() {
                line.to_string()
            } else {
                to_root(line, &known)
            };
            BARE.replace_all(&line, |c: &Captures| bind(&c[1], &current, styles))
                .into_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Quellnamen in Helferaufrufen auf `@root.Name` umstellen.
///
/// Handlebars sucht die Argumente eines Teilausdrucks sonst nur im aktuellen
/// Element — innerhalb eines `{{#each}}` fände es die Quelle nicht.
fn to_root(line: &str, known: &impl Fn(&str) -> bool) -> String {
    let line = LOOKUP_OR_GROUP.replace_all(line, |c: &Captures| {
        if known(&c[2]) {
            format!("({} @root.{}", &c[1], &c[2])
        } else {
            c[0].to_string()
        }
    });
    JOIN_ROWS
        .replace_all(&line, |c: &Captures| {
            let a = if known(&c[1]) {
                format!("@root.{}", &c[1])
            } else {
                c[1].to_string()
            };
            let b = if known(&c[3]) {
                format!("@root.{}", &c[3])
            } else {
                c[3].to_string()
            };
            format!("(join-rows {a}{}{b}", &c[2])
        })
        .into_owned()
}

/// Aus `{{feld}}` wird der Aufruf des Helfers `nodeId` — mit Form, Klasse und
/// Quelle, soweit die Vorlage etwas dazu sagt.
fn bind(name: &str, source: &str, styles: &Map<String, Value>) -> String {
    if KEYWORDS.contains(&name) {
        return format!("{{{{{name}}}}}");
    }
    let source_part = if source.is_empty() {
        String::new()
    } else {
        format!(" source=\"{source}\"")
    };

    let Some((merged, source_specific)) = style::for_attribute(styles, source, name) else {
        return format!("{{{{nodeId \"{name}\" {name}{source_part}}}}}");
    };
    let shape_part = match style::text(&merged, "shape") {
        Some(shape) => format!(" shape=\"{shape}\""),
        None => String::new(),
    };
    let class_part = if style::has_color(&merged) {
        let class = if source_specific {
            format!("cls_{source}_{name}")
        } else {
            format!("cls_{name}")
        };
        format!(" className=\"{class}\"")
    } else {
        String::new()
    };
    format!("{{{{nodeId \"{name}\" {name}{shape_part}{class_part}{source_part}}}}}")
}
