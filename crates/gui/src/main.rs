//! Die grafische Schale.
//!
//! Aufgabe dieser Datei: Protokoll einrichten, Pfade auflösen, Fenster öffnen.
//! Alles Weitere steht in [`app::Gui`].
//!
//! `windows_subsystem` fehlt absichtlich — das Kit zielt auf macOS und Linux.
//! Wer Windows nachrüstet, braucht hier
//! `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`, sonst
//! öffnet sich neben dem Fenster eine Konsole.

use starter_core::Paths;
use starter_gui::Gui;

fn main() -> eframe::Result<()> {
    let paths = Paths::resolve().unwrap_or_else(|err| {
        // Ohne Heimatverzeichnis gibt es nichts zu retten. Hier darf die
        // Schale — und nur sie — abbrechen.
        eprintln!("Fehler: {err}");
        std::process::exit(1);
    });
    init_tracing(&paths);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 680.0])
            .with_min_inner_size([560.0, 400.0])
            .with_title("Starter"),
        ..Default::default()
    };

    // Schlägt das Öffnen fehl — Platte voll, Datenbank einer neueren Fassung —,
    // wird trotzdem ein Fenster gezeigt. Ein Programm, das beim Doppelklick
    // wortlos nichts tut, ist das schlechteste aller Verhalten.
    match starter_core::App::open(paths) {
        Ok(core) => eframe::run_native(
            "Starter",
            options,
            Box::new(|_cc| Ok(Box::new(Gui::new(core)))),
        ),
        Err(err) => {
            tracing::error!(%err, "Start fehlgeschlagen");
            let message = err.to_string();
            eframe::run_native(
                "Starter",
                options,
                Box::new(move |_cc| Ok(Box::new(Fatal { message }))),
            )
        }
    }
}

/// Fenster für den Fall, dass die Anwendung gar nicht erst starten konnte.
struct Fatal {
    message: String,
}

impl eframe::App for Fatal {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.heading("Starter kann nicht starten");
                ui.add_space(8.0);
                ui.colored_label(ui.visuals().error_fg_color, &self.message);
                ui.add_space(16.0);
                ui.label("Mehr Auskunft gibt:  starter paths");
            });
        });
    }
}

/// Protokoll in eine Datei — eine Oberfläche hat kein Terminal, auf dem
/// jemand stderr läse.
fn init_tracing(paths: &Paths) {
    let level = std::env::var("STARTER_LOG").unwrap_or_else(|_| "starter=info".into());
    let file = paths
        .log_file()
        .parent()
        .and_then(|dir| std::fs::create_dir_all(dir).ok())
        .and_then(|()| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(paths.log_file())
                .ok()
        });

    let builder = tracing_subscriber::fmt()
        .with_env_filter(level)
        .with_ansi(false);
    match file {
        Some(file) => builder.with_writer(file).init(),
        None => builder.with_writer(std::io::stderr).init(),
    }
}
