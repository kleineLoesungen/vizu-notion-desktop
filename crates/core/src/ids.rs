//! Kurzkennungen für die Bedienung.

use rusqlite::Connection;
use uuid::Uuid;

use crate::error::{Error, Result};

/// Länge der Kurzform. Acht Hexziffern sind 32 Bit Zufall — für die Quellen
/// auf einem Arbeitsplatzrechner reichlich.
pub const SHORT_ID_LEN: usize = 8;

/// Die Kurzform einer Kennung: die letzten acht Zeichen.
///
/// **Nicht die ersten.** Eine UUIDv7 beginnt mit dem Zeitstempel in
/// Millisekunden — zwei Datensätze aus derselben Sekunde teilen sich die
/// ersten zwölf Zeichen. Zufällig ist erst das Ende.
pub fn short_id(id: Uuid) -> String {
    let full = id.to_string();
    full[full.len() - SHORT_ID_LEN..].to_string()
}

/// Findet die Kennung zu einem eindeutigen Endstück in `table`.
///
/// Ist es nicht eindeutig, gibt es [`Error::Ambiguous`] statt eines zufälligen
/// Treffers — „irgendeine davon" wäre bei `rm` die falsche Antwort.
///
/// `table` ist immer ein fester Name aus dem Code, nie eine Eingabe.
pub(crate) fn resolve_suffix(conn: &Connection, table: &'static str, needle: &str) -> Result<Uuid> {
    let needle = needle.trim();
    // Ein UUID-Stück besteht aus Hexziffern und Bindestrichen. Alles andere
    // wird abgewiesen, statt es für LIKE zu entschärfen — damit kann aus einem
    // "%" kein Joker werden, der versehentlich alles trifft.
    if needle.is_empty() || !needle.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return Err(Error::NotFound);
    }
    let sql = format!("SELECT id FROM {table} WHERE id LIKE ?1 ORDER BY id LIMIT 10");
    let mut stmt = conn.prepare(&sql)?;
    let matches: Vec<String> = stmt
        .query_map([format!("%{}", needle.to_ascii_lowercase())], |r| {
            r.get::<_, String>(0)
        })?
        .collect::<std::result::Result<_, _>>()?;

    match matches.as_slice() {
        [] => Err(Error::NotFound),
        [one] => parse_uuid(one),
        _ => Err(Error::Ambiguous {
            prefix: needle.to_string(),
            matches,
        }),
    }
}

pub(crate) fn parse_uuid(text: &str) -> Result<Uuid> {
    Uuid::parse_str(text).map_err(|e| Error::Corrupt(format!("id {text:?} ist keine UUID: {e}")))
}
