//! Eine Quelle von Notion abrufen und zwischenspeichern.
//!
//! In zwei Schritten, und das ist Absicht:
//!
//! ```text
//! download(client, source)  →  Download      Netz, dauert Sekunden, keine Datenbank
//! store(conn, download)     →  FetchStatus   Datenbank, eine Transaktion, kein Netz
//! ```
//!
//! Die Desktop-Schale hält die Datenbank hinter einer Sperre. Läge der
//! Netzabruf darin, stünde jeder andere Befehl so lange still. Getrennt kann sie
//! den Download ohne Sperre laufen lassen und nur zum Speichern kurz sperren.

use std::collections::{BTreeMap, HashSet};

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::{Value, json};
use time::OffsetDateTime;
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{Error, NotionErrorKind, Result, Validator};
use crate::notion::{self, Client, Database, Page, Transport};
use crate::source::{self, ColumnMapping, Source};
use crate::timestamp;

/// Was ein Abruf von Notion mitgebracht hat, noch nicht gespeichert.
#[derive(Debug, Clone)]
pub struct Download {
    pub source_id: Uuid,
    pub database: Database,
    pub data_source_id: String,
    /// Die Antwort von `GET /data_sources/{id}`.
    pub schema: Value,
    pub pages: Vec<Page>,
    /// Titel von Relationszielen außerhalb dieser Quelle. Ohne sie stünde im
    /// Diagramm die Seiten-ID.
    pub titles: BTreeMap<String, String>,
    pub requests: u32,
}

/// Stand des letzten Abrufs einer Quelle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct FetchStatus {
    pub source_id: Uuid,
    pub database_title: String,
    pub database_url: String,
    pub page_count: u32,
    pub request_count: u32,
    #[serde(with = "crate::timestamp::serde_rfc3339")]
    #[ts(type = "string")]
    pub fetched_at: OffsetDateTime,
}

/// Die Spalten einer Notion-Datenbank, mit einem Vorschlag für die Zuordnung.
///
/// Für die Einrichtung: Beim Anlegen einer Quelle gibt es noch keinen Abruf,
/// aus dem die Spalten kämen — hier werden sie live geholt. Zwei Anfragen, mehr
/// nicht; die Seiten bleiben unangetastet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct DatabaseSchema {
    /// Wie die Datenbank in Notion heißt — als Vorschlag für den Namen.
    pub title: String,
    /// Die eigene Datenquelle. Eine Relation, die hierhin zeigt, verbindet
    /// Seiten derselben Datenbank — nur so eine kann `next` sein.
    pub data_source_id: String,
    pub properties: Vec<notion::Property>,
    /// Was core aus den Spalten für die Zuordnung ableitet.
    pub suggestion: Vec<ColumnMapping>,
}

pub fn inspect<T: Transport>(client: &Client<T>, database_id: &str) -> Result<DatabaseSchema> {
    let id = match notion::parse_id(database_id) {
        Some(id) => id,
        None => {
            let mut v = Validator::new();
            v.add(
                "database_id",
                "keine Notion-Kennung — die 32 Zeichen aus der Adresse der Datenbank einfügen",
            );
            v.finish()?;
            unreachable!("finish() bricht mit dem eben gemeldeten Fehler ab")
        }
    };
    let database = client.database(&id)?;
    let data_source = database.data_sources.first().ok_or_else(|| Error::Notion {
        kind: NotionErrorKind::Other,
        status: 200,
        message: format!(
            "„{}\u{201c} hat keine Datenquelle — ist es eine verknüpfte Ansicht statt der Datenbank selbst?",
            database.title()
        ),
    })?;
    let schema = client.data_source(&data_source.id)?;
    let properties = notion::property_kinds(&schema);
    let suggestion = source::suggest(&properties, &data_source.id);
    Ok(DatabaseSchema {
        title: database.title(),
        data_source_id: data_source.id.clone(),
        properties,
        suggestion,
    })
}

/// Holt Schema und alle Seiten einer Quelle.
///
/// Prüft dabei die Zuordnung gegen das Schema: Eine Rolle, deren Spalte es in
/// Notion nicht gibt, ist ein Eingabefehler an `mappings.<rolle>` — mit den
/// vorhandenen Spalten in der Meldung, denn meist ist es ein Tippfehler.
pub fn download<T: Transport>(
    client: &Client<T>,
    source: &Source,
    known_titles: &HashSet<String>,
) -> Result<Download> {
    let database = client.database(&source.database_id)?;
    let data_source = database.data_sources.first().ok_or_else(|| Error::Notion {
        kind: NotionErrorKind::Other,
        status: 200,
        message: format!(
            "„{}\u{201c} hat keine Datenquelle — ist es eine verknüpfte Ansicht statt der Datenbank selbst?",
            database.title()
        ),
    })?;
    if database.data_sources.len() > 1 {
        tracing::warn!(
            database = %database.title(),
            count = database.data_sources.len(),
            "mehrere Datenquellen — es wird die erste benutzt"
        );
    }
    let data_source_id = data_source.id.clone();

    let schema = client.data_source(&data_source_id)?;
    let properties = notion::property_kinds(&schema);
    check_mappings(source, &database, &properties)?;

    let mut pages = client.query_data_source(&data_source_id)?;
    complete_relations(client, &properties, &mut pages)?;
    let titles = foreign_titles(client, &pages, known_titles)?;

    Ok(Download {
        source_id: source.id,
        database,
        data_source_id,
        schema,
        pages,
        titles,
        requests: client.requests(),
    })
}

/// Ersetzt den Zwischenspeicher der Quelle durch den Download — ganz oder gar nicht.
pub fn store(conn: &Connection, download: &Download) -> Result<FetchStatus> {
    // Die Quelle kann während des Downloads gelöscht worden sein.
    let source = source::get(conn, download.source_id)?;
    let id = download.source_id.to_string();
    let fetched_at = timestamp::now();

    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM pages WHERE source_id = ?1", [&id])?;
    {
        let mut insert = tx.prepare(
            "INSERT INTO pages (source_id, page_id, position, title, url, last_edited_time, properties) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for (position, page) in download.pages.iter().enumerate() {
            insert.execute(params![
                id,
                page.id,
                position as i64,
                crate::rows::page_title(page, &source),
                page.url,
                page.last_edited_time,
                Value::Object(page.properties.clone()).to_string(),
            ])?;
        }
    }
    {
        let mut insert = tx.prepare(
            "INSERT INTO page_titles (page_id, title, fetched_at) VALUES (?1, ?2, ?3) \
             ON CONFLICT (page_id) DO UPDATE SET title = excluded.title, \
                                                 fetched_at = excluded.fetched_at",
        )?;
        for (page_id, title) in &download.titles {
            insert.execute(params![page_id, title, timestamp::to_text(fetched_at)])?;
        }
    }
    tx.execute(
        "INSERT INTO fetches (source_id, database_title, database_url, data_source_id, schema, \
                              page_count, request_count, fetched_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
         ON CONFLICT (source_id) DO UPDATE SET \
             database_title = excluded.database_title, database_url = excluded.database_url, \
             data_source_id = excluded.data_source_id, schema = excluded.schema, \
             page_count = excluded.page_count, request_count = excluded.request_count, \
             fetched_at = excluded.fetched_at",
        params![
            id,
            download.database.title(),
            download.database.url,
            download.data_source_id,
            download.schema.to_string(),
            download.pages.len() as i64,
            download.requests,
            timestamp::to_text(fetched_at),
        ],
    )?;
    tx.commit()?;

    tracing::info!(source = %download.source_id, pages = download.pages.len(), "Abruf gespeichert");
    status(conn, download.source_id)?.ok_or(Error::NotFound)
}

/// Token holen, abrufen, speichern — für Aufrufer, die nichts dazwischen tun.
///
/// Die Desktop-Schale ruft [`download`] und [`store`] getrennt, siehe oben.
pub fn fetch_with_http(conn: &Connection, token: &str, source: &Source) -> Result<FetchStatus> {
    let client = Client::new(notion::HttpTransport::new(token));
    let known = known_titles(conn)?;
    store(conn, &download(&client, source, &known)?)
}

/// [`inspect`] mit dem Token aus dem Speicher und echtem Netz.
pub fn inspect_with_http(token: &str, database_id: &str) -> Result<DatabaseSchema> {
    inspect(&Client::new(notion::HttpTransport::new(token)), database_id)
}

/// Seiten, deren Titel schon in der Datenbank stehen — als eigene Seite einer
/// Quelle oder als früher geholtes Relationsziel.
///
/// Wer das beim Abruf mitgibt, spart je bekannter Seite eine Anfrage.
pub fn known_titles(conn: &Connection) -> Result<HashSet<String>> {
    let mut out = HashSet::new();
    for sql in [
        "SELECT page_id FROM pages",
        "SELECT page_id FROM page_titles",
    ] {
        let mut stmt = conn.prepare(sql)?;
        let ids = stmt.query_map([], |r| r.get::<_, String>(0))?;
        for id in ids {
            out.insert(id?);
        }
    }
    Ok(out)
}

/// Eine Quelle mit dem Stand ihres letzten Abrufs — für Listen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct SourceOverview {
    pub source: Source,
    /// `null`: noch nie abgerufen.
    pub fetch: Option<FetchStatus>,
    /// Welche fertigen Ansichten die Zuordnung hergibt — ohne Vorlage.
    pub views: Vec<ViewKind>,
}

/// Eine Ansicht, die sich allein aus der Zuordnung ergibt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ViewKind {
    /// Braucht `next`: Kanten entlang der Nachfolger.
    Flow,
    /// Braucht `date` und `next`: Linien auf einer Zeitachse.
    Metro,
}

/// Was eine Quelle ohne Vorlage zeichnen kann.
pub fn views(source: &Source) -> Vec<ViewKind> {
    let mut out = Vec::new();
    if crate::flow::eligible(source) {
        out.push(ViewKind::Flow);
    }
    if crate::metro::eligible(source) {
        out.push(ViewKind::Metro);
    }
    out
}

/// Alle Quellen, nach Namen sortiert, jeweils mit ihrem letzten Abruf.
pub fn overview(conn: &Connection) -> Result<Vec<SourceOverview>> {
    source::list(conn)?
        .into_iter()
        .map(|source| {
            let fetch = status(conn, source.id)?;
            let views = views(&source);
            Ok(SourceOverview {
                source,
                fetch,
                views,
            })
        })
        .collect()
}

/// `None`: noch nie abgerufen.
pub fn status(conn: &Connection, source_id: Uuid) -> Result<Option<FetchStatus>> {
    let row = conn
        .query_row(
            "SELECT database_title, database_url, page_count, request_count, fetched_at \
             FROM fetches WHERE source_id = ?1",
            [source_id.to_string()],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?;
    row.map(|(title, url, pages, requests, at)| {
        Ok(FetchStatus {
            source_id,
            database_title: title,
            database_url: url,
            page_count: u32::try_from(pages).unwrap_or(u32::MAX),
            request_count: u32::try_from(requests).unwrap_or(u32::MAX),
            fetched_at: timestamp::from_text(&at)?,
        })
    })
    .transpose()
}

/// Die Spalten, die der letzte Abruf in Notion gesehen hat.
///
/// Leer, solange nie abgerufen wurde. Die Oberfläche bietet sie beim Zuordnen
/// zur Auswahl an, statt den Spaltennamen abtippen zu lassen.
pub fn schema_properties(conn: &Connection, source_id: Uuid) -> Result<Vec<notion::Property>> {
    let schema: Option<String> = conn
        .query_row(
            "SELECT schema FROM fetches WHERE source_id = ?1",
            [source_id.to_string()],
            |r| r.get(0),
        )
        .optional()?;
    let Some(schema) = schema else {
        return Ok(Vec::new());
    };
    let value: Value = serde_json::from_str(&schema)
        .map_err(|e| Error::Corrupt(format!("Schema der Quelle {source_id} unlesbar: {e}")))?;
    Ok(notion::property_kinds(&value))
}

/// Die zwischengespeicherten Seiten einer Quelle, in der Reihenfolge der Abfrage.
pub fn pages(conn: &Connection, source_id: Uuid) -> Result<Vec<Page>> {
    let mut stmt = conn.prepare(
        "SELECT page_id, url, last_edited_time, properties FROM pages \
         WHERE source_id = ?1 ORDER BY position",
    )?;
    let rows = stmt
        .query_map([source_id.to_string()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(id, url, last_edited_time, properties)| {
            let properties = match serde_json::from_str(&properties) {
                Ok(Value::Object(map)) => map,
                _ => {
                    return Err(Error::Corrupt(format!(
                        "Eigenschaften der Seite {id} unlesbar"
                    )));
                }
            };
            Ok(Page {
                id,
                url,
                last_edited_time,
                properties,
            })
        })
        .collect()
}

/// Holt die Titel der Relationsziele, die weder in dieser Quelle liegen noch
/// schon bekannt sind.
fn foreign_titles<T: Transport>(
    client: &Client<T>,
    pages: &[Page],
    known: &HashSet<String>,
) -> Result<BTreeMap<String, String>> {
    let own: HashSet<&str> = pages.iter().map(|p| p.id.as_str()).collect();
    let mut wanted: Vec<String> = Vec::new();
    for page in pages {
        for target in crate::rows::relation_targets(page) {
            if !own.contains(target.as_str())
                && !known.contains(&target)
                && !wanted.contains(&target)
            {
                wanted.push(target);
            }
        }
    }

    let mut titles = BTreeMap::new();
    for id in wanted {
        match client.page(&id) {
            Ok(page) => {
                let title = page
                    .properties
                    .values()
                    .find(|v| v["type"] == "title")
                    .map(crate::rows::text_of)
                    .unwrap_or_default();
                titles.insert(id, title);
            }
            // Eine Seite im Papierkorb oder ohne Freigabe soll den ganzen
            // Abruf nicht scheitern lassen — dann steht eben die Kennung da.
            Err(Error::Notion {
                kind: NotionErrorKind::NotShared,
                ..
            }) => {
                tracing::warn!(page = %id, "Relationsziel nicht lesbar");
            }
            Err(e) => return Err(e),
        }
    }
    Ok(titles)
}

fn check_mappings(
    source: &Source,
    database: &Database,
    properties: &[notion::Property],
) -> Result<()> {
    let mut v = Validator::new();
    for m in &source.mappings {
        if !properties.iter().any(|p| p.name == m.property) {
            let available: Vec<&str> = properties.iter().map(|p| p.name.as_str()).collect();
            v.add(
                format!("mappings.{}", m.role),
                format!(
                    "Spalte „{}\u{201c} gibt es in „{}\u{201c} nicht (vorhanden: {})",
                    m.property,
                    database.title(),
                    available.join(", ")
                ),
            );
        }
    }
    v.finish()
}

/// Lädt Relationen nach, die Notion wegen `has_more` gekürzt hat.
fn complete_relations<T: Transport>(
    client: &Client<T>,
    properties: &[notion::Property],
    pages: &mut [Page],
) -> Result<()> {
    for page in pages.iter_mut() {
        for (name, value) in page.properties.iter_mut() {
            if value["type"] != "relation" || value["has_more"] != json!(true) {
                continue;
            }
            // Die Kurzkennung steht auch in der Eigenschaft selbst; das Schema
            // ist die Rückfallebene.
            let property_id = value["id"]
                .as_str()
                .map(String::from)
                .or_else(|| {
                    properties
                        .iter()
                        .find(|p| &p.name == name)
                        .map(|p| p.id.clone())
                })
                .unwrap_or_default();
            let ids = client.relation_ids(&page.id, &property_id)?;
            tracing::debug!(page = %page.id, property = %name, count = ids.len(), "Relation nachgeladen");
            value["relation"] =
                Value::Array(ids.into_iter().map(|id| json!({ "id": id })).collect());
            value["has_more"] = json!(false);
        }
    }
    Ok(())
}
