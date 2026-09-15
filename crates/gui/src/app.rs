//! Der Übersetzer zwischen Ansicht und Fachlogik.
//!
//! Hier — und nur hier — treffen [`Model`] und [`starter_core::App`]
//! aufeinander. Der Ablauf jedes Bildes:
//!
//! ```text
//! views::show(ui, &mut model)  →  Option<Action>  →  Gui::apply  →  starter_core
//! ```
//!
//! Fachliche Regeln stehen keine hier. Wer in [`Gui::dispatch`] ein `if
//! title.is_empty()` schreibt, hat eine Regel erzeugt, die der Kommandozeile
//! fehlt.

use starter_core::note::{self, NoteInput};
use starter_core::{Config, Error};

use crate::model::{Action, Banner, BannerKind, Draft, Model, View};
use crate::theme;
use crate::views;

pub struct Gui {
    core: starter_core::App,
    model: Model,
}

impl Gui {
    pub fn new(core: starter_core::App) -> Self {
        let mut gui = Self {
            model: Model {
                config: core.config().clone(),
                config_draft: core.config().clone(),
                ..Model::default()
            },
            core,
        };
        gui.apply(Action::Reload);
        gui
    }

    /// Ein Bild zeichnen und die ausgelöste Aktion gleich anwenden.
    ///
    /// Eigene Funktion statt nur [`eframe::App::ui`], weil ein Abnahmetest
    /// keinen `eframe::Frame` bauen kann. So lässt sich die ganze Oberfläche
    /// ohne Fenster durchklicken — siehe `crates/gui/tests/gui.rs`.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        theme::apply(ui.ctx(), &self.model.config, &mut self.model.theme_applied);

        if let Some(action) = views::show(ui, &mut self.model) {
            self.apply(action);
        }
    }

    /// Der gezeichnete Zustand — für Tests und zum Nachsehen.
    pub fn model(&self) -> &Model {
        &self.model
    }

    /// Den Zustand von außen setzen. **Nur für Tests.**
    ///
    /// Damit lässt sich ein Formular füllen, ohne das richtige von mehreren
    /// Textfeldern treffen zu müssen — sonst prüfte der Test die
    /// Feldreihenfolge statt der Regel. Im laufenden Programm gibt es keinen
    /// Aufrufer: Änderungen kommen dort über [`Action`].
    #[doc(hidden)]
    pub fn model_mut(&mut self) -> &mut Model {
        &mut self.model
    }

    pub fn apply(&mut self, action: Action) {
        if let Err(err) = self.dispatch(action) {
            self.report(err);
        }
    }

    fn dispatch(&mut self, action: Action) -> starter_core::Result<()> {
        match action {
            Action::Reload => {
                self.model.notes = note::list(self.core.conn(), self.model.order)?;
                Ok(())
            }

            Action::NewDraft => {
                self.model.clear_messages();
                self.model.draft = Draft::default();
                Ok(())
            }

            Action::Select(id) => {
                self.model.clear_messages();
                self.model.draft = Draft::from(&note::get(self.core.conn(), id)?);
                Ok(())
            }

            Action::SaveDraft => {
                let input = NoteInput::new(
                    self.model.draft.title.clone(),
                    self.model.draft.body.clone(),
                );
                let saved = match self.model.draft.id {
                    Some(id) => note::update(self.core.conn(), id, input)?,
                    None => note::create(self.core.conn(), input)?,
                };
                self.model.clear_messages();
                self.model.draft = Draft::from(&saved);
                self.model.info(format!("„{}“ gespeichert.", saved.title));
                self.dispatch(Action::Reload)
            }

            Action::AskDelete(id) => {
                self.model.confirm_delete = Some(id);
                Ok(())
            }

            Action::CancelDelete => {
                self.model.confirm_delete = None;
                Ok(())
            }

            Action::ConfirmDelete(id) => {
                self.model.confirm_delete = None;
                note::delete(self.core.conn(), id)?;
                // Wer den gerade bearbeiteten Datensatz löscht, soll kein
                // Formular behalten, das auf nichts mehr zeigt.
                if self.model.draft.id == Some(id) {
                    self.model.draft = Draft::default();
                }
                self.model.clear_messages();
                self.model.info("Gelöscht.");
                self.dispatch(Action::Reload)
            }

            Action::SetOrder(order) => {
                self.model.order = order;
                self.dispatch(Action::Reload)
            }

            Action::Show(view) => {
                self.model.clear_messages();
                if view == View::Settings {
                    // Beim Betreten der Seite den gespeicherten Stand zeigen,
                    // nicht die Reste eines früheren Versuchs.
                    self.model.config_draft = self.model.config.clone();
                }
                self.model.view = view;
                Ok(())
            }

            Action::SaveConfig => {
                self.core.set_config(self.model.config_draft.clone())?;
                self.model.config = self.core.config().clone();
                self.model.clear_messages();
                self.model.info("Einstellungen gespeichert.");
                Ok(())
            }

            Action::ResetConfig => {
                self.core.set_config(Config::default())?;
                self.model.config = self.core.config().clone();
                self.model.config_draft = self.model.config.clone();
                self.model.clear_messages();
                self.model.info("Einstellungen zurückgesetzt.");
                Ok(())
            }
        }
    }

    /// Einen Fehler dorthin bringen, wo er hingehört.
    ///
    /// Ein Eingabefehler gehört an das Feld, um das es geht — eine Meldung
    /// oben am Fenster ließe den Benutzer suchen. Alles andere ist kein
    /// Bedienfehler und kommt als Meldung.
    fn report(&mut self, err: Error) {
        tracing::warn!(%err, "Aktion fehlgeschlagen");
        match err.fields() {
            Some(fields) => {
                self.model.field_errors = fields.to_vec();
                self.model.banner = None;
            }
            None => {
                self.model.field_errors.clear();
                self.model.banner = Some(Banner {
                    text: err.to_string(),
                    kind: BannerKind::Error,
                });
            }
        }
    }
}

impl eframe::App for Gui {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}
