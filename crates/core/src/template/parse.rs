//! Eine `.mmd`-Datei zerlegen: Frontmatter und Rumpf.

use serde_json::{Map, Value};

use crate::error::{Result, Validator};

/// Der Kopf einer Vorlage.
#[derive(Debug, Clone, PartialEq)]
pub struct Meta {
    /// Beschriftung der Schaltfläche in der Oberfläche.
    pub title: String,
    /// Namen der Quellen, die diese Vorlage benutzt.
    pub sources: Vec<String>,
    /// Form und Farbe je Attribut, siehe [`super::style`].
    pub styles: Map<String, Value>,
}

/// Frontmatter und Rumpf, ohne inhaltliche Prüfung.
///
/// Wie gray-matter in der Webapp: Das Frontmatter zählt **nur ganz am
/// Anfang** der Datei. Steht davor ein Kommentar, gibt es keins — genau daran
/// scheitert `config/mermaid.example` der Webapp.
pub fn split(content: &str) -> (Option<&str>, &str) {
    let Some(rest) = content.strip_prefix("---") else {
        return (None, content);
    };
    let Some(close) = rest.find("\n---") else {
        return (None, content);
    };
    let yaml = &rest[..close];
    let after = &rest[close + 4..];
    let body = after.find('\n').map(|i| &after[i + 1..]).unwrap_or("");
    (Some(yaml), body)
}

/// Liest und prüft den Kopf. Fehler hängen am Feld `body`, weil die
/// Oberfläche genau ein Textfeld dafür hat.
pub fn meta(content: &str) -> Result<Meta> {
    let (yaml, _) = split(content);
    let mut v = Validator::new();

    let Some(yaml) = yaml else {
        v.add(
            "body",
            "kein Kopf — die Vorlage beginnt mit `---`, dann `title:` und `sources:`, dann `---`",
        );
        return Err(v.finish().unwrap_err());
    };

    let data: Value = match serde_saphyr::from_str(yaml) {
        Ok(data) => data,
        Err(e) => {
            v.add("body", format!("Kopf ist kein gültiges YAML: {e}"));
            return Err(v.finish().unwrap_err());
        }
    };

    let title = data["title"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .to_string();
    v.require(!title.is_empty(), "body", "im Kopf fehlt `title`");

    let sources: Vec<String> = data["sources"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    v.require(
        !sources.is_empty(),
        "body",
        "im Kopf fehlt `sources` — mindestens eine Quelle",
    );

    let styles = data
        .get("styles")
        .and_then(|s| s.as_object())
        .cloned()
        .unwrap_or_default();

    v.finish()?;
    Ok(Meta {
        title,
        sources,
        styles,
    })
}

/// Der Rumpf ohne Frontmatter.
pub fn body(content: &str) -> &str {
    split(content).1
}
