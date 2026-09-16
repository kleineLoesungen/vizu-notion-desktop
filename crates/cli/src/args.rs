//! Die Befehlsstruktur.
//!
//! Hier steht **nur**, wie Argumente aussehen. Was passiert, steht in
//! `commands/`. Die Trennung hält diese Datei lesbar und erlaubt es, die
//! Hilfetexte zu überfliegen, ohne Logik zu lesen.
//!
//! Hilfetexte sind deutsch, Bezeichner und Optionsnamen englisch — wie überall
//! im Kit.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "vizu-notion",
    version,
    about = "Notion-Datenbanken als Diagramme",
    long_about = "Verwaltet Notion-Quellen und ruft ihre Daten ab. Dieselbe Fachlogik \
                  und dieselbe Datenbank benutzt die Oberfläche `vizu-notion-desktop`.",
    propagate_version = true
)]
pub struct Cli {
    /// Ausgabe als JSON statt für Menschen.
    ///
    /// Gilt auch für Fehler: dann steht auf stderr ein JSON-Objekt mit
    /// `error.code`, `error.message` und gegebenenfalls `error.fields`.
    #[arg(long, global = true)]
    pub json: bool,

    /// Mehr Protokoll auf stderr. Zweimal für noch mehr.
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Datenverzeichnis überschreiben (sonst der Ort der Plattform).
    #[arg(
        long,
        global = true,
        value_name = "VERZEICHNIS",
        env = "VIZU_NOTION_DATA_DIR"
    )]
    pub data_dir: Option<PathBuf>,

    /// Konfigurationsverzeichnis überschreiben.
    #[arg(
        long,
        global = true,
        value_name = "VERZEICHNIS",
        env = "VIZU_NOTION_CONFIG_DIR"
    )]
    pub config_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Quellen verwalten: welche Notion-Datenbank unter welchem Namen.
    #[command(subcommand)]
    Source(SourceCommand),

    /// Daten von Notion abrufen und zwischenspeichern.
    ///
    /// Ohne Angabe werden alle Quellen abgerufen. Bricht beim ersten Fehler ab.
    Fetch {
        /// Name oder Kennung der Quelle.
        #[arg(value_name = "QUELLE")]
        sources: Vec<String>,
    },

    /// Den Fluss einer Quelle zeichnen — ohne Vorlage.
    ///
    /// Braucht eine Rolle `next`. Die Anordnung ist dieselbe wie im Fenster.
    Flow {
        /// Name oder Kennung der Quelle.
        #[arg(value_name = "QUELLE")]
        source: String,

        /// Rolle, die unter dem Titel steht (`status`, `date`, …).
        #[arg(long, value_name = "ROLLE")]
        sub: Option<String>,

        /// Seiten-IDs, die nicht gezeichnet werden. Mehrfach oder mit Komma.
        #[arg(long = "hide", value_name = "SEITEN-ID", value_delimiter = ',')]
        hidden: Vec<String>,
    },

    /// Die Metro-Karte einer Quelle zeichnen — ohne Vorlage.
    ///
    /// Braucht die Rollen `date` und `next`.
    Metro {
        /// Name oder Kennung der Quelle.
        #[arg(value_name = "QUELLE")]
        source: String,

        /// Seiten-IDs, die nicht gezeichnet werden. Mehrfach oder mit Komma.
        #[arg(long = "hide", value_name = "SEITEN-ID", value_delimiter = ',')]
        hidden: Vec<String>,
    },

    /// Mermaid-Vorlagen verwalten.
    #[command(subcommand)]
    Template(TemplateCommand),

    /// Eine Vorlage zeichnen und den Mermaid-Text ausgeben.
    ///
    /// Der Text ist derselbe, den auch die Oberfläche an mermaid.js gibt.
    Render {
        /// Kurzname oder Kennung der Vorlage.
        #[arg(value_name = "VORLAGE")]
        template: String,

        /// Seiten-IDs, die nicht gezeichnet werden. Mehrfach oder mit Komma.
        #[arg(long = "hide", value_name = "SEITEN-ID", value_delimiter = ',')]
        hidden: Vec<String>,
    },

    /// Den Notion-Token speichern, prüfen oder löschen.
    #[command(subcommand)]
    Token(TokenCommand),

    /// Einstellungen anzeigen und ändern.
    #[command(subcommand)]
    Config(ConfigCommand),

    /// Zeigt, wo die Anwendung ihre Dateien ablegt.
    Paths,

    /// Vervollständigung für die Shell ausgeben.
    ///
    /// Beispiel: `vizu-notion completions zsh > ~/.zfunc/_vizu-notion`
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Handbuchseiten nach VERZEICHNIS schreiben.
    Man {
        #[arg(long, value_name = "VERZEICHNIS", default_value = "dist/man")]
        out: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum SourceCommand {
    /// Alle Quellen mit dem Stand ihres letzten Abrufs.
    #[command(alias = "ls")]
    List,

    /// Eine Quelle vollständig anzeigen.
    Show {
        /// Name, Kennung oder ein eindeutiges Ende der Kennung.
        #[arg(value_name = "QUELLE")]
        source: String,
    },

    /// Neue Quelle anlegen.
    ///
    /// Beispiel: `vizu-notion source add Projekte --database 396f6627… --map title=Name --map next=Nächstes`
    Add {
        /// Unter diesem Namen benutzen Vorlagen die Quelle.
        name: String,

        /// Kennung oder Adresse der Notion-Datenbank.
        #[arg(long, value_name = "ID")]
        database: String,

        /// Rolle einer Spalte zuordnen, mehrfach möglich.
        #[arg(long = "map", value_name = "ROLLE=SPALTE", value_parser = parse_mapping)]
        mappings: Vec<(String, String)>,
    },

    /// Eine Quelle ändern. Nicht Angegebenes bleibt, wie es ist.
    Edit {
        #[arg(value_name = "QUELLE")]
        source: String,

        #[arg(long)]
        name: Option<String>,

        #[arg(long, value_name = "ID")]
        database: Option<String>,

        /// Rolle zuordnen oder umhängen, mehrfach möglich.
        #[arg(long = "map", value_name = "ROLLE=SPALTE", value_parser = parse_mapping)]
        mappings: Vec<(String, String)>,

        /// Zuordnung einer Rolle entfernen, mehrfach möglich.
        #[arg(long = "unmap", value_name = "ROLLE")]
        unmap: Vec<String>,
    },

    /// Eine Quelle samt zwischengespeicherten Daten löschen.
    #[command(alias = "delete")]
    Rm {
        #[arg(value_name = "QUELLE")]
        source: String,

        /// Nicht nachfragen.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Quellen aus der `sources.json` der Webapp vizu-notion-local anlegen.
    ///
    /// Alle oder keine: Ist ein Eintrag ungültig, wird nichts angelegt.
    Import {
        /// Pfad zur Datei, `-` für stdin.
        #[arg(value_name = "DATEI")]
        file: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum TemplateCommand {
    /// Alle Vorlagen mit Titel und Quellen.
    #[command(alias = "ls")]
    List,

    /// Eine Vorlage anzeigen, mit ihrem Text.
    Show {
        #[arg(value_name = "VORLAGE")]
        template: String,
    },

    /// `.mmd`-Dateien einlesen. Ein vorhandener Kurzname wird ersetzt.
    ///
    /// Der Kurzname ist der Dateiname ohne Endung. Ein Verzeichnis liest alle
    /// `.mmd`-Dateien darin.
    Import {
        #[arg(value_name = "DATEI", required = true)]
        files: Vec<PathBuf>,
    },

    /// Eine Vorlage löschen.
    #[command(alias = "delete")]
    Rm {
        #[arg(value_name = "VORLAGE")]
        template: String,

        /// Nicht nachfragen.
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum TokenCommand {
    /// Token speichern — im Schlüsselbund des Systems.
    ///
    /// Im Terminal wird verdeckt nachgefragt. Ohne Terminal kommt der Token
    /// von stdin: `pass notion | vizu-notion token set`. Als Argument gibt es
    /// ihn absichtlich nicht — er stünde sonst in der Shell-Historie.
    Set,

    /// Zeigt, ob und woher ein Token kommt — nie den Token selbst.
    Status,

    /// Gespeicherten Token löschen. `VIZU_NOTION_TOKEN` bleibt unberührt.
    Clear,
}

/// `title=Name` → (`title`, `Name`). Nur die Form; ob die Rolle gültig ist,
/// prüft core.
fn parse_mapping(raw: &str) -> Result<(String, String), String> {
    raw.split_once('=')
        .map(|(role, property)| (role.to_string(), property.to_string()))
        .ok_or_else(|| format!("„{raw}\u{201c}: erwartet ROLLE=SPALTE, z. B. title=Name"))
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Aktuelle Einstellungen anzeigen.
    Show,

    /// Eine Einstellung setzen.
    ///
    /// Schlüssel: `theme` (system|light|dark), `accent` (#rrggbb).
    Set { key: String, value: String },

    /// Einstellungen auf die Voreinstellung zurücksetzen.
    Reset,
}

pub use clap_complete::Shell;
