use std::io::Read;

use anyhow::{Context, Result};
use vizu_notion_core::source::{self, ColumnMapping, SourceInput};
use vizu_notion_core::{App, fetch, ids};

use crate::args::SourceCommand;
use crate::commands::confirm;
use crate::output::Out;

pub fn run(cmd: SourceCommand, app: &App, out: &Out) -> Result<()> {
    match cmd {
        SourceCommand::List => {
            out.sources(&fetch::overview(app.conn())?);
            Ok(())
        }

        SourceCommand::Show { source: needle } => {
            let id = source::resolve(app.conn(), &needle)?;
            let found = source::get(app.conn(), id)?;
            let status = fetch::status(app.conn(), id)?;
            out.source(&found, status.as_ref());
            Ok(())
        }

        SourceCommand::Add {
            name,
            database,
            mappings,
        } => {
            let created = source::create(
                app.conn(),
                SourceInput::new(name, database, to_mappings(mappings)),
            )?;
            out.done(
                format!(
                    "Angelegt: {}  {}\nAbrufen mit:  vizu-notion fetch {}",
                    ids::short_id(created.id),
                    created.name,
                    shell_word(&created.name)
                ),
                &created,
            );
            Ok(())
        }

        SourceCommand::Edit {
            source: needle,
            name,
            database,
            mappings,
            unmap,
        } => {
            let id = source::resolve(app.conn(), &needle)?;
            let current = source::get(app.conn(), id)?;

            // Die Zuordnung zusammenstellen ist Bedienung, keine Regel: Was
            // davon gültig ist, entscheidet core beim Speichern.
            let mut merged: Vec<ColumnMapping> = current
                .mappings
                .iter()
                .filter(|m| !unmap.contains(&m.role))
                .filter(|m| !mappings.iter().any(|(role, _)| role.trim() == m.role))
                .cloned()
                .collect();
            merged.extend(to_mappings(mappings));

            let input = SourceInput::new(
                name.unwrap_or(current.name),
                database.unwrap_or(current.database_id),
                merged,
            );
            let updated = source::update(app.conn(), id, input)?;
            out.done(format!("Geändert: {}", updated.name), &updated);
            Ok(())
        }

        SourceCommand::Rm {
            source: needle,
            yes,
        } => {
            let id = source::resolve(app.conn(), &needle)?;
            let existing = source::get(app.conn(), id)?;

            if !yes
                && !confirm(&format!(
                    "Quelle „{}\u{201c} und ihre abgerufenen Daten löschen?",
                    existing.name
                ))?
            {
                // Abbruch auf Wunsch des Benutzers ist kein Fehler.
                if !out.json {
                    println!("Abgebrochen.");
                }
                return Ok(());
            }

            source::delete(app.conn(), id)?;
            out.done(format!("Gelöscht: {}", existing.name), &existing);
            Ok(())
        }

        SourceCommand::Import { file } => {
            let text = if file.as_os_str() == "-" {
                let mut buf = String::new();
                std::io::stdin()
                    .read_to_string(&mut buf)
                    .context("sources.json von stdin lesen")?;
                buf
            } else {
                std::fs::read_to_string(&file)
                    .with_context(|| format!("{} lesen", file.display()))?
            };
            let imported = source::import_webapp_json(app.conn(), &text)?;
            let names: Vec<&str> = imported.iter().map(|s| s.name.as_str()).collect();
            out.done(
                format!("{} Quellen angelegt: {}", imported.len(), names.join(", ")),
                &imported,
            );
            Ok(())
        }
    }
}

fn to_mappings(pairs: Vec<(String, String)>) -> Vec<ColumnMapping> {
    pairs
        .into_iter()
        .map(|(role, property)| ColumnMapping::new(role, property))
        .collect()
}

/// Für den Hinweis in der Ausgabe: Namen mit Leerzeichen in Anführungszeichen.
fn shell_word(name: &str) -> String {
    if name
        .chars()
        .all(|c| c.is_alphanumeric() || "-_.".contains(c))
    {
        name.to_string()
    } else {
        format!("\"{}\"", name.replace('"', "\\\""))
    }
}
