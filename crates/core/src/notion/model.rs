//! Was von Notion zurückkommt — nur die Teile, die diese Anwendung braucht.
//!
//! Unbekannte Felder werden überlesen. Notion ergänzt regelmäßig Felder, und
//! ein neues Feld soll keinen Abruf scheitern lassen.

use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Deserialize)]
pub struct Database {
    pub id: String,
    #[serde(default)]
    title: Vec<RichText>,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub data_sources: Vec<DataSourceRef>,
}

impl Database {
    /// Der Titel als Text, alle Stücke verbunden.
    pub fn title(&self) -> String {
        self.title.iter().map(|t| t.plain_text.as_str()).collect()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DataSourceRef {
    pub id: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RichText {
    #[serde(default)]
    plain_text: String,
}

/// Eine Spalte aus dem Schema einer Datenquelle.
///
/// Geht auch über die IPC-Grenze: Die Oberfläche bietet beim Zuordnen die
/// Spalten zur Auswahl an, die der letzte Abruf gesehen hat.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
pub struct Property {
    pub name: String,
    /// Kurzkennung der Spalte, z. B. `%5EGHx` — bereits für Adressen kodiert.
    pub id: String,
    /// `title`, `relation`, `multi_select`, …
    pub kind: String,
    /// Bei einer Relation: die Datenquelle, auf die sie zeigt. Nur so ist ein
    /// Verweis auf dieselbe Datenbank („Nächstes") von einem auf eine andere
    /// („Ziel") zu unterscheiden.
    pub relation_to: Option<String>,
}

/// Die Spalten aus der Antwort von `GET /data_sources/{id}`, nach Namen sortiert.
pub fn property_kinds(data_source: &Value) -> Vec<Property> {
    let mut out: Vec<Property> = data_source["properties"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(name, p)| Property {
            name: name.clone(),
            id: p["id"].as_str().unwrap_or_default().to_string(),
            kind: p["type"].as_str().unwrap_or_default().to_string(),
            relation_to: p["relation"]["data_source_id"].as_str().map(String::from),
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Page {
    pub id: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub last_edited_time: String,
    /// Spaltenname → Eigenschaft, so wie Notion sie liefert.
    #[serde(default)]
    pub properties: Map<String, Value>,
}
