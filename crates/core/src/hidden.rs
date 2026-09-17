//! Diagramme, die in der Liste nicht auftauchen sollen.
//!
//! Fluss und Metro entstehen von selbst, je Quelle eines. Wer sie nicht
//! braucht, blendet sie aus: Die Vorlage und die Quelle bleiben, nur der
//! Eintrag rutscht in den Bereich „Versteckt".
//!
//! Warum in der Datenbank und nicht in `config.toml`: Die Einstellungsdatei
//! ist zum Anfassen gedacht — eine Liste von Kennungen gehört dort nicht hin.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Result, Validator};
use crate::timestamp;

/// Ein verstecktes Diagramm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct HiddenDiagram {
    /// `template`, `flow` oder `metro`.
    pub kind: String,
    /// Kennung der Vorlage bzw. der Quelle.
    pub target: String,
}

/// Die Arten, die es zu verstecken gibt — dieselben wie in der Liste.
const KINDS: [&str; 3] = ["template", "flow", "metro"];

/// Alle versteckten Diagramme.
pub fn list(conn: &Connection) -> Result<Vec<HiddenDiagram>> {
    let mut stmt =
        conn.prepare("SELECT kind, target FROM hidden_diagrams ORDER BY kind, target")?;
    let rows = stmt
        .query_map([], |row| {
            Ok(HiddenDiagram {
                kind: row.get(0)?,
                target: row.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Versteckt ein Diagramm oder holt es zurück.
///
/// Zweimal verstecken ist kein Fehler — die Oberfläche soll nicht darüber
/// stolpern, dass zwei Fenster dasselbe tun.
pub fn set(conn: &Connection, entry: &HiddenDiagram, hidden: bool) -> Result<()> {
    let mut v = Validator::new();
    v.require(
        KINDS.contains(&entry.kind.as_str()),
        "kind",
        format!("unbekannte Art — erlaubt sind {}", KINDS.join(", ")),
    );
    v.require(!entry.target.trim().is_empty(), "target", "fehlt");
    v.finish()?;

    if hidden {
        conn.execute(
            "INSERT INTO hidden_diagrams (kind, target, hidden_at) VALUES (?1, ?2, ?3) \
             ON CONFLICT (kind, target) DO NOTHING",
            (
                &entry.kind,
                &entry.target,
                timestamp::to_text(timestamp::now()),
            ),
        )?;
    } else {
        conn.execute(
            "DELETE FROM hidden_diagrams WHERE kind = ?1 AND target = ?2",
            (&entry.kind, &entry.target),
        )?;
    }
    Ok(())
}
