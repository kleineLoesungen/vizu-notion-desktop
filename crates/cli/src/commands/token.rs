use std::io::{IsTerminal, Read};

use anyhow::{Context, Result};
use vizu_notion_core::{App, secret};

use crate::args::TokenCommand;
use crate::output::Out;

pub fn run(cmd: TokenCommand, app: &App, out: &Out) -> Result<()> {
    match cmd {
        TokenCommand::Set => {
            let raw = read_token()?;
            let status = secret::set(app.secrets(), &raw)?;
            out.done(
                format!(
                    "Gespeichert ({}): {}",
                    status.store,
                    status.hint.as_deref().unwrap_or("…")
                ),
                &status,
            );
            if std::env::var_os(secret::ENV_TOKEN).is_some() && !out.json {
                eprintln!(
                    "Hinweis: {} ist gesetzt und hat Vorrang vor dem gespeicherten Token.",
                    secret::ENV_TOKEN
                );
            }
            Ok(())
        }

        TokenCommand::Status => {
            out.token_status(&app.token_status());
            Ok(())
        }

        TokenCommand::Clear => {
            let removed = secret::clear(app.secrets())?;
            let message = if removed {
                "Gespeicherten Token gelöscht."
            } else {
                "Es war kein Token gespeichert."
            };
            out.done(message, &serde_json::json!({ "removed": removed }));
            Ok(())
        }
    }
}

/// Im Terminal verdeckt nachfragen, sonst stdin lesen.
fn read_token() -> Result<String> {
    if std::io::stdin().is_terminal() {
        rpassword::prompt_password("Notion-Token (Eingabe bleibt unsichtbar): ")
            .context("Token lesen")
    } else {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("Token von stdin lesen")?;
        Ok(buf)
    }
}
