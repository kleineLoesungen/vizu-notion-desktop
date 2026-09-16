//! Fachlogik von vizu-notion: Quellen, Notion-Abruf, Zwischenspeicher.
//!
//! # Die eine wichtige Regel
//!
//! Diese Kiste kennt **weder Kommandozeile noch Oberfläche noch stdout**.
//! Sie nimmt Werte entgegen, gibt Werte oder [`Error`] zurück und schreibt
//! nichts auf den Bildschirm. Wer hier `clap`, `tauri`, `println!` oder
//! `std::process::exit` einbaut, bringt `crates/core/tests/layering.rs` zum
//! Fehlschlagen — und das ist der Zweck dieses Tests.
//!
//! Der Grund ist nicht Sauberkeit um ihrer selbst willen: Alles, was hier
//! steht, gilt für die Kommandozeile **und** die Oberfläche gleichermaßen.
//! Eine Regel, die in der einen Schale steht, fehlt der anderen.
//!
//! # Einstieg
//!
//! ```no_run
//! use vizu_notion_core::{App, Paths, fetch, source::{self, ColumnMapping, SourceInput}};
//!
//! let app = App::open(Paths::resolve()?)?;
//! let projekte = source::create(
//!     app.conn(),
//!     SourceInput::new("Projekte", "396f66270f5d8034b55cebc685aa5e50",
//!                      vec![ColumnMapping::new("title", "Name")]),
//! )?;
//! let status = fetch::fetch_with_http(app.conn(), app.secrets(), &projekte)?;
//! # Ok::<(), vizu_notion_core::Error>(())
//! ```

pub mod config;
pub mod db;
pub mod error;
pub mod fetch;
pub mod flow;
pub mod ids;
pub mod notion;
pub mod paths;
pub mod rows;
pub mod secret;
pub mod source;
pub mod template;
pub mod timestamp;

pub use config::{Config, Theme};
pub use error::{Error, ErrorCode, FieldError, NotionErrorKind, Result, ValidationError};
pub use paths::Paths;

use rusqlite::Connection;

/// Alles, was eine laufende Anwendung braucht: Pfade, Einstellungen,
/// Datenbank und der Speicher für den Notion-Token.
///
/// Beide Schalen bauen sich beim Start genau eines davon.
pub struct App {
    paths: Paths,
    config: Config,
    conn: Connection,
    secrets: Box<dyn secret::SecretStore>,
}

impl App {
    /// Legt Verzeichnisse an, liest die Einstellungen und öffnet die Datenbank.
    ///
    /// Der Token liegt im Schlüsselbund, außer `VIZU_NOTION_TOKEN_FILE` ist
    /// gesetzt — siehe [`secret::default_store`].
    pub fn open(paths: Paths) -> Result<Self> {
        Self::open_with(paths, secret::default_store())
    }

    /// Wie [`App::open`], mit ausdrücklich gewähltem Token-Speicher.
    pub fn open_with(paths: Paths, secrets: Box<dyn secret::SecretStore>) -> Result<Self> {
        paths.ensure()?;
        let config = Config::load(&paths.config_file())?;
        let conn = db::open(&paths.db_file())?;
        Ok(Self {
            paths,
            config,
            conn,
            secrets,
        })
    }

    /// Eine Anwendung ohne Datei auf der Platte, für Tests.
    ///
    /// Die Pfade zeigen ins Leere — wer in einem Test Dateien braucht, nimmt
    /// [`App::open`] mit [`Paths::under`] auf einem `tempfile::TempDir`.
    pub fn in_memory() -> Result<Self> {
        Ok(Self {
            paths: Paths::under(std::env::temp_dir().join("vizu-notion-in-memory")),
            config: Config::default(),
            conn: db::open_in_memory()?,
            secrets: Box::new(secret::MemoryStore::default()),
        })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn secrets(&self) -> &dyn secret::SecretStore {
        self.secrets.as_ref()
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Prüft, speichert und übernimmt neue Einstellungen.
    ///
    /// Bei ungültigen Werten bleibt der alte Stand unverändert — eine
    /// halbübernommene Konfiguration wäre schlimmer als eine abgelehnte.
    pub fn set_config(&mut self, config: Config) -> Result<()> {
        config.validate()?;
        self.paths.ensure()?;
        config.save(&self.paths.config_file())?;
        self.config = config;
        Ok(())
    }
}
