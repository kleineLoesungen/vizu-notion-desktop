//! Rückgabewerte und Fehlerausgabe.
//!
//! Ein Skript soll auf den Rückgabewert reagieren können, ohne die Meldung zu
//! lesen. Deshalb hat jede Fehlerart ihre eigene Nummer, und die Nummern
//! stehen in docs/CLI.md.
//!
//! | Wert | Bedeutung |
//! |---|---|
//! | 0 | in Ordnung |
//! | 1 | allgemeiner Fehler (Platte, Datenbank, …) |
//! | 2 | falscher Aufruf — vergibt clap selbst |
//! | 3 | nicht gefunden |
//! | 4 | Eingabe ungültig |
//! | 5 | Kennung nicht eindeutig |
//! | 6 | Notion nicht erreichbar oder Fehler von Notion |
//! | 7 | kein Token, oder Notion lehnt ihn ab |

use std::process::ExitCode;

use vizu_notion_core::Error as CoreError;

pub const FAILURE: u8 = 1;
pub const NOT_FOUND: u8 = 3;
pub const INVALID_INPUT: u8 = 4;
pub const AMBIGUOUS: u8 = 5;
pub const NOTION: u8 = 6;
pub const TOKEN: u8 = 7;

fn exit_code(err: &CoreError) -> u8 {
    use vizu_notion_core::ErrorCode as C;
    match err.code() {
        C::NotFound => NOT_FOUND,
        C::ValidationFailed => INVALID_INPUT,
        C::AmbiguousId => AMBIGUOUS,
        C::NotionUnreachable | C::NotionError | C::NotionNotShared => NOTION,
        C::TokenMissing | C::NotionUnauthorized => TOKEN,
        C::ConfigInvalid | C::InternalError | C::SecretStoreUnavailable => FAILURE,
    }
}

/// Meldet den Fehler auf stderr und liefert den passenden Rückgabewert.
pub fn report(err: &anyhow::Error, json: bool) -> ExitCode {
    // Nur ein Fehler der Fachlogik bekommt eine eigene Nummer. Alles andere
    // ist ein Fehler des Programms und damit schlicht 1.
    let core = err.downcast_ref::<CoreError>();

    if json {
        let mut obj = serde_json::json!({
            "code": core.map_or("internal_error", |e| e.code().as_str()),
            "message": err.to_string(),
        });
        if let Some(fields) = core.and_then(CoreError::fields) {
            obj["fields"] = serde_json::to_value(fields).expect("Feldfehler serialisierbar");
        }
        if let Some(CoreError::Ambiguous { matches, .. }) = core {
            obj["matches"] = serde_json::to_value(matches).expect("Treffer serialisierbar");
        }
        let envelope = serde_json::json!({ "error": obj });
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&envelope).expect("Hülle serialisierbar")
        );
    } else {
        eprintln!("Fehler: {err}");
        // Die Ursachenkette zeigen — sonst steht da nur "Datenbankfehler"
        // ohne den eigentlichen Grund.
        for cause in err.chain().skip(1) {
            eprintln!("  Ursache: {cause}");
        }
        if let Some(CoreError::Ambiguous { matches, .. }) = core {
            for m in matches {
                eprintln!("  {m}");
            }
        }
    }

    ExitCode::from(core.map_or(FAILURE, exit_code))
}
