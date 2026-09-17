//! Die Desktop-Schale.
//!
//! Aufgabe: ein Fenster mit der Weboberfläche aus `ui/` öffnen und die
//! Befehle aus `commands.rs` bereitstellen. Fachlogik steht in
//! `vizu_notion_core`, Darstellung in `ui/`. Hier steht nur, was die beiden
//! verbindet.
//!
//! ```text
//! ui/src/api.ts  ──invoke──▶  commands.rs  ──▶  vizu_notion_core
//!      ▲                           │
//!      └──── JSON oder ApiError ───┘
//! ```

pub mod commands;
pub mod error;
pub mod state;

use tauri::{Runtime, Wry};
use vizu_notion_core::{App, Paths};

pub use error::{ApiError, ApiResult};
pub use state::AppState;

/// Startet die Anwendung. Kehrt erst zurück, wenn das letzte Fenster zu ist.
pub fn run() {
    let state = match Paths::resolve().and_then(App::open) {
        Ok(app) => {
            init_tracing(app.paths());
            AppState::new(app)
        }
        Err(err) => {
            init_tracing_stderr();
            tracing::error!(error = %err, "Anwendung ließ sich nicht öffnen");
            AppState::failed(err.into())
        }
    };

    configure(tauri::Builder::<Wry>::default(), state)
        .run(tauri::generate_context!())
        .expect("Tauri ließ sich nicht starten");
}

/// Plugins, Zustand und Befehle — ohne Fenster und ohne Kontext.
///
/// Getrennt von [`run`], damit `tests/` denselben Aufbau mit
/// `tauri::test::mock_builder()` benutzen kann. Ein Befehl, der hier fehlt,
/// fehlt damit auch im Test.
pub fn configure<R: Runtime>(builder: tauri::Builder<R>, state: AppState) -> tauri::Builder<R> {
    builder
        // Öffnet Verweise im Standardbrowser statt im Webview. Welche
        // Adressen erlaubt sind, steht in capabilities/default.json.
        .plugin(tauri_plugin_opener::init())
        // Speichern-Dialog für den SVG-Export. Was er darf, steht in
        // capabilities/default.json.
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        // NEUER BEFEHL? Hier eintragen UND in ui/src/api.ts.
        // tests/ipc_contract.rs vergleicht beide Listen.
        .invoke_handler(tauri::generate_handler![
            commands::source_list,
            commands::source_get,
            commands::source_fetch,
            commands::source_create,
            commands::source_update,
            commands::source_delete,
            commands::source_properties,
            commands::database_inspect,
            commands::hidden_list,
            commands::hidden_set,
            commands::template_list,
            commands::view_list,
            commands::view_save,
            commands::view_delete,
            commands::template_get,
            commands::template_save,
            commands::template_delete,
            commands::diagram_render,
            commands::diagram_preview,
            commands::flow_render,
            commands::metro_render,
            commands::template_help,
            commands::export_svg,
            commands::token_status,
            commands::token_set,
            commands::token_clear,
            commands::config_get,
            commands::config_set,
            commands::app_info,
        ])
}

/// Protokoll in eine Datei — eine Anwendung aus dem Dock hat kein Terminal.
///
/// In einem Entwicklungsbau zusätzlich auf stderr, damit `just dev` es zeigt.
fn init_tracing(paths: &Paths) {
    use tracing_subscriber::prelude::*;

    let filter = tracing_subscriber::EnvFilter::try_from_env("VIZU_NOTION_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("vizu-notion=info"));

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.log_file())
        .ok()
        .map(|f| {
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(std::sync::Mutex::new(f))
        });
    let stderr = cfg!(debug_assertions)
        .then(|| tracing_subscriber::fmt::layer().with_writer(std::io::stderr));

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(file)
        .with(stderr)
        .try_init();
}

fn init_tracing_stderr() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .try_init();
}
