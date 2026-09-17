//! Gespeicherte Ansichten: ein Diagramm samt dem, was daran eingestellt ist.
//!
//! Die Webapp vizu-notion-local hatte dafür Teilen-Links, die den Zustand in
//! die Adresse kodierten und über einen Server liefen. Hier gibt es keinen
//! Server — also bekommt der Zustand einen Namen und bleibt in der Datenbank
//! (docs/UMSETZUNG.md, E5).
//!
//! Was eine Ansicht festhält: welches Diagramm (`kind` + `target`), welche
//! Seiten ausgeblendet sind und — beim Fluss — welche Rolle unter dem Titel
//! steht. Beim Öffnen wird genau das wiederhergestellt.
//!
//! Absichtlich **ohne** Fremdschlüssel auf Vorlage oder Quelle: Eine Ansicht
//! auf etwas Gelöschtes ist kein Datenfehler, sondern eine Ansicht, die ins
//! Leere zeigt. Sie zu löschen wäre eine Entscheidung, die dem Menschen
//! gehört.

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{Error, Result, Validator};
use crate::ids::{parse_uuid, resolve_suffix};
use crate::timestamp;

/// Längster erlaubter Name. Er steht in der Liste.
pub const NAME_MAX: usize = 80;

/// Die Arten von Diagrammen, die es zu speichern gibt.
pub const KINDS: [&str; 3] = ["template", "flow", "metro"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct View {
    pub id: Uuid,
    pub name: String,
    /// `template`, `flow` oder `metro`.
    pub kind: String,
    /// Kennung der Vorlage bzw. der Quelle.
    pub target: String,
    /// Seiten-IDs, die nicht gezeichnet werden.
    pub hidden: Vec<String>,
    /// Beim Fluss: die Rolle unter dem Titel.
    pub subtitle: Option<String>,
    #[serde(with = "crate::timestamp::serde_rfc3339")]
    #[ts(type = "string")]
    pub created_at: OffsetDateTime,
    #[serde(with = "crate::timestamp::serde_rfc3339")]
    #[ts(type = "string")]
    pub updated_at: OffsetDateTime,
}

impl View {
    /// Die Kennung der Vorlage bzw. der Quelle, auf die die Ansicht zeigt.
    ///
    /// Steht als Text in der Datenbank, weil dort Vorlage und Quelle
    /// nebeneinander liegen — geprüft wird sie erst hier.
    pub fn target_id(&self) -> Result<Uuid> {
        parse_uuid(&self.target)
    }

    /// Die ausgeblendeten Seiten als Menge, so wie die Zeichner sie wollen.
    pub fn hidden_set(&self) -> std::collections::HashSet<String> {
        self.hidden.iter().cloned().collect()
    }
}

/// Was von außen hereinkommt — ungeprüft.
#[derive(Debug, Clone, Default, Deserialize, TS)]
pub struct ViewInput {
    pub name: String,
    pub kind: String,
    pub target: String,
    #[serde(default)]
    pub hidden: Vec<String>,
    #[serde(default)]
    pub subtitle: Option<String>,
}

impl ViewInput {
    pub fn new(
        name: impl Into<String>,
        kind: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            kind: kind.into(),
            target: target.into(),
            hidden: Vec::new(),
            subtitle: None,
        }
    }

    pub fn hiding(mut self, hidden: Vec<String>) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn with_subtitle(mut self, role: Option<String>) -> Self {
        self.subtitle = role;
        self
    }

    /// Räumt auf und prüft alle Felder, dann einmal abbrechen.
    pub fn clean(self) -> Result<ViewInput> {
        let mut v = Validator::new();

        let name = self.name.trim().to_string();
        v.require(!name.is_empty(), "name", "darf nicht leer sein");
        v.require(
            name.chars().count() <= NAME_MAX,
            "name",
            format!("darf höchstens {NAME_MAX} Zeichen lang sein"),
        );
        v.require(
            KINDS.contains(&self.kind.as_str()),
            "kind",
            format!("unbekannte Art — erlaubt sind {}", KINDS.join(", ")),
        );
        let target = self.target.trim().to_string();
        v.require(!target.is_empty(), "target", "fehlt");
        v.finish()?;

        // Doppelte und leere Kennungen fielen sonst erst beim Zeichnen auf.
        let mut hidden: Vec<String> = self
            .hidden
            .into_iter()
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty())
            .collect();
        hidden.sort();
        hidden.dedup();

        Ok(ViewInput {
            name,
            kind: self.kind,
            target,
            hidden,
            subtitle: self.subtitle.filter(|s| !s.trim().is_empty()),
        })
    }
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<Result<View>> {
    let id: String = row.get(0)?;
    let hidden: String = row.get(4)?;
    let created: String = row.get(6)?;
    let updated: String = row.get(7)?;
    Ok((|| {
        Ok(View {
            id: parse_uuid(&id)?,
            name: row.get(1)?,
            kind: row.get(2)?,
            target: row.get(3)?,
            // Die Spalte schreibt nur dieser Code; steht dort etwas anderes,
            // ist die Datei beschädigt.
            hidden: serde_json::from_str(&hidden)
                .map_err(|e| Error::Corrupt(format!("Ansicht {id}: `hidden` unlesbar: {e}")))?,
            subtitle: row.get(5)?,
            created_at: timestamp::from_text(&created)?,
            updated_at: timestamp::from_text(&updated)?,
        })
    })())
}

const COLUMNS: &str = "id, name, kind, target, hidden, subtitle, created_at, updated_at";

/// Alle Ansichten, nach Namen sortiert.
pub fn list(conn: &Connection) -> Result<Vec<View>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM views ORDER BY name COLLATE NOCASE, id"
    ))?;
    let rows = stmt
        .query_map([], from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    rows.into_iter().collect()
}

pub fn get(conn: &Connection, id: Uuid) -> Result<View> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM views WHERE id = ?1"),
        [id.to_string()],
        from_row,
    )
    .optional()?
    .ok_or(Error::NotFound)?
}

pub fn create(conn: &Connection, input: ViewInput) -> Result<View> {
    let input = input.clean()?;
    ensure_unique_name(conn, &input.name, None)?;

    let id = Uuid::now_v7();
    let now = timestamp::to_text(timestamp::now());
    conn.execute(
        "INSERT INTO views (id, name, kind, target, hidden, subtitle, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            id.to_string(),
            input.name,
            input.kind,
            input.target,
            serde_json::to_string(&input.hidden).expect("Liste von Text ist serialisierbar"),
            input.subtitle,
            now,
        ],
    )?;
    tracing::debug!(%id, name = %input.name, "Ansicht gespeichert");
    get(conn, id)
}

/// Schreibt eine Ansicht neu — etwa, wenn man dieselbe noch einmal speichert.
pub fn update(conn: &Connection, id: Uuid, input: ViewInput) -> Result<View> {
    let input = input.clean()?;
    get(conn, id)?;
    ensure_unique_name(conn, &input.name, Some(id))?;

    conn.execute(
        "UPDATE views SET name = ?2, kind = ?3, target = ?4, hidden = ?5, subtitle = ?6, \
         updated_at = ?7 WHERE id = ?1",
        params![
            id.to_string(),
            input.name,
            input.kind,
            input.target,
            serde_json::to_string(&input.hidden).expect("Liste von Text ist serialisierbar"),
            input.subtitle,
            timestamp::to_text(timestamp::now()),
        ],
    )?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: Uuid) -> Result<()> {
    let changed = conn.execute("DELETE FROM views WHERE id = ?1", [id.to_string()])?;
    if changed == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

/// Findet eine Ansicht über ihren Namen, ihre Kennung oder deren Endstück.
pub fn resolve(conn: &Connection, needle: &str) -> Result<Uuid> {
    let needle = needle.trim();
    let by_name: Option<String> = conn
        .query_row(
            "SELECT id FROM views WHERE name = ?1 COLLATE NOCASE",
            [needle],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = by_name {
        return parse_uuid(&id);
    }
    if let Ok(id) = Uuid::parse_str(needle) {
        get(conn, id)?;
        return Ok(id);
    }
    resolve_suffix(conn, "views", needle)
}

fn ensure_unique_name(conn: &Connection, name: &str, except: Option<Uuid>) -> Result<()> {
    let taken: Option<String> = conn
        .query_row(
            "SELECT id FROM views WHERE name = ?1 COLLATE NOCASE AND id IS NOT ?2",
            params![name, except.map(|id| id.to_string())],
            |r| r.get(0),
        )
        .optional()?;
    let mut v = Validator::new();
    v.require(
        taken.is_none(),
        "name",
        format!("eine Ansicht „{name}\u{201c} gibt es schon"),
    );
    v.finish()
}
