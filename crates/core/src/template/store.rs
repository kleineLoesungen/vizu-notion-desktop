//! Vorlagen speichern, finden, löschen.

use rusqlite::{Connection, OptionalExtension, Row as SqlRow, params};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;
use uuid::Uuid;

use super::parse;
use crate::error::{Error, Result, Validator};
use crate::ids::{parse_uuid, resolve_suffix};
use crate::{source, timestamp};

/// Längster erlaubter Kurzname.
pub const SLUG_MAX: usize = 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct Template {
    pub id: Uuid,
    /// Kurzname für die Kommandozeile — der Dateiname ohne `.mmd`.
    pub slug: String,
    /// Aus dem Kopf: die Beschriftung in der Oberfläche.
    pub title: String,
    /// Aus dem Kopf: die benutzten Quellen.
    pub sources: Vec<String>,
    /// Der ganze Text, Kopf inbegriffen.
    pub body: String,
    #[serde(with = "crate::timestamp::serde_rfc3339")]
    #[ts(type = "string")]
    pub created_at: OffsetDateTime,
    #[serde(with = "crate::timestamp::serde_rfc3339")]
    #[ts(type = "string")]
    pub updated_at: OffsetDateTime,
}

/// Was von außen hereinkommt — ungeprüft.
#[derive(Debug, Clone, Default, Deserialize, TS)]
pub struct TemplateInput {
    pub slug: String,
    pub body: String,
}

impl TemplateInput {
    pub fn new(slug: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            slug: slug.into(),
            body: body.into(),
        }
    }

    /// Räumt auf und prüft, was ohne Datenbank prüfbar ist: Kurzname und Kopf.
    ///
    /// Ob die Quellen existieren, weiß erst [`create`].
    pub fn clean(self) -> Result<(TemplateInput, parse::Meta)> {
        let slug = self.slug.trim().to_ascii_lowercase();
        let mut v = Validator::new();
        v.require(!slug.is_empty(), "slug", "darf nicht leer sein");
        v.require(
            slug.chars().count() <= SLUG_MAX,
            "slug",
            format!("darf höchstens {SLUG_MAX} Zeichen lang sein"),
        );
        v.require(
            slug.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "slug",
            "nur Buchstaben, Ziffern, - und _",
        );

        // Der Kopf wird auch dann geprüft, wenn der Kurzname schon falsch ist —
        // alle Fehler auf einmal.
        let meta = match parse::meta(&self.body) {
            Ok(meta) => Some(meta),
            Err(e) => {
                v.absorb("", e)?;
                None
            }
        };
        v.finish()?;
        Ok((
            TemplateInput {
                slug,
                body: self.body,
            },
            meta.expect("ohne Fehler gibt es einen Kopf"),
        ))
    }
}

pub fn list(conn: &Connection) -> Result<Vec<Template>> {
    let mut stmt = conn.prepare(
        "SELECT id, slug, body, created_at, updated_at FROM templates \
         ORDER BY slug COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], from_row)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .collect()
}

pub fn get(conn: &Connection, id: Uuid) -> Result<Template> {
    conn.query_row(
        "SELECT id, slug, body, created_at, updated_at FROM templates WHERE id = ?1",
        [id.to_string()],
        from_row,
    )
    .optional()?
    .ok_or(Error::NotFound)?
}

/// Findet eine Vorlage über ihren Kurznamen, ihre Kennung oder ein eindeutiges
/// Endstück davon.
pub fn resolve(conn: &Connection, needle: &str) -> Result<Uuid> {
    let needle = needle.trim();
    let by_slug: Option<String> = conn
        .query_row(
            "SELECT id FROM templates WHERE slug = ?1 COLLATE NOCASE",
            [needle],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = by_slug {
        return parse_uuid(&id);
    }
    if let Ok(id) = Uuid::parse_str(needle) {
        get(conn, id)?;
        return Ok(id);
    }
    resolve_suffix(conn, "templates", needle)
}

pub fn create(conn: &Connection, input: TemplateInput) -> Result<Template> {
    let (input, meta) = input.clean()?;
    ensure_unique_slug(conn, &input.slug, None)?;
    check_sources(conn, &meta)?;

    let id = Uuid::now_v7();
    let now = timestamp::to_text(timestamp::now());
    conn.execute(
        "INSERT INTO templates (id, slug, body, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
        params![id.to_string(), input.slug, input.body, now],
    )?;
    tracing::debug!(%id, slug = %input.slug, "Vorlage angelegt");
    get(conn, id)
}

pub fn update(conn: &Connection, id: Uuid, input: TemplateInput) -> Result<Template> {
    let (input, meta) = input.clean()?;
    get(conn, id)?;
    ensure_unique_slug(conn, &input.slug, Some(id))?;
    check_sources(conn, &meta)?;

    conn.execute(
        "UPDATE templates SET slug = ?2, body = ?3, updated_at = ?4 WHERE id = ?1",
        params![
            id.to_string(),
            input.slug,
            input.body,
            timestamp::to_text(timestamp::now())
        ],
    )?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: Uuid) -> Result<()> {
    let changed = conn.execute("DELETE FROM templates WHERE id = ?1", [id.to_string()])?;
    if changed == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

/// Legt eine Vorlage aus dem Inhalt einer `.mmd`-Datei an.
///
/// Gibt es den Kurznamen schon, wird die vorhandene Vorlage **ersetzt** — eine
/// Datei einzulesen soll man wiederholen können, ohne vorher zu löschen.
pub fn import_mmd(conn: &Connection, slug: &str, body: &str) -> Result<Template> {
    let input = TemplateInput::new(slug, body);
    match resolve(conn, slug) {
        Ok(id) => update(conn, id, input),
        Err(Error::NotFound) => create(conn, input),
        Err(e) => Err(e),
    }
}

fn ensure_unique_slug(conn: &Connection, slug: &str, except: Option<Uuid>) -> Result<()> {
    let taken: Option<String> = conn
        .query_row(
            "SELECT id FROM templates WHERE slug = ?1 COLLATE NOCASE AND id IS NOT ?2",
            params![slug, except.map(|id| id.to_string())],
            |r| r.get(0),
        )
        .optional()?;
    let mut v = Validator::new();
    v.require(
        taken.is_none(),
        "slug",
        format!("eine Vorlage „{slug}\u{201c} gibt es schon"),
    );
    v.finish()
}

/// Jede Quelle im Kopf muss es geben — sonst zeichnet die Vorlage später ins
/// Leere, und der Fehler fiele erst beim Zeichnen auf.
fn check_sources(conn: &Connection, meta: &parse::Meta) -> Result<()> {
    let known = source::list(conn)?;
    let mut v = Validator::new();
    for name in &meta.sources {
        if !known.iter().any(|s| s.name.eq_ignore_ascii_case(name)) {
            let names: Vec<&str> = known.iter().map(|s| s.name.as_str()).collect();
            v.add(
                "body",
                format!(
                    "Quelle „{name}\u{201c} gibt es nicht (vorhanden: {})",
                    if names.is_empty() {
                        "keine".to_string()
                    } else {
                        names.join(", ")
                    }
                ),
            );
        }
    }
    v.finish()
}

fn from_row(row: &SqlRow<'_>) -> rusqlite::Result<Result<Template>> {
    let id: String = row.get(0)?;
    let slug: String = row.get(1)?;
    let body: String = row.get(2)?;
    let created_at: String = row.get(3)?;
    let updated_at: String = row.get(4)?;
    Ok((|| {
        // Der Kopf steht im Text; Titel und Quellen kommen beim Lesen daraus.
        // Eine gespeicherte Vorlage hat ihn bestanden — schlägt er jetzt fehl,
        // ist die Zeile beschädigt.
        let meta =
            parse::meta(&body).map_err(|e| Error::Corrupt(format!("Vorlage {slug}: {e}")))?;
        Ok(Template {
            id: parse_uuid(&id)?,
            slug,
            title: meta.title,
            sources: meta.sources,
            body,
            created_at: timestamp::from_text(&created_at)?,
            updated_at: timestamp::from_text(&updated_at)?,
        })
    })())
}
