//! Einstellungen — dieselben Werte, die `starter config set` schreibt.

use egui::Ui;
use starter_core::Theme;

use crate::model::{Action, Model};
use crate::theme;

pub fn show(ui: &mut Ui, model: &mut Model) -> Option<Action> {
    let mut action = None;
    let error_color = theme::error_color(ui);
    let name_error = model.field_error("app_name").map(str::to_string);
    let accent_error = model.field_error("accent").map(str::to_string);

    ui.heading("Einstellungen");
    ui.add_space(4.0);
    ui.label("Gespeichert wird in dieselbe Datei, die auch die Kommandozeile liest.");
    ui.add_space(12.0);

    egui::Grid::new("einstellungen")
        .num_columns(2)
        .spacing([12.0, 10.0])
        .show(ui, |ui| {
            ui.label("Name");
            ui.add(
                egui::TextEdit::singleline(&mut model.config_draft.app_name).desired_width(240.0),
            );
            ui.end_row();

            ui.label("");
            if let Some(message) = &name_error {
                ui.colored_label(error_color, message);
            }
            ui.end_row();

            ui.label("Darstellung");
            egui::ComboBox::from_id_salt("theme")
                .selected_text(model.config_draft.theme.label())
                .show_ui(ui, |ui| {
                    for theme in Theme::ALL {
                        ui.selectable_value(&mut model.config_draft.theme, theme, theme.label());
                    }
                });
            ui.end_row();

            ui.label("Akzentfarbe");
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut model.config_draft.accent)
                        .desired_width(100.0)
                        .hint_text("#rrggbb"),
                );
                // Die Vorschau zeigt sofort, was der getippte Wert bedeutet —
                // übernommen wird er trotzdem erst beim Speichern.
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(28.0, 20.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 4.0, theme::accent(&model.config_draft));
            });
            ui.end_row();

            ui.label("");
            if let Some(message) = &accent_error {
                ui.colored_label(error_color, message);
            }
            ui.end_row();
        });

    ui.add_space(16.0);
    ui.horizontal(|ui| {
        if ui.button("Speichern").clicked() {
            action = Some(Action::SaveConfig);
        }
        if ui.button("Zurücksetzen").clicked() {
            action = Some(Action::ResetConfig);
        }
    });

    action
}
