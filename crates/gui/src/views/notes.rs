//! Liste links, Formular rechts.

use egui::Ui;
use starter_core::note::{Order, TITLE_MAX, short_id};

use crate::model::{Action, Model};
use crate::theme;
use crate::widgets;

pub fn show(ui: &mut Ui, model: &mut Model) -> Option<Action> {
    let mut action = None;

    // Die Felder einzeln ausleihen. Ohne diese Zerlegung sieht der Compiler
    // nur „model wird gelesen und geschrieben" und lehnt die Schleife ab.
    let Model {
        notes,
        draft,
        order,
        ..
    } = model;
    let selected = draft.id;

    egui::Panel::left("liste")
        .resizable(true)
        .default_size(260.0)
        .show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Neu").clicked() {
                    action = Some(Action::NewDraft);
                }
                ui.separator();
                for (value, label) in [(Order::Recent, "Zuletzt"), (Order::Title, "A–Z")] {
                    if ui.selectable_label(*order == value, label).clicked() {
                        action = Some(Action::SetOrder(value));
                    }
                }
            });
            ui.separator();

            if notes.is_empty() {
                ui.add_space(8.0);
                ui.label("Noch keine Notizen.");
                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for note in notes.iter() {
                    ui.horizontal(|ui| {
                        let label = ui.selectable_label(selected == Some(note.id), &note.title);
                        if label.clicked() {
                            action = Some(Action::Select(note.id));
                        }
                        // Die Kennung ist dieselbe Kurzform wie in
                        // `starter note list` — man kann sie abtippen.
                        label.on_hover_text(short_id(note.id));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // "×" (U+00D7), nicht "✕" (U+2715): das zweite
                            // fehlt in der mitgelieferten Schrift und wird
                            // als leeres Kästchen gezeichnet.
                            let button = widgets::labelled(ui.small_button("×"), "Löschen");
                            if button.on_hover_text("Löschen").clicked() {
                                action = Some(Action::AskDelete(note.id));
                            }
                        });
                    });
                }
            });
        });

    let from_editor = egui::CentralPanel::default()
        .show(ui, |ui| editor(ui, model))
        .inner;

    action.or(from_editor)
}

fn editor(ui: &mut Ui, model: &mut Model) -> Option<Action> {
    let mut action = None;
    let accent = theme::accent(&model.config);
    let error_color = theme::error_color(ui);

    let title_error = model.field_error("title").map(str::to_string);
    let body_error = model.field_error("body").map(str::to_string);

    ui.heading(if model.draft.is_new() {
        "Neue Notiz"
    } else {
        "Notiz bearbeiten"
    });
    ui.add_space(8.0);

    ui.label("Titel");
    let title = egui::TextEdit::singleline(&mut model.draft.title)
        .hint_text("Worum geht es?")
        .char_limit(TITLE_MAX)
        .desired_width(f32::INFINITY);
    ui.add(title);
    if let Some(message) = &title_error {
        ui.colored_label(error_color, message);
    }

    ui.add_space(8.0);
    ui.label("Text");
    ui.add(
        egui::TextEdit::multiline(&mut model.draft.body)
            .desired_width(f32::INFINITY)
            .desired_rows(12),
    );
    if let Some(message) = &body_error {
        ui.colored_label(error_color, message);
    }

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        let save =
            egui::Button::new(egui::RichText::new("Speichern").color(theme::on_accent(accent)))
                .fill(accent);
        if ui.add(save).clicked() {
            action = Some(Action::SaveDraft);
        }
        if !model.draft.is_new() && ui.button("Verwerfen").clicked() {
            action = Some(Action::NewDraft);
        }
    });

    action
}
