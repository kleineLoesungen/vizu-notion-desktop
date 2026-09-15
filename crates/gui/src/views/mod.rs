//! Die Ansichten.
//!
//! **Regel für jede Datei in diesem Verzeichnis:** zeichnen, mehr nicht. Kein
//! Zugriff auf Datenbank oder Dateisystem — die Signaturen geben das gar nicht
//! her, sie sehen nur [`Model`]. Was geschehen soll, wird als [`Action`]
//! zurückgegeben.
//!
//! Der Grund ist der Sofortmodus: `ui()` läuft viele Male pro Sekunde. Ein
//! `note::create(...)` an dieser Stelle liefe genauso oft.

pub mod notes;
pub mod settings;

use egui::Ui;

use crate::model::{Action, BannerKind, Model, View};
use crate::theme;

pub fn show(ui: &mut Ui, model: &mut Model) -> Option<Action> {
    let mut action = None;

    egui::Panel::top("nav").show(ui, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.heading(&model.config.app_name);
            ui.separator();
            for (view, label) in [(View::Notes, "Notizen"), (View::Settings, "Einstellungen")] {
                if ui.selectable_label(model.view == view, label).clicked() {
                    action = Some(Action::Show(view));
                }
            }
        });
        ui.add_space(4.0);
    });

    // Die Aktion wird aus dem Abschluss zurückgegeben statt hineingeschrieben:
    // ein `action = action.or(...)` im Abschluss würde `action` verschieben.
    let from_view = egui::CentralPanel::default().show(ui, |ui| {
        if let Some(banner) = model.banner.clone() {
            let color = match banner.kind {
                BannerKind::Info => theme::accent(&model.config),
                BannerKind::Error => theme::error_color(ui),
            };
            ui.horizontal(|ui| {
                ui.colored_label(color, &banner.text);
            });
            ui.separator();
        }

        match model.view {
            View::Notes => notes::show(ui, model),
            View::Settings => settings::show(ui, model),
        }
    });
    action = action.or(from_view.inner);

    // Die Rückfrage liegt über allem und wird deshalb zuletzt gezeichnet.
    if let Some(id) = model.confirm_delete {
        let title = model
            .notes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.title.clone())
            .unwrap_or_default();

        let response = egui::Modal::new(egui::Id::new("loeschen")).show(ui.ctx(), |ui| {
            ui.set_width(320.0);
            ui.heading("Löschen?");
            ui.label(format!("„{title}“ wird endgültig entfernt."));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Abbrechen").clicked() {
                    action = Some(Action::CancelDelete);
                }
                // Nicht noch einmal „Löschen": derselbe Name zweimal auf dem
                // Bildschirm ist für Bildschirmleser und Tests mehrdeutig.
                if ui.button("Endgültig löschen").clicked() {
                    action = Some(Action::ConfirmDelete(id));
                }
            });
        });

        // Klick daneben oder Escape zählt als Abbruch — nie als Zustimmung.
        if response.should_close() {
            action = Some(Action::CancelDelete);
        }
    }

    action
}
