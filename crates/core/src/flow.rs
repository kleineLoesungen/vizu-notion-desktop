//! Das Flussdiagramm: Knoten und Kanten aus einer Relation, fertig angeordnet.
//!
//! ```text
//! Zeilen einer Quelle ──next──▶ Kanten
//!                     │
//!                     ▼  Ebenen nach Kahn: jeder Knoten hinter seine Vorgänger
//!               FlowGraph { nodes mit x/y, edges }
//! ```
//!
//! **Die Anordnung steht hier, nicht in der Oberfläche.** Sonst hätte die
//! Kommandozeile keine (`vizu-notion flow --json` gibt dieselben Zahlen aus),
//! und zwei Schalen rechneten verschieden. Gezeichnet wird in `ui/` — dort
//! stehen Farben und Schrift.
//!
//! Anders als bei einer Vorlage ist hier **jede Seite ein eigener Knoten**,
//! auch bei gleichem Titel: Der Graph kommt aus den Relationen, und die kennen
//! Seiten, keine Texte.

use std::collections::{BTreeMap, HashSet};

use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::error::Result;
use crate::rows::{self, NodeInfo};
use crate::source::Source;
use crate::{fetch, source};

/// Breite eines Knotens in Punkten. Die Oberfläche zeichnet damit, die
/// Anordnung rechnet damit — deshalb steht die Zahl hier.
pub const NODE_WIDTH: f32 = 180.0;
pub const NODE_HEIGHT: f32 = 56.0;
/// Abstand zwischen zwei Knoten derselben Ebene.
const GAP_X: f32 = 40.0;
/// Abstand zwischen zwei Ebenen.
const GAP_Y: f32 = 72.0;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct FlowNode {
    /// Die Seiten-ID — dieselbe wie im Filterfeld.
    pub id: String,
    pub title: String,
    /// Ein zweiter Wert unter dem Titel, etwa der Status. Leer, wenn keiner
    /// gewählt ist.
    pub subtitle: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct FlowEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct FlowGraph {
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
    /// Größe der Zeichnung in Punkten — die Oberfläche passt danach ein.
    pub width: f32,
    pub height: f32,
    /// Jede Seite der Quelle, auch die ausgeblendeten: das Filterfeld.
    pub all_nodes: Vec<NodeInfo>,
    /// Rollen, die als zweite Zeile taugen — für die Auswahl in der Oberfläche.
    pub subtitle_roles: Vec<String>,
}

/// Eine Quelle taugt als Fluss, sobald sie eine Rolle `next` hat.
pub fn eligible(source: &Source) -> bool {
    source.property("next").is_some()
}

/// Baut den Graphen aus dem Zwischenspeicher — ohne Netz.
///
/// `subtitle` ist die Rolle, die unter dem Titel steht (`status`, `date`, …).
pub fn build(
    conn: &Connection,
    source_id: Uuid,
    hidden: &HashSet<String>,
    subtitle: Option<&str>,
) -> Result<FlowGraph> {
    let source = source::get(conn, source_id)?;
    let pages = fetch::pages(conn, source_id)?;

    let known = rows::known_titles(conn)?;
    let mut all_nodes = Vec::new();
    let mut titles: BTreeMap<String, String> = BTreeMap::new();
    for page in &pages {
        let node = rows::node_info(page, &source, &known);
        titles.insert(page.id.clone(), node.title.clone());
        all_nodes.push(node);
    }

    // Kanten entstehen nur aus der Rolle `next` — `parent` zeigt auf eine
    // andere Datenbank und gehört in die Metro-Karte, nicht hierher.
    let next_property = source.property("next");
    let mut edges = Vec::new();
    let mut visible = Vec::new();
    for page in &pages {
        if hidden.contains(&page.id) {
            continue;
        }
        visible.push(page);
        let Some(property) = next_property else {
            continue;
        };
        let value = page.properties.get(property);
        for target in value
            .map(|v| v["relation"].as_array().cloned().unwrap_or_default())
            .unwrap_or_default()
        {
            let Some(target) = target["id"].as_str() else {
                continue;
            };
            // Ziele außerhalb der Quelle oder ausgeblendete zeichnen nicht mit.
            if hidden.contains(target) || !titles.contains_key(target) {
                continue;
            }
            edges.push(FlowEdge {
                from: page.id.clone(),
                to: target.to_string(),
            });
        }
    }

    let ids: Vec<String> = visible.iter().map(|p| p.id.clone()).collect();
    let levels = levels(&ids, &edges);
    let nodes = place(&ids, &levels, |id| {
        let page = visible.iter().find(|p| &p.id == id);
        let title = titles.get(id).cloned().unwrap_or_default();
        let subtitle = match (subtitle, page) {
            (Some(role), Some(page)) => source
                .property(role)
                .and_then(|property| page.properties.get(property))
                .map(rows::text_of)
                .unwrap_or_default(),
            _ => String::new(),
        };
        (title, subtitle)
    });

    let width = nodes
        .iter()
        .map(|n| n.x + NODE_WIDTH)
        .fold(0.0_f32, f32::max);
    let height = nodes
        .iter()
        .map(|n| n.y + NODE_HEIGHT)
        .fold(0.0_f32, f32::max);

    Ok(FlowGraph {
        nodes,
        edges,
        width,
        height,
        all_nodes,
        subtitle_roles: source
            .mappings
            .iter()
            .map(|m| m.role.clone())
            .filter(|role| role != "title" && role != "next")
            .collect(),
    })
}

/// Jeder Knoten kommt eine Ebene hinter seinen spätesten Vorgänger.
///
/// Kahn, mit einer Bremse: Ein Kreis in den Daten — A zeigt auf B, B zurück auf
/// A — würde sonst ewig laufen. Was übrig bleibt, landet auf der letzten Ebene.
fn levels(ids: &[String], edges: &[FlowEdge]) -> BTreeMap<String, usize> {
    let mut level: BTreeMap<String, usize> = BTreeMap::new();
    let mut incoming: BTreeMap<&str, usize> = ids.iter().map(|id| (id.as_str(), 0)).collect();
    for edge in edges {
        if let Some(count) = incoming.get_mut(edge.to.as_str()) {
            *count += 1;
        }
    }

    let mut queue: Vec<&str> = ids
        .iter()
        .map(String::as_str)
        .filter(|id| incoming[id] == 0)
        .collect();
    for id in &queue {
        level.insert((*id).to_string(), 0);
    }

    let mut i = 0;
    while i < queue.len() {
        let current = queue[i];
        i += 1;
        let depth = level[current];
        for edge in edges.iter().filter(|e| e.from == current) {
            let target = edge.to.as_str();
            let next = level.get(target).copied().unwrap_or(0).max(depth + 1);
            level.insert(target.to_string(), next);
            if let Some(count) = incoming.get_mut(target) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    queue.push(target);
                }
            }
        }
    }

    // Was in einem Kreis steckt, hat nie die Warteschlange erreicht.
    let deepest = level.values().copied().max().unwrap_or(0);
    for id in ids {
        level.entry(id.clone()).or_insert(deepest + 1);
    }
    level
}

/// Ebenen untereinander, innerhalb einer Ebene nebeneinander und mittig.
fn place(
    ids: &[String],
    levels: &BTreeMap<String, usize>,
    describe: impl Fn(&String) -> (String, String),
) -> Vec<FlowNode> {
    let mut by_level: BTreeMap<usize, Vec<&String>> = BTreeMap::new();
    // Die Reihenfolge der Seiten bleibt die des Abrufs — so steht dasselbe
    // Diagramm bei jedem Zeichnen gleich da.
    for id in ids {
        by_level.entry(levels[id]).or_default().push(id);
    }
    let widest = by_level
        .values()
        .map(|row| row.len())
        .max()
        .unwrap_or(1)
        .max(1) as f32;
    let full_width = widest * NODE_WIDTH + (widest - 1.0) * GAP_X;

    let mut nodes = Vec::new();
    for (depth, row) in by_level {
        let count = row.len() as f32;
        let row_width = count * NODE_WIDTH + (count - 1.0) * GAP_X;
        let left = (full_width - row_width) / 2.0;
        for (i, id) in row.into_iter().enumerate() {
            let (title, subtitle) = describe(id);
            nodes.push(FlowNode {
                id: id.clone(),
                title,
                subtitle,
                x: left + i as f32 * (NODE_WIDTH + GAP_X),
                y: depth as f32 * (NODE_HEIGHT + GAP_Y),
            });
        }
    }
    nodes
}
