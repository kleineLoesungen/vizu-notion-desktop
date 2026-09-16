use std::collections::HashSet;

use anyhow::Result;
use vizu_notion_core::{App, flow, source};

use crate::output::Out;

/// Zeichnet den Fluss einer Quelle aus dem Zwischenspeicher — ohne Netz.
pub fn run(
    needle: &str,
    subtitle: Option<&str>,
    hidden: &[String],
    app: &App,
    out: &Out,
) -> Result<()> {
    let id = source::resolve(app.conn(), needle)?;
    let found = source::get(app.conn(), id)?;
    if !flow::eligible(&found) {
        // Kein Fehler des Programms, sondern der Zuordnung — deshalb ein
        // Eingabefehler mit dem Feld, an dem es fehlt.
        let mut v = vizu_notion_core::error::Validator::new();
        v.add(
            "mappings.next",
            format!(
                "„{}\u{201c} hat keine Rolle `next` — ohne sie gibt es keine Kanten",
                found.name
            ),
        );
        v.finish()?;
    }
    let hidden: HashSet<String> = hidden.iter().map(|h| h.trim().to_string()).collect();
    out.flow(&flow::build(app.conn(), id, &hidden, subtitle)?);
    Ok(())
}
