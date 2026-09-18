//! Einen Baustein in eine Vorlage setzen, ohne ihren Aufbau zu zerbrechen.
//!
//! Eine Vorlage hat immer dieselbe Form:
//!
//! ```text
//! ---                       ┐
//! title: "…"                │ Kopf — `sources` nennt jede benutzte Quelle
//! sources:                  │
//!   - Projekte              ┘
//! ---
//! flowchart LR              ← die Art des Diagramms
//! {{#each Projekte}} …      ← der Rumpf
//! ```
//!
//! Wer einen Baustein blind an der Schreibmarke einsetzt, verletzt das
//! leicht: Er landet im Kopf, vor der `flowchart`-Zeile, oder benutzt eine
//! Quelle, die der Kopf nicht nennt. Jeder dieser Fälle endet in einer
//! Fehlermeldung. [`insert`] kennt die Form und setzt so ein, dass sie hält.
//!
//! Die Schreibmarke kommt aus dem Textfeld des Webviews und zählt dort in
//! UTF-16-Einheiten, nicht in Bytes — ein Emoji im Titel verschöbe sie sonst.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Was der Editor schickt.
#[derive(Debug, Clone, Deserialize, TS)]
pub struct InsertInput {
    pub body: String,
    /// Schreibmarke in UTF-16-Einheiten, wie `selectionStart` im Textfeld.
    pub cursor: u32,
    pub snippet: String,
    /// Quellen, die der Baustein benutzt — der Kopf muss sie nennen.
    pub sources: Vec<String>,
}

/// Die Vorlage danach, und wo die Schreibmarke hingehört: hinter das
/// Eingesetzte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct Inserted {
    pub body: String,
    pub cursor: u32,
}

/// Wörter, mit denen Mermaid eine Diagrammart beginnt. Steht keins davon am
/// Anfang des Rumpfs, fehlt die Zeile.
const DIAGRAM_KINDS: [&str; 23] = [
    "flowchart",
    "graph",
    "sequenceDiagram",
    "classDiagram",
    "stateDiagram",
    "stateDiagram-v2",
    "erDiagram",
    "gantt",
    "pie",
    "journey",
    "timeline",
    "mindmap",
    "kanban",
    "quadrantChart",
    "gitGraph",
    "xychart-beta",
    "sankey-beta",
    "block-beta",
    "packet-beta",
    "architecture-beta",
    "requirementDiagram",
    "C4Context",
    "radar-beta",
];

const DEFAULT_KIND: &str = "flowchart LR";

pub fn insert(input: InsertInput) -> Inserted {
    let InsertInput {
        body,
        cursor,
        snippet,
        sources,
    } = input;

    let (head, rest) = split_head(&body);
    let snippet = normalize_snippet(&snippet, head.is_some());

    let (head, rest, cursor_in_rest) = match head {
        Some(head) => {
            let at = utf16_to_byte(&body, cursor);
            // In den Kopf gehört kein Baustein: dann ans Ende.
            let in_rest = at.checked_sub(head.len());
            (head.to_string(), rest.to_string(), in_rest)
        }
        None => {
            // Ohne Kopf: das Gerüst eines Bausteins, der einen mitbringt,
            // oder ein frischer. Was schon dastand, bleibt als Rumpf erhalten.
            if let Some(stripped) = snippet.full_head.clone() {
                let text = stripped.clone() + rest.trim_start();
                return finish(text, stripped.len(), &sources);
            }
            (new_head(&sources), rest.trim_start().to_string(), None)
        }
    };

    let (rest, at) = place(&rest, cursor_in_rest, &snippet.text);
    let text = head.clone() + &rest;
    finish(text, head.len() + at, &sources)
}

/// Kopf ergänzen und die Schreibmarke umrechnen.
fn finish(text: String, byte_cursor: usize, sources: &[String]) -> Inserted {
    let (head, rest) = split_head(&text);
    let Some(head) = head else {
        return Inserted {
            cursor: byte_to_utf16(&text, byte_cursor),
            body: text,
        };
    };
    let new_head = with_sources(head, sources);
    let shift = new_head.len() as isize - head.len() as isize;
    let body = new_head + rest;
    let cursor = (byte_cursor as isize + shift).max(0) as usize;
    Inserted {
        cursor: byte_to_utf16(&body, cursor),
        body,
    }
}

/// Ein Baustein, bereit zum Einsetzen.
struct Snippet {
    /// Was in den Rumpf kommt, mit Zeilenende.
    text: String,
    /// Bringt der Baustein einen eigenen Kopf mit (das Gerüst) und hat die
    /// Vorlage noch keinen: der ganze Baustein.
    full_head: Option<String>,
}

fn normalize_snippet(snippet: &str, has_head: bool) -> Snippet {
    let mut text = snippet.to_string();
    if !text.ends_with('\n') {
        text.push('\n');
    }
    let (own_head, own_rest) = split_head(&text);
    match own_head {
        Some(_) if !has_head => Snippet {
            text: String::new(),
            full_head: Some(text.clone()),
        },
        // Das Gerüst in eine fertige Vorlage: nur sein Rumpf, ohne Kopf und
        // ohne zweite `flowchart`-Zeile.
        Some(_) => Snippet {
            text: skip_kind_line(own_rest).to_string(),
            full_head: None,
        },
        None => Snippet {
            text,
            full_head: None,
        },
    }
}

/// Setzt den Baustein in den Rumpf: an die Schreibmarke, wenn sie hinter der
/// Zeile mit der Diagrammart steht, sonst ans Ende. Fehlt diese Zeile, kommt
/// sie dazu.
fn place(rest: &str, cursor: Option<usize>, snippet: &str) -> (String, usize) {
    let mut rest = rest.to_string();
    let mut cursor = cursor;
    let kind_end = match kind_line_end(&rest) {
        Some(end) => end,
        None => {
            let line = format!("{DEFAULT_KIND}\n");
            rest.insert_str(0, &line);
            // Was vorher an der Schreibmarke stand, ist um die Zeile gerückt.
            cursor = cursor.map(|c| c + line.len());
            line.len()
        }
    };

    let at = cursor
        .filter(|&c| c >= kind_end && c <= rest.len() && rest.is_char_boundary(c))
        .unwrap_or(rest.len());

    // Ein Baustein beginnt auf einer eigenen Zeile.
    let needs_break = at > 0 && !rest[..at].ends_with('\n');
    let piece = if needs_break {
        format!("\n{snippet}")
    } else {
        snippet.to_string()
    };
    rest.insert_str(at, &piece);
    (rest, at + piece.len())
}

/// Ende der Zeile mit der Diagrammart (hinter ihrem Zeilenumbruch), falls die
/// erste nicht leere Zeile des Rumpfs eine ist.
fn kind_line_end(rest: &str) -> Option<usize> {
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            offset += line.len();
            continue;
        }
        let word = trimmed.split_whitespace().next().unwrap_or("");
        return DIAGRAM_KINDS.contains(&word).then_some(offset + line.len());
    }
    None
}

fn skip_kind_line(rest: &str) -> &str {
    kind_line_end(rest).map(|end| &rest[end..]).unwrap_or(rest)
}

/// Kopf (bis einschließlich der schließenden `---`-Zeile) und Rest — dieselbe
/// Grenze wie [`super::parse::split`].
fn split_head(content: &str) -> (Option<&str>, &str) {
    let Some(after_open) = content.strip_prefix("---") else {
        return (None, content);
    };
    let Some(close) = after_open.find("\n---") else {
        return (None, content);
    };
    let after_close = 3 + close + 4;
    let end = content[after_close..]
        .find('\n')
        .map(|i| after_close + i + 1)
        .unwrap_or(content.len());
    (Some(&content[..end]), &content[end..])
}

fn new_head(sources: &[String]) -> String {
    let mut head = String::from("---\ntitle: \"Neues Diagramm\"\nsources:\n");
    for name in sources {
        head.push_str(&format!("  - {name}\n"));
    }
    head.push_str("---\n");
    head
}

/// Der Kopf mit jeder Quelle aus `needed` unter `sources` — was schon da ist,
/// bleibt, wie es geschrieben ist.
fn with_sources(head: &str, needed: &[String]) -> String {
    let lines: Vec<&str> = head.split_inclusive('\n').collect();
    let present = listed_sources(&lines);
    let missing: Vec<&String> = needed
        .iter()
        .filter(|name| !present.iter().any(|p| p.eq_ignore_ascii_case(name)))
        .collect();
    if missing.is_empty() {
        return head.to_string();
    }
    let items: String = missing.iter().map(|n| format!("  - {n}\n")).collect();

    let key = lines
        .iter()
        .position(|l| l.trim_start().starts_with("sources:"));
    match key {
        // `sources: [A, B]` — als Liste neu schreiben, sonst passte das
        // Anhängen nicht zur Schreibweise.
        Some(i) if lines[i].contains('[') => {
            let mut out: String = lines[..i].concat();
            out.push_str("sources:\n");
            for name in &present {
                out.push_str(&format!("  - {name}\n"));
            }
            out.push_str(&items);
            out.push_str(&lines[i + 1..].concat());
            out
        }
        // Block-Liste: hinter ihren letzten Eintrag.
        Some(i) => {
            let mut last = i;
            for (j, line) in lines.iter().enumerate().skip(i + 1) {
                if line.trim_start().starts_with("- ") {
                    last = j;
                } else if !line.trim().is_empty() {
                    break;
                }
            }
            let mut out: String = lines[..=last].concat();
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&items);
            out.push_str(&lines[last + 1..].concat());
            out
        }
        // Gar kein `sources`: vor die schließende Zeile.
        None => {
            let close = lines.len().saturating_sub(1);
            let mut out: String = lines[..close].concat();
            out.push_str("sources:\n");
            out.push_str(&items);
            out.push_str(&lines[close..].concat());
            out
        }
    }
}

/// Die Namen unter `sources`, in Block- oder Klammerschreibweise.
fn listed_sources(lines: &[&str]) -> Vec<String> {
    let Some(i) = lines
        .iter()
        .position(|l| l.trim_start().starts_with("sources:"))
    else {
        return Vec::new();
    };
    let clean = |s: &str| s.trim().trim_matches(['"', '\'']).to_string();
    if let Some(open) = lines[i].find('[') {
        let inner = lines[i][open + 1..].split(']').next().unwrap_or("");
        return inner
            .split(',')
            .map(clean)
            .filter(|s| !s.is_empty())
            .collect();
    }
    let mut out = Vec::new();
    for line in &lines[i + 1..] {
        let trimmed = line.trim_start();
        if let Some(item) = trimmed.strip_prefix("- ") {
            out.push(clean(item));
        } else if !trimmed.trim().is_empty() {
            break;
        }
    }
    out
}

fn utf16_to_byte(s: &str, units: u32) -> usize {
    let mut count = 0u32;
    for (byte, c) in s.char_indices() {
        if count >= units {
            return byte;
        }
        count += c.len_utf16() as u32;
    }
    s.len()
}

fn byte_to_utf16(s: &str, byte: usize) -> u32 {
    s[..byte.min(s.len())]
        .chars()
        .map(|c| c.len_utf16() as u32)
        .sum()
}
