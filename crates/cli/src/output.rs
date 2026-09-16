//! Ausgabe — für Menschen oder als JSON.
//!
//! **Jeder Befehl kann beides.** Wer einen neuen Unterbefehl schreibt, gibt
//! ihm beide Fassungen; ein `--json`, das bei manchen Befehlen nichts tut, ist
//! schlimmer als keins, weil ein Skript es nicht vorhersehen kann.
//!
//! Regeln für die JSON-Fassung:
//!
//! * Listen sind ein Array, kein Objekt mit Zähler — `jq '.[] | .source.name'`
//!   soll ohne Umweg gehen.
//! * Zeitstempel in UTC als RFC-3339-Text (macht `vizu_notion_core` schon).
//! * Für Menschen wird in Ortszeit umgerechnet, für Maschinen nie.
//! * Der Notion-Token erscheint in keiner Fassung — höchstens sein Hinweis.

use std::time::Duration;

use time::macros::format_description;
use vizu_notion_core::fetch::{FetchStatus, SourceOverview};
use vizu_notion_core::secret::{TokenOrigin, TokenStatus};
use vizu_notion_core::source::Source;
use vizu_notion_core::template::{Diagram, Template};
use vizu_notion_core::{Config, Paths, ids, timestamp};

pub struct Out {
    pub json: bool,
}

impl Out {
    fn local(t: time::OffsetDateTime) -> String {
        let fmt = format_description!("[day].[month].[year] [hour]:[minute]");
        timestamp::to_local(t)
            .format(fmt)
            .unwrap_or_else(|_| timestamp::to_text(t))
    }

    pub fn sources(&self, sources: &[SourceOverview]) {
        if self.json {
            self.print(sources);
            return;
        }
        if sources.is_empty() {
            println!("Keine Quellen. Anlegen mit:");
            println!("  vizu-notion source add Projekte --database <ID> --map title=Name");
            println!("  vizu-notion source import sources.json");
            return;
        }
        // Die Spaltenbreite richtet sich nach dem längsten Namen, gedeckelt,
        // damit ein Ausreißer die Tabelle nicht sprengt.
        let width = sources
            .iter()
            .map(|s| s.source.name.chars().count())
            .max()
            .unwrap_or(4)
            .clamp(4, 40);

        println!(
            "{:<8}  {:<width$}  {:>6}  ABGERUFEN",
            "ID", "NAME", "SEITEN"
        );
        for s in sources {
            let (pages, at) = match &s.fetch {
                Some(f) => (f.page_count.to_string(), Self::local(f.fetched_at)),
                None => ("—".to_string(), "noch nie".to_string()),
            };
            println!(
                "{:<8}  {:<width$}  {:>6}  {}",
                ids::short_id(s.source.id),
                truncate(&s.source.name, width),
                pages,
                at
            );
        }
    }

    pub fn source(&self, source: &Source, fetch: Option<&FetchStatus>) {
        if self.json {
            self.print(&serde_json::json!({ "source": source, "fetch": fetch }));
            return;
        }
        println!("{}", source.name);
        println!("{}", "─".repeat(source.name.chars().count().max(3)));
        println!("Datenbank:  {}", source.database_id);
        if source.mappings.is_empty() {
            println!("Zuordnung:  keine");
        } else {
            let width = source
                .mappings
                .iter()
                .map(|m| m.role.chars().count())
                .max()
                .unwrap_or(0);
            println!("Zuordnung:");
            for m in &source.mappings {
                println!("  {:<width$}  →  {}", m.role, m.property);
            }
        }
        match fetch {
            Some(f) => {
                println!(
                    "Abgerufen:  {}, {} Seiten aus „{}\u{201c}",
                    Self::local(f.fetched_at),
                    f.page_count,
                    f.database_title
                );
                println!("In Notion:  {}", f.database_url);
            }
            None => println!("Abgerufen:  noch nie"),
        }
        println!("Kennung:    {}", source.id);
    }

    pub fn fetched_one(&self, name: &str, status: &FetchStatus, took: Duration) {
        println!(
            "{name}: {} Seiten aus „{}\u{201c} ({} Anfragen, {:.1} s)",
            status.page_count,
            status.database_title,
            status.request_count,
            took.as_secs_f32()
        );
    }

    pub fn fetched(&self, statuses: &[FetchStatus]) {
        self.print(statuses);
    }

    pub fn templates(&self, templates: &[Template]) {
        if self.json {
            self.print(templates);
            return;
        }
        if templates.is_empty() {
            println!("Keine Vorlagen. Einlesen mit:  vizu-notion template import diagramm.mmd");
            return;
        }
        let width = templates
            .iter()
            .map(|t| t.slug.chars().count())
            .max()
            .unwrap_or(8)
            .clamp(8, 30);
        println!("{:<width$}  {:<28}  QUELLEN", "KURZNAME", "TITEL");
        for t in templates {
            println!(
                "{:<width$}  {:<28}  {}",
                truncate(&t.slug, width),
                truncate(&t.title, 28),
                t.sources.join(", ")
            );
        }
    }

    pub fn template(&self, template: &Template) {
        if self.json {
            self.print(template);
            return;
        }
        println!("{}", template.title);
        println!("{}", "─".repeat(template.title.chars().count().max(3)));
        println!("Kurzname:  {}", template.slug);
        println!("Quellen:   {}", template.sources.join(", "));
        println!("Kennung:   {}", template.id);
        println!();
        println!("{}", template.body.trim_end());
    }

    /// Der Mermaid-Text geht roh auf stdout — damit
    /// `vizu-notion render x > diagramm.mmd` das Richtige tut.
    pub fn diagram(&self, diagram: &Diagram) {
        if self.json {
            self.print(diagram);
            return;
        }
        println!("{}", diagram.mermaid);
    }

    pub fn token_status(&self, status: &TokenStatus) {
        if self.json {
            self.print(status);
            return;
        }
        match (status.origin, &status.hint) {
            (Some(TokenOrigin::Environment), Some(hint)) => {
                println!("Token:     {hint}  (aus VIZU_NOTION_TOKEN)");
            }
            (Some(TokenOrigin::Store), Some(hint)) => {
                println!("Token:     {hint}  (gespeichert)");
            }
            _ => println!("Token:     keiner — speichern mit  vizu-notion token set"),
        }
        println!("Speicher:  {}", status.store);
        if let Some(err) = &status.store_error {
            println!("Problem:   {err}");
        }
    }

    pub fn config(&self, config: &Config) {
        if self.json {
            self.print(config);
            return;
        }
        println!("app_name  {}", config.app_name);
        println!("theme     {:?}  ({})", config.theme, config.theme.label());
        println!("accent    {}", config.accent);
    }

    pub fn paths(&self, paths: &Paths) {
        if self.json {
            self.print(&serde_json::json!({
                "data_dir": paths.data_dir(),
                "config_dir": paths.config_dir(),
                "db_file": paths.db_file(),
                "config_file": paths.config_file(),
                "log_file": paths.log_file(),
            }));
            return;
        }
        println!("Daten          {}", paths.data_dir().display());
        println!("Konfiguration  {}", paths.config_dir().display());
        println!("Datenbank      {}", paths.db_file().display());
        println!("Protokoll      {}", paths.log_file().display());
    }

    /// Eine Rückmeldung nach einer Änderung.
    ///
    /// Im JSON-Betrieb wird das betroffene Objekt ausgegeben, nicht der Satz —
    /// ein Skript will den Datensatz, keinen deutschen Text.
    pub fn done(&self, message: impl std::fmt::Display, value: &impl serde::Serialize) {
        if self.json {
            self.print(value);
        } else {
            println!("{message}");
        }
    }

    fn print(&self, value: &(impl serde::Serialize + ?Sized)) {
        let text = serde_json::to_string_pretty(value).expect("Ausgabe serialisierbar");
        println!("{text}");
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}
