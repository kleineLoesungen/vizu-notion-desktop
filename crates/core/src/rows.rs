//! Von Notion-Seiten zu flachen Zeilen, wie eine Vorlage sie sieht.
//!
//! ```text
//! Page { properties: { "Name": {title:[…]}, "Ziel": {relation:[…]} } }
//!   │  Spaltenzuordnung der Quelle (title → "Name", parent → "Ziel")
//!   ▼
//! Row  { id: "…", title: "Website", parent: "Wachstum" }
//! ```
//!
//! Drei Dinge passieren dabei:
//!
//! 1. **Werte werden Text.** Ein Datum wird sein Anfangstag, ein Select sein
//!    Name, eine Mehrfachauswahl `"A, B"` — wie in der Webapp, damit alte
//!    Vorlagen dasselbe ergeben.
//! 2. **Relationen werden Titel.** Aus Seiten-IDs werden Namen: erst aus den
//!    Seiten der Quellen, sonst aus `page_titles` (beim Abruf gefüllt).
//! 3. **Mehrfachrelationen werden zu mehreren Zeilen.** Ein Projekt mit zwei
//!    Zielen ergibt zwei Zeilen — so entsteht aus `{{title}} --> {{parent}}`
//!    je Ziel eine Kante, ohne dass die Vorlage etwas davon wissen muss.
//!
//! Alles hier ist **reine Rechnung** auf schon abgerufenen Daten: kein Netz,
//! keine Datenbank. Gelesen wird über [`context`].

use std::collections::{BTreeMap, HashSet};

use rusqlite::Connection;
use serde_json::Value;

use crate::error::Result;
use crate::notion::Page;
use crate::source::Source;
use crate::{fetch, source};

/// Eine Zeile: Rollenname → Text. `id` ist immer dabei.
pub type Row = BTreeMap<String, String>;

/// Was eine Vorlage zu sehen bekommt: Quellname → Zeilen.
pub type Context = BTreeMap<String, Vec<Row>>;

/// Ein Knoten für das Filterfeld — jede Zeile, auch ausgeblendete.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
pub struct NodeInfo {
    pub id: String,
    pub title: String,
    pub source: String,
    /// Seiten-IDs, auf die diese Seite über irgendeine Relation zeigt.
    pub relations: Vec<String>,
}

/// Die Zeilen aller genannten Quellen aus dem Zwischenspeicher.
///
/// `hidden` sind Seiten-IDs, die nicht in den Kontext kommen — weder als
/// eigene Zeile noch als Ziel einer Relation. Die Knotenliste enthält sie
/// trotzdem, damit das Filterfeld sie wieder einblenden kann.
pub fn context(
    conn: &Connection,
    source_names: &[String],
    hidden: &HashSet<String>,
) -> Result<(Context, Vec<NodeInfo>)> {
    let all = source::list(conn)?;
    let mut pages_by_source = Vec::new();
    for name in source_names {
        let source = all
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| unknown_source(name, &all))?;
        pages_by_source.push((source.clone(), fetch::pages(conn, source.id)?));
    }

    // Titel aller Seiten, die eine Relation treffen könnte: die aller
    // Quellen — auch der nicht gezeichneten — und die beim Abruf mitgeholten
    // Ziele außerhalb.
    let titles = known_titles(conn)?;

    let mut context = Context::new();
    let mut nodes = Vec::new();
    for (source, pages) in &pages_by_source {
        let mut rows = Vec::new();
        for page in pages {
            nodes.push(NodeInfo {
                id: page.id.clone(),
                title: page_title(page, source),
                source: source.name.clone(),
                relations: relation_targets(page),
            });
            if hidden.contains(&page.id) {
                continue;
            }
            rows.extend(expand(row(page, source, &titles, hidden)));
        }
        context.insert(source.name.clone(), rows);
    }
    Ok((context, nodes))
}

fn unknown_source(name: &str, all: &[Source]) -> crate::Error {
    let known: Vec<&str> = all.iter().map(|s| s.name.as_str()).collect();
    let mut v = crate::error::Validator::new();
    v.add(
        "sources",
        format!(
            "Quelle „{name}\u{201c} gibt es nicht (vorhanden: {})",
            if known.is_empty() {
                "keine".to_string()
            } else {
                known.join(", ")
            }
        ),
    );
    v.finish().unwrap_err()
}

/// Seiten-ID → Titel, aus allen Quellen und aus `page_titles`.
fn known_titles(conn: &Connection) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    // Zuerst die fremden, dann die eigenen: Kennt eine Quelle die Seite
    // selbst, gilt deren Titel.
    for sql in [
        "SELECT page_id, title FROM page_titles",
        "SELECT page_id, title FROM pages",
    ] {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (id, title) = row?;
            out.insert(id, title);
        }
    }
    Ok(out)
}

/// Der Titel einer Seite: die zugeordnete Titelspalte, sonst die erste
/// Spalte vom Typ `title`.
pub fn page_title(page: &Page, source: &Source) -> String {
    if let Some(property) = source.property("title")
        && let Some(value) = page.properties.get(property)
    {
        let text = text_of(value);
        if !text.is_empty() {
            return text;
        }
    }
    page.properties
        .values()
        .find(|v| v["type"] == "title")
        .map(text_of)
        .unwrap_or_default()
}

/// Alle Seiten-IDs, auf die diese Seite zeigt — für „verwandte Knoten".
pub fn relation_targets(page: &Page) -> Vec<String> {
    let mut out = Vec::new();
    for value in page.properties.values() {
        if value["type"] != "relation" {
            continue;
        }
        for target in value["relation"].as_array().into_iter().flatten() {
            if let Some(id) = target["id"].as_str() {
                out.push(id.to_string());
            }
        }
    }
    out
}

/// Eine Seite als Zeile. Mehrfachrelationen stehen zunächst unter
/// `<rolle>\0all`, [`expand`] macht daraus mehrere Zeilen.
fn row(
    page: &Page,
    source: &Source,
    titles: &BTreeMap<String, String>,
    hidden: &HashSet<String>,
) -> (Row, BTreeMap<String, Vec<String>>) {
    let mut row = Row::new();
    let mut multi = BTreeMap::new();
    row.insert("id".to_string(), page.id.clone());

    for mapping in &source.mappings {
        let value = page.properties.get(&mapping.property);
        let Some(value) = value else {
            // Die Spalte gibt es nicht (mehr). Beim Abruf wäre das ein
            // Eingabefehler gewesen; hier ist ein leerer Wert die
            // freundlichere Antwort als ein Abbruch beim Zeichnen.
            row.insert(mapping.role.clone(), String::new());
            continue;
        };
        if value["type"] == "relation" {
            let names: Vec<String> = value["relation"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|t| t["id"].as_str())
                .filter(|id| !hidden.contains(*id))
                .map(|id| titles.get(id).cloned().unwrap_or_else(|| id.to_string()))
                .collect();
            row.insert(
                mapping.role.clone(),
                names.first().cloned().unwrap_or_default(),
            );
            if names.len() > 1 {
                multi.insert(mapping.role.clone(), names);
            }
        } else {
            row.insert(mapping.role.clone(), text_of(value));
        }
    }
    (row, multi)
}

/// Kreuzt die Mehrfachrelationen aus: zwei Ziele × zwei Nachfolger = vier Zeilen.
fn expand((base, multi): (Row, BTreeMap<String, Vec<String>>)) -> Vec<Row> {
    let mut rows = vec![base];
    for (role, values) in multi {
        let mut next = Vec::with_capacity(rows.len() * values.len());
        for row in rows {
            for value in &values {
                let mut copy = row.clone();
                copy.insert(role.clone(), value.clone());
                next.push(copy);
            }
        }
        rows = next;
    }
    rows
}

/// Eine Notion-Eigenschaft als Text.
///
/// Gegenüber der Webapp behoben (siehe docs/UMSETZUNG.md 3.1): Text aus
/// mehreren Stücken wird verbunden statt abgeschnitten, und `status`,
/// `people`, `formula`, `rollup`, `unique_id` sowie die Zeitstempel liefern
/// einen Wert statt einer leeren Zeichenkette.
pub fn text_of(value: &Value) -> String {
    let kind = value["type"].as_str().unwrap_or_default();
    let field = &value[kind];
    match kind {
        "title" | "rich_text" => rich_text(field),
        "select" | "status" => field["name"].as_str().unwrap_or_default().to_string(),
        "multi_select" => join(field, |o| o["name"].as_str().map(String::from)),
        "people" => join(field, |p| p["name"].as_str().map(String::from)),
        "files" => join(field, |f| f["name"].as_str().map(String::from)),
        "date" => field["start"].as_str().unwrap_or_default().to_string(),
        "checkbox" => field.as_bool().unwrap_or(false).to_string(),
        "number" => number(field),
        "url" | "email" | "phone_number" | "created_time" | "last_edited_time" => {
            field.as_str().unwrap_or_default().to_string()
        }
        "unique_id" => match (field["prefix"].as_str(), field["number"].as_i64()) {
            (Some(prefix), Some(n)) => format!("{prefix}-{n}"),
            (None, Some(n)) => n.to_string(),
            _ => String::new(),
        },
        // Formel und Rollup tragen ihren eigenen Typ in sich.
        "formula" | "rollup" => match field["type"].as_str().unwrap_or_default() {
            "string" => field["string"].as_str().unwrap_or_default().to_string(),
            "number" => number(&field["number"]),
            "boolean" => field["boolean"].as_bool().unwrap_or(false).to_string(),
            "date" => field["date"]["start"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            "array" => join(&field["array"], |v| {
                Some(text_of(v)).filter(|t| !t.is_empty())
            }),
            _ => String::new(),
        },
        // relation wird über die Titel aufgelöst, nicht hier.
        _ => String::new(),
    }
}

fn rich_text(value: &Value) -> String {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|part| part["plain_text"].as_str())
        .collect()
}

fn join(value: &Value, pick: impl Fn(&Value) -> Option<String>) -> String {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(pick)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Zahlen wie JavaScript: `3` statt `3.0`, aber `3.5` bleibt `3.5`.
fn number(value: &Value) -> String {
    match value.as_f64() {
        Some(n) if n.fract() == 0.0 && n.abs() < 1e15 => format!("{}", n as i64),
        Some(n) => format!("{n}"),
        None => String::new(),
    }
}
