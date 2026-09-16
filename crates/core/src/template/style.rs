//! Formen und Farben aus dem `styles`-Kopf einer Vorlage.
//!
//! Ein Eintrag heißt wie ein Attribut (`title`) oder wie ein Attribut einer
//! bestimmten Quelle (`Projekte.title`); der besondere Eintrag erbt vom
//! allgemeinen und überschreibt einzelne Angaben.

use serde_json::{Map, Value};

/// Klammern je Form — Mermaid unterscheidet Knotenformen über die Klammern.
pub const SHAPES: &[(&str, &str, &str)] = &[
    ("rectangle", "[\"", "\"]"),
    ("rounded", "(\"", "\")"),
    ("circle", "((\"", "\"))"),
    ("cylindrical", "[(\"", "\")]"),
    ("diamond", "{\"", "\"}"),
    ("stadium", "([\"", "\"])"),
];

pub fn brackets(shape: &str) -> (&'static str, &'static str) {
    let found = SHAPES.iter().find(|s| s.0 == shape).unwrap_or(&SHAPES[0]);
    (found.1, found.2)
}

/// Die `classDef`-Zeilen zu einem `styles`-Kopf, in dessen Reihenfolge.
///
/// Nur Farben stehen darin. Die Form steckt in den Klammern des Knotens und
/// nicht in einer CSS-Klasse: `rx` ist ein SVG-Attribut, und Mermaid verwirft
/// eine ganze `classDef`, in der so etwas steht.
pub fn class_defs(styles: &Map<String, Value>) -> String {
    let mut lines = Vec::new();
    for (key, entry) in styles {
        let Some(entry) = entry.as_object() else {
            continue;
        };
        let merged = merged_entry(styles, key, entry);
        let mut parts = Vec::new();
        if let Some(fill) = text(&merged, "fill") {
            parts.push(format!("fill:{fill}"));
        }
        if let Some(stroke) = text(&merged, "stroke") {
            parts.push(format!("stroke:{stroke}"));
        }
        if let Some(width) = merged.get("stroke-width").filter(|v| !v.is_null()) {
            parts.push(format!("stroke-width:{}px", scalar(width)));
        }
        if parts.is_empty() {
            continue;
        }
        lines.push(format!(
            "classDef cls_{} {}",
            key.replace('.', "_"),
            parts.join(",")
        ));
    }
    lines.join("\n")
}

/// `Projekte.title` erbt von `title`.
fn merged_entry(
    styles: &Map<String, Value>,
    key: &str,
    entry: &Map<String, Value>,
) -> Map<String, Value> {
    match key.find('.') {
        Some(i) => {
            let mut merged = styles
                .get(&key[i + 1..])
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            for (k, v) in entry {
                merged.insert(k.clone(), v.clone());
            }
            merged
        }
        None => entry.clone(),
    }
}

/// Was für ein Attribut gilt — allgemein, von der Quelle überschrieben.
pub fn for_attribute(
    styles: &Map<String, Value>,
    source: &str,
    attribute: &str,
) -> Option<(Map<String, Value>, bool)> {
    let specific = if source.is_empty() {
        None
    } else {
        styles
            .get(&format!("{source}.{attribute}"))
            .and_then(|v| v.as_object())
    };
    let general = styles.get(attribute).and_then(|v| v.as_object());
    if specific.is_none() && general.is_none() {
        return None;
    }
    let mut merged = general.cloned().unwrap_or_default();
    for (k, v) in specific.into_iter().flatten() {
        merged.insert(k.clone(), v.clone());
    }
    Some((merged, specific.is_some()))
}

pub fn text(entry: &Map<String, Value>, key: &str) -> Option<String> {
    entry
        .get(key)
        .filter(|v| !v.is_null() && *v != "")
        .map(scalar)
}

pub fn has_color(entry: &Map<String, Value>) -> bool {
    text(entry, "fill").is_some()
        || text(entry, "stroke").is_some()
        || entry.get("stroke-width").is_some_and(|v| !v.is_null())
}

/// Ein YAML-Skalar als Text — wie JavaScript ihn schreiben würde.
pub fn scalar(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}
