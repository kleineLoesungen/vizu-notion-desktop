//! Die Befehle, die das Webview aufrufen kann.
//!
//! **Jeder Befehl ist dünn:** Argumente annehmen, eine Funktion in
//! `vizu_notion_core` rufen, das Ergebnis zurückgeben. Prüfung und SQL stehen in
//! core — sonst gälten sie für die Kommandozeile nicht.
//! `crates/core/tests/layering.rs` sucht hier nach `execute(` und `Validator`.
//!
//! Drei Tauri-Eigenheiten, die hier zusammenkommen (ausführlich in CLAUDE.md):
//!
//! 1. **`async fn`, nicht `fn`.** Ein synchroner Befehl läuft auf dem
//!    Hauptfaden und friert das Fenster ein, solange er arbeitet.
//! 2. **Argumentnamen werden in JavaScript zu camelCase.** `note_id` hieße im
//!    Webview `noteId`. Hier sind deshalb alle Argumente einwortig.
//! 3. **Neuer Befehl = drei Stellen:** hier, `generate_handler!` in `lib.rs`,
//!    `ui/src/api.ts`. `tests/ipc_contract.rs` prüft, dass keine fehlt.

use std::collections::HashSet;

use serde::Serialize;
use tauri::State;
use ts_rs::TS;
use uuid::Uuid;
use vizu_notion_core::fetch::{self, FetchStatus, SourceOverview};
use vizu_notion_core::flow::{self, FlowGraph};
use vizu_notion_core::metro::{self, MetroMap};
use vizu_notion_core::notion::Property;
use vizu_notion_core::secret::{self, TokenStatus};
use vizu_notion_core::source::{self, Source, SourceInput};
use vizu_notion_core::template::{self, Diagram, Example, Hint, Template, TemplateInput};
use vizu_notion_core::{Config, Paths, notion};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// --- Quellen ---------------------------------------------------------------

#[tauri::command]
pub async fn source_list(state: State<'_, AppState>) -> ApiResult<Vec<SourceOverview>> {
    state.with(|app| fetch::overview(app.conn()))
}

#[tauri::command]
pub async fn source_get(state: State<'_, AppState>, id: Uuid) -> ApiResult<Source> {
    state.with(|app| source::get(app.conn(), id))
}

/// Ruft eine Quelle von Notion ab und speichert das Ergebnis.
///
/// Der Netzabruf dauert Sekunden. Er läuft deshalb in `spawn_blocking` und
/// **ohne** die Sperre auf die Anwendung — sonst stünde das Fenster still.
/// Gesperrt wird nur zweimal kurz: Quelle und Token holen, Ergebnis speichern.
#[tauri::command]
pub async fn source_fetch(state: State<'_, AppState>, id: Uuid) -> ApiResult<FetchStatus> {
    let (source, token, known) = state.with(|app| {
        let source = source::get(app.conn(), id)?;
        let token = secret::resolve(app.secrets())?;
        // Titel, die schon in der Datenbank stehen, spart der Abruf sich.
        let known = fetch::known_titles(app.conn())?;
        Ok((source, token, known))
    })?;

    let download = tauri::async_runtime::spawn_blocking(move || {
        let client = notion::Client::new(notion::HttpTransport::new(&token));
        fetch::download(&client, &source, &known)
    })
    .await
    .map_err(|e| ApiError::internal(format!("Abruf abgebrochen: {e}")))??;

    state.with(|app| fetch::store(app.conn(), &download))
}

#[tauri::command]
pub async fn source_create(state: State<'_, AppState>, input: SourceInput) -> ApiResult<Source> {
    state.with(|app| source::create(app.conn(), input))
}

#[tauri::command]
pub async fn source_update(
    state: State<'_, AppState>,
    id: Uuid,
    input: SourceInput,
) -> ApiResult<Source> {
    state.with(|app| source::update(app.conn(), id, input))
}

#[tauri::command]
pub async fn source_delete(state: State<'_, AppState>, id: Uuid) -> ApiResult<()> {
    state.with(|app| source::delete(app.conn(), id))
}

/// Die Spalten, die der letzte Abruf in Notion gesehen hat — für die Auswahl
/// beim Zuordnen. Leer, solange nie abgerufen wurde.
#[tauri::command]
pub async fn source_properties(state: State<'_, AppState>, id: Uuid) -> ApiResult<Vec<Property>> {
    state.with(|app| fetch::schema_properties(app.conn(), id))
}

// --- Vorlagen und Diagramme --------------------------------------------------

#[tauri::command]
pub async fn template_list(state: State<'_, AppState>) -> ApiResult<Vec<Template>> {
    state.with(|app| template::list(app.conn()))
}

#[tauri::command]
pub async fn template_get(state: State<'_, AppState>, id: Uuid) -> ApiResult<Template> {
    state.with(|app| template::get(app.conn(), id))
}

/// Legt eine Vorlage an oder ändert sie — je nachdem, ob `id` gesetzt ist.
#[tauri::command]
pub async fn template_save(
    state: State<'_, AppState>,
    id: Option<Uuid>,
    input: TemplateInput,
) -> ApiResult<Template> {
    state.with(|app| match id {
        Some(id) => template::update(app.conn(), id, input),
        None => template::create(app.conn(), input),
    })
}

#[tauri::command]
pub async fn template_delete(state: State<'_, AppState>, id: Uuid) -> ApiResult<()> {
    state.with(|app| template::delete(app.conn(), id))
}

/// Beispiele und Spickzettel zur Vorlagensprache — für den Editor.
#[tauri::command]
pub async fn template_help() -> ApiResult<TemplateHelp> {
    Ok(TemplateHelp {
        examples: template::examples::examples(),
        hints: template::examples::cheat_sheet(),
    })
}

/// Was der Editor an Hilfe anzeigt.
#[derive(Debug, Clone, Serialize, TS)]
pub struct TemplateHelp {
    pub examples: Vec<Example>,
    pub hints: Vec<Hint>,
}

/// Zeichnet einen noch nicht gespeicherten Vorlagentext — die Vorschau des
/// Editors. Dieselbe Rechnung wie beim fertigen Diagramm, damit die Vorschau
/// nicht davon abweichen kann.
#[tauri::command]
pub async fn diagram_preview(
    state: State<'_, AppState>,
    body: String,
    hidden: Vec<String>,
) -> ApiResult<Diagram> {
    let hidden: HashSet<String> = hidden.into_iter().collect();
    state.with(|app| template::render_body(app.conn(), &body, &hidden))
}

/// Zeichnet eine Vorlage aus dem Zwischenspeicher — ohne Netz.
///
/// `hidden` sind Seiten-IDs, die das Filterfeld ausgeblendet hat. Sie fehlen
/// im Mermaid-Text, stehen aber weiter in `nodes`, damit die Oberfläche sie
/// wieder einblenden kann.
#[tauri::command]
pub async fn diagram_render(
    state: State<'_, AppState>,
    id: Uuid,
    hidden: Vec<String>,
) -> ApiResult<Diagram> {
    let hidden: HashSet<String> = hidden.into_iter().collect();
    state.with(|app| {
        let found = template::get(app.conn(), id)?;
        template::render(app.conn(), &found, &hidden)
    })
}

/// Zeichnet den Fluss einer Quelle — ohne Vorlage, ohne Netz.
#[tauri::command]
pub async fn flow_render(
    state: State<'_, AppState>,
    id: Uuid,
    hidden: Vec<String>,
    subtitle: Option<String>,
) -> ApiResult<FlowGraph> {
    let hidden: HashSet<String> = hidden.into_iter().collect();
    state.with(|app| flow::build(app.conn(), id, &hidden, subtitle.as_deref()))
}

/// Zeichnet die Metro-Karte einer Quelle — ohne Vorlage, ohne Netz.
#[tauri::command]
pub async fn metro_render(
    state: State<'_, AppState>,
    id: Uuid,
    hidden: Vec<String>,
) -> ApiResult<MetroMap> {
    let hidden: HashSet<String> = hidden.into_iter().collect();
    state.with(|app| metro::build(app.conn(), id, &hidden))
}

// --- Token -----------------------------------------------------------------

/// Speichert den Token. Er kommt aus dem Eingabefeld der Oberfläche und geht
/// von hier in den Schlüsselbund — protokolliert wird er nirgends.
#[tauri::command]
pub async fn token_set(state: State<'_, AppState>, token: String) -> ApiResult<TokenStatus> {
    state.with(|app| secret::set(app.secrets(), &token))
}

#[tauri::command]
pub async fn token_clear(state: State<'_, AppState>) -> ApiResult<TokenStatus> {
    state.with(|app| {
        secret::clear(app.secrets())?;
        Ok(secret::status(app.secrets()))
    })
}

/// Ob ein Token da ist und woher — nie der Token selbst.
#[tauri::command]
pub async fn token_status(state: State<'_, AppState>) -> ApiResult<TokenStatus> {
    state.with(|app| Ok(secret::status(app.secrets())))
}

// --- Einstellungen ---------------------------------------------------------

#[tauri::command]
pub async fn config_get(state: State<'_, AppState>) -> ApiResult<Config> {
    state.with(|app| Ok(app.config().clone()))
}

/// Prüft und speichert. Bei ungültigen Werten bleibt der alte Stand stehen,
/// und der Fehler trägt `fields` — die Oberfläche zeigt ihn am Feld.
#[tauri::command]
pub async fn config_set(state: State<'_, AppState>, config: Config) -> ApiResult<Config> {
    state.with(|app| {
        app.set_config(config)?;
        Ok(app.config().clone())
    })
}

// --- Dateien ---------------------------------------------------------------

/// Schreibt das SVG eines Diagramms in eine Datei.
///
/// Den Pfad wählt die Oberfläche über den Speichern-Dialog des Systems; ein
/// Download aus dem Webview heraus ginge nicht.
#[tauri::command]
pub async fn export_svg(path: String, svg: String) -> ApiResult<String> {
    let path = std::path::PathBuf::from(path);
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::write(&path, svg)
            .map(|()| path.display().to_string())
            .map_err(|e| ApiError::internal(format!("{}: {e}", path.display())))
    })
    .await
    .map_err(|e| ApiError::internal(format!("Speichern abgebrochen: {e}")))?
}

// --- Über die Anwendung ----------------------------------------------------

/// Wo die Dateien liegen — dieselben Pfade wie `vizu-notion paths --json`.
#[derive(Debug, Clone, Serialize, TS)]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub config_file: String,
    pub db_file: String,
    pub log_file: String,
}

impl AppInfo {
    fn new(paths: &Paths) -> Self {
        let text = |p: &std::path::Path| p.display().to_string();
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            data_dir: text(paths.data_dir()),
            config_file: text(&paths.config_file()),
            db_file: text(&paths.db_file()),
            log_file: text(&paths.log_file()),
        }
    }
}

#[tauri::command]
pub async fn app_info(state: State<'_, AppState>) -> ApiResult<AppInfo> {
    state.with(|app| Ok(AppInfo::new(app.paths())))
}
