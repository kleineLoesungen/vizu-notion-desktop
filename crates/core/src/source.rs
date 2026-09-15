//! Quellen: eine Notion-Datenbank unter einem Namen, mit Spaltenzuordnung.
//!
//! Der Name ist das, was eine Vorlage benutzt (`{{#each Projekte}}`). Die
//! Zuordnung sagt, welche Notion-Spalte hinter einer Rolle steht
//! (`title` → „Name", `next` → „Nächstes").
//!
//! Die Prüfung steht hier und nur hier — die Kommandozeile, der Import einer
//! `sources.json` und die Oberfläche gehen alle durch [`SourceInput::clean`].

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{Error, Result, Validator};
use crate::ids::{parse_uuid, resolve_suffix};
use crate::{notion, timestamp};

/// Längster erlaubter Name. Er steht in Menüs und in Vorlagen.
pub const NAME_MAX: usize = 80;

/// Diese Rolle belegt die Anwendung selbst: `{{this.id}}` ist die Seiten-ID.
pub const RESERVED_ROLE: &str = "id";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct Source {
    pub id: Uuid,
    pub name: String,
    /// Die Notion-Kennung, klein und mit Bindestrichen.
    pub database_id: String,
    /// Nach Rolle sortiert.
    pub mappings: Vec<ColumnMapping>,
    #[serde(with = "crate::timestamp::serde_rfc3339")]
    #[ts(type = "string")]
    pub created_at: OffsetDateTime,
    #[serde(with = "crate::timestamp::serde_rfc3339")]
    #[ts(type = "string")]
    pub updated_at: OffsetDateTime,
}

impl Source {
    /// Die Notion-Spalte hinter einer Rolle.
    pub fn property(&self, role: &str) -> Option<&str> {
        self.mappings
            .iter()
            .find(|m| m.role == role)
            .map(|m| m.property.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ColumnMapping {
    /// Wie die Vorlage das Feld nennt: `title`, `next`, `parent` …
    pub role: String,
    /// Wie die Spalte in Notion heißt — genau so geschrieben.
    pub property: String,
}

impl ColumnMapping {
    pub fn new(role: impl Into<String>, property: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            property: property.into(),
        }
    }
}

/// Was von außen hereinkommt — ungeprüft.
#[derive(Debug, Clone, Default, Deserialize, TS)]
pub struct SourceInput {
    pub name: String,
    /// Kennung, Seitenname mit Kennung oder Adresse aus Notion.
    pub database_id: String,
    pub mappings: Vec<ColumnMapping>,
}

impl SourceInput {
    pub fn new(
        name: impl Into<String>,
        database_id: impl Into<String>,
        mappings: Vec<ColumnMapping>,
    ) -> Self {
        Self {
            name: name.into(),
            database_id: database_id.into(),
            mappings,
        }
    }

    /// Räumt auf und prüft alle Felder, dann einmal abbrechen.
    ///
    /// Die Datenbank-Kennung wird dabei in die einheitliche Form gebracht, die
    /// Zuordnung nach Rolle sortiert.
    pub fn clean(self) -> Result<SourceInput> {
        let mut v = Validator::new();

        let name = self.name.trim().to_string();
        v.require(!name.is_empty(), "name", "darf nicht leer sein");
        v.require(
            name.chars().count() <= NAME_MAX,
            "name",
            format!("darf höchstens {NAME_MAX} Zeichen lang sein"),
        );

        let database_id = match notion::parse_id(&self.database_id) {
            Some(id) => id,
            None => {
                v.add(
                    "database_id",
                    "keine Notion-Kennung — die 32 Zeichen aus der Adresse der Datenbank einfügen",
                );
                String::new()
            }
        };

        let mut mappings: Vec<ColumnMapping> = Vec::with_capacity(self.mappings.len());
        for m in self.mappings {
            let role = m.role.trim().to_string();
            let property = m.property.trim().to_string();
            let field = format!("mappings.{role}");
            if !is_role_name(&role) {
                // Der Umschreiber der Vorlagen erkennt nur solche Namen als
                // Feld ({{next}}). Ein „next-step" wäre dort unsichtbar.
                v.add(
                    "mappings",
                    format!(
                        "Rolle „{role}\u{201c} ungültig — Buchstaben, Ziffern und _, nicht mit einer Ziffer beginnend"
                    ),
                );
                continue;
            }
            if role == RESERVED_ROLE {
                v.add(
                    field,
                    "„id\u{201c} ist die Seiten-ID und kann nicht zugeordnet werden",
                );
                continue;
            }
            if property.is_empty() {
                v.add(field, "Spaltenname fehlt");
                continue;
            }
            if mappings.iter().any(|existing| existing.role == role) {
                v.add(field, "Rolle doppelt zugeordnet");
                continue;
            }
            mappings.push(ColumnMapping { role, property });
        }
        mappings.sort_by(|a, b| a.role.cmp(&b.role));

        v.finish()?;
        Ok(SourceInput {
            name,
            database_id,
            mappings,
        })
    }
}

fn is_role_name(role: &str) -> bool {
    let mut chars = role.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn list(conn: &Connection) -> Result<Vec<Source>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, database_id, created_at, updated_at \
         FROM sources ORDER BY name COLLATE NOCASE, id",
    )?;
    let heads = stmt
        .query_map([], head_from_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    heads
        .into_iter()
        .map(|head| with_mappings(conn, head?))
        .collect()
}

pub fn get(conn: &Connection, id: Uuid) -> Result<Source> {
    let head = conn
        .query_row(
            "SELECT id, name, database_id, created_at, updated_at FROM sources WHERE id = ?1",
            [id.to_string()],
            head_from_row,
        )
        .optional()?
        .ok_or(Error::NotFound)??;
    with_mappings(conn, head)
}

pub fn count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT count(*) FROM sources", [], |r| r.get(0))?)
}

pub fn create(conn: &Connection, input: SourceInput) -> Result<Source> {
    let input = input.clean()?;
    ensure_unique_name(conn, &input.name, None, "name")?;
    let now = timestamp::now();
    let id = Uuid::now_v7();

    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO sources (id, name, database_id, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![
            id.to_string(),
            input.name,
            input.database_id,
            timestamp::to_text(now)
        ],
    )?;
    insert_mappings(&tx, id, &input.mappings)?;
    tx.commit()?;

    tracing::debug!(%id, name = %input.name, "Quelle angelegt");
    get(conn, id)
}

/// Ersetzt Name, Kennung und die ganze Zuordnung.
///
/// Zeigt die Quelle danach auf eine andere Datenbank, ist der letzte Abruf
/// wertlos und wird verworfen — sonst stünden Seiten der alten Datenbank unter
/// dem neuen Namen.
pub fn update(conn: &Connection, id: Uuid, input: SourceInput) -> Result<Source> {
    let input = input.clean()?;
    let before = get(conn, id)?;
    ensure_unique_name(conn, &input.name, Some(id), "name")?;
    let now = timestamp::now();

    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE sources SET name = ?2, database_id = ?3, updated_at = ?4 WHERE id = ?1",
        params![
            id.to_string(),
            input.name,
            input.database_id,
            timestamp::to_text(now)
        ],
    )?;
    tx.execute(
        "DELETE FROM source_mappings WHERE source_id = ?1",
        [id.to_string()],
    )?;
    insert_mappings(&tx, id, &input.mappings)?;
    if before.database_id != input.database_id {
        tx.execute("DELETE FROM pages WHERE source_id = ?1", [id.to_string()])?;
        tx.execute("DELETE FROM fetches WHERE source_id = ?1", [id.to_string()])?;
    }
    tx.commit()?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: Uuid) -> Result<()> {
    // Zuordnung, Abruf und Seiten gehen per ON DELETE CASCADE mit.
    let changed = conn.execute("DELETE FROM sources WHERE id = ?1", [id.to_string()])?;
    if changed == 0 {
        return Err(Error::NotFound);
    }
    tracing::debug!(%id, "Quelle gelöscht");
    Ok(())
}

/// Findet eine Quelle über ihren Namen (ohne Rücksicht auf Groß- und
/// Kleinschreibung), ihre volle Kennung oder ein eindeutiges Endstück.
///
/// Der Name hat Vorrang: So heißt die Quelle in Vorlagen und in der Liste.
pub fn resolve(conn: &Connection, needle: &str) -> Result<Uuid> {
    let needle = needle.trim();
    let by_name: Option<String> = conn
        .query_row(
            "SELECT id FROM sources WHERE name = ?1 COLLATE NOCASE",
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
    resolve_suffix(conn, "sources", needle)
}

// --- Import aus der Webapp -------------------------------------------------

/// Die `sources.json` der Webapp vizu-notion-local.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WebappConfig {
    sources: Vec<WebappSource>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct WebappSource {
    database_id: String,
    name: String,
    column_mappings: std::collections::BTreeMap<String, String>,
}

/// Legt alle Quellen aus einer `sources.json` an — alle oder keine.
///
/// Fehler tragen die Nummer des Eintrags im Feldnamen: `sources[2].name`.
/// Ein Name, den es schon gibt, ist ein Fehler; stillschweigend überschreiben
/// wäre bei einer vorhandenen Zuordnung die falsche Voreinstellung.
pub fn import_webapp_json(conn: &Connection, text: &str) -> Result<Vec<Source>> {
    let config: WebappConfig = serde_json::from_str(text).map_err(|e| {
        let mut v = Validator::new();
        v.add("file", format!("keine gültige sources.json: {e}"));
        v.finish().unwrap_err()
    })?;

    let mut v = Validator::new();
    let mut cleaned = Vec::with_capacity(config.sources.len());
    for (i, s) in config.sources.into_iter().enumerate() {
        let prefix = format!("sources[{i}].");
        let input = SourceInput::new(
            s.name,
            s.database_id,
            s.column_mappings
                .into_iter()
                .map(|(role, property)| ColumnMapping { role, property })
                .collect(),
        );
        match input.clean() {
            Ok(input) => {
                if cleaned
                    .iter()
                    .any(|c: &SourceInput| c.name.eq_ignore_ascii_case(&input.name))
                {
                    v.add(format!("{prefix}name"), "kommt in der Datei doppelt vor");
                } else if let Err(e) =
                    ensure_unique_name(conn, &input.name, None, &format!("{prefix}name"))
                {
                    v.absorb("", e)?;
                }
                cleaned.push(input);
            }
            Err(e) => v.absorb(&prefix, e)?,
        }
    }
    v.finish()?;

    let tx = conn.unchecked_transaction()?;
    let mut ids = Vec::with_capacity(cleaned.len());
    for input in cleaned {
        let now = timestamp::to_text(timestamp::now());
        let id = Uuid::now_v7();
        tx.execute(
            "INSERT INTO sources (id, name, database_id, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![id.to_string(), input.name, input.database_id, now],
        )?;
        insert_mappings(&tx, id, &input.mappings)?;
        ids.push(id);
    }
    tx.commit()?;
    ids.into_iter().map(|id| get(conn, id)).collect()
}

// --- intern ------------------------------------------------------------------

fn ensure_unique_name(
    conn: &Connection,
    name: &str,
    except: Option<Uuid>,
    field: &str,
) -> Result<()> {
    let taken: Option<String> = conn
        .query_row(
            "SELECT id FROM sources WHERE name = ?1 COLLATE NOCASE AND id IS NOT ?2",
            params![name, except.map(|id| id.to_string())],
            |r| r.get(0),
        )
        .optional()?;
    let mut v = Validator::new();
    v.require(
        taken.is_none(),
        field,
        format!("eine Quelle „{name}\u{201c} gibt es schon"),
    );
    v.finish()
}

fn insert_mappings(conn: &Connection, id: Uuid, mappings: &[ColumnMapping]) -> Result<()> {
    let mut stmt = conn
        .prepare("INSERT INTO source_mappings (source_id, role, property) VALUES (?1, ?2, ?3)")?;
    for m in mappings {
        stmt.execute(params![id.to_string(), m.role, m.property])?;
    }
    Ok(())
}

struct Head {
    id: Uuid,
    name: String,
    database_id: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

fn head_from_row(row: &Row<'_>) -> rusqlite::Result<Result<Head>> {
    let id: String = row.get(0)?;
    let created_at: String = row.get(3)?;
    let updated_at: String = row.get(4)?;
    let name: String = row.get(1)?;
    let database_id: String = row.get(2)?;
    Ok((|| {
        Ok(Head {
            id: parse_uuid(&id)?,
            name,
            database_id,
            created_at: timestamp::from_text(&created_at)?,
            updated_at: timestamp::from_text(&updated_at)?,
        })
    })())
}

fn with_mappings(conn: &Connection, head: Head) -> Result<Source> {
    let mut stmt = conn
        .prepare("SELECT role, property FROM source_mappings WHERE source_id = ?1 ORDER BY role")?;
    let mappings = stmt
        .query_map([head.id.to_string()], |r| {
            Ok(ColumnMapping {
                role: r.get(0)?,
                property: r.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(Source {
        id: head.id,
        name: head.name,
        database_id: head.database_id,
        mappings,
        created_at: head.created_at,
        updated_at: head.updated_at,
    })
}
