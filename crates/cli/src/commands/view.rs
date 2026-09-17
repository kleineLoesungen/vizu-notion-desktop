use anyhow::{Result, bail};
use vizu_notion_core::view::{self, ViewInput};
use vizu_notion_core::{App, flow, ids, metro, source, template};

use crate::args::ViewCommand;
use crate::commands::confirm;
use crate::output::Out;

pub fn run(cmd: ViewCommand, app: &App, out: &Out) -> Result<()> {
    match cmd {
        ViewCommand::List => {
            out.views(&view::list(app.conn())?);
            Ok(())
        }

        ViewCommand::Show { view: needle } => {
            let id = view::resolve(app.conn(), &needle)?;
            let found = view::get(app.conn(), id)?;
            draw(&found, app, out)
        }

        ViewCommand::Add {
            name,
            template: as_template,
            flow: as_flow,
            metro: as_metro,
            hidden,
            sub,
        } => {
            // Was gezeichnet wird, steht in genau einer der drei Optionen;
            // clap sorgt über `group` dafür, dass es nicht zwei sind.
            let (kind, needle) = match (as_template, as_flow, as_metro) {
                (Some(t), None, None) => ("template", t),
                (None, Some(f), None) => ("flow", f),
                (None, None, Some(m)) => ("metro", m),
                _ => bail!(
                    "Was soll die Ansicht zeigen? Eines von --template, --flow oder --metro angeben."
                ),
            };
            // Der Name in der Ansicht wäre morgen vielleicht ein anderer —
            // gespeichert wird die Kennung.
            let target = match kind {
                "template" => template::resolve(app.conn(), &needle)?,
                _ => source::resolve(app.conn(), &needle)?,
            };
            let saved = view::create(
                app.conn(),
                ViewInput::new(name, kind, target.to_string())
                    .hiding(hidden)
                    .with_subtitle(sub),
            )?;
            out.done(
                format!(
                    "Gespeichert: {}  {}\nZeichnen mit:  vizu-notion view show {}",
                    ids::short_id(saved.id),
                    saved.name,
                    shell_word(&saved.name)
                ),
                &saved,
            );
            Ok(())
        }

        ViewCommand::Rm { view: needle, yes } => {
            let id = view::resolve(app.conn(), &needle)?;
            let found = view::get(app.conn(), id)?;
            if !yes && !confirm(&format!("Ansicht „{}\u{201c} löschen?", found.name))? {
                println!("Abgebrochen.");
                return Ok(());
            }
            view::delete(app.conn(), id)?;
            out.done(format!("Gelöscht: {}", found.name), &found);
            Ok(())
        }
    }
}

/// Zeichnet, was die Ansicht festhält — dieselben Funktionen wie `render`,
/// `flow` und `metro`, damit eine Ansicht nichts anderes zeigt als diese.
fn draw(saved: &view::View, app: &App, out: &Out) -> Result<()> {
    let hidden = saved.hidden_set();
    let id = saved.target_id()?;
    match saved.kind.as_str() {
        "template" => {
            let found = template::get(app.conn(), id)?;
            out.diagram(&template::render(app.conn(), &found, &hidden)?);
        }
        "flow" => out.flow(&flow::build(
            app.conn(),
            id,
            &hidden,
            saved.subtitle.as_deref(),
        )?),
        _ => out.metro(&metro::build(app.conn(), id, &hidden)?),
    }
    Ok(())
}

/// Ein Name mit Leerzeichen muss in der Beispielzeile in Anführungszeichen.
fn shell_word(name: &str) -> String {
    if name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        name.to_string()
    } else {
        format!("\"{}\"", name.replace('"', "\\\""))
    }
}
