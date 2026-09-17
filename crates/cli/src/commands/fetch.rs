use std::time::Instant;

use anyhow::Result;
use vizu_notion_core::{App, fetch, source};

use crate::output::Out;

/// Ruft die genannten Quellen ab, ohne Angabe alle. Bricht beim ersten Fehler
/// ab — was bis dahin abgerufen ist, bleibt gespeichert.
pub fn run(needles: Vec<String>, app: &App, out: &Out) -> Result<()> {
    let sources = if needles.is_empty() {
        source::list(app.conn())?
    } else {
        needles
            .iter()
            .map(|n| source::get(app.conn(), source::resolve(app.conn(), n)?))
            .collect::<vizu_notion_core::Result<Vec<_>>>()?
    };

    if sources.is_empty() {
        if !out.json {
            println!("Keine Quellen. Anlegen mit:  vizu-notion source add NAME --database ID");
        } else {
            out.fetched(&[]);
        }
        return Ok(());
    }

    let mut done = Vec::with_capacity(sources.len());
    for s in &sources {
        if !out.json {
            // Fortschritt auf stderr — stdout bleibt die Nutzausgabe.
            eprintln!("Rufe „{}\u{201c} ab …", s.name);
        }
        let started = Instant::now();
        let status = fetch::fetch_with_http(app.conn(), &app.token()?, s)?;
        if !out.json {
            out.fetched_one(&s.name, &status, started.elapsed());
        }
        done.push(status);
    }
    if out.json {
        out.fetched(&done);
    }
    Ok(())
}
