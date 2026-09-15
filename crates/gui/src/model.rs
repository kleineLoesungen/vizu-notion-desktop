//! Der Zustand, den die Oberfläche zeichnet — und sonst nichts.
//!
//! # Warum es diesen Typ gibt
//!
//! `Model` kennt **kein** [`starter_core::App`], keine Datenbank und keine
//! Konfigurationsdatei. Die Ansichten in `views/` bekommen nur ein `&mut Model`
//! zu sehen. Damit ist es technisch unmöglich, aus einer Zeichenfunktion heraus
//! etwas zu speichern — und genau das ist der häufigste Fehler in
//! Sofortmodus-Oberflächen: Speichern beim Zeichnen, also viele Male pro
//! Sekunde.
//!
//! Der Weg nach draußen führt ausschließlich über [`Action`]: die Ansicht
//! *wünscht* etwas, [`crate::app::Gui::apply`] *tut* es.

use starter_core::note::{Note, Order};
use starter_core::{Config, FieldError};
use uuid::Uuid;

/// Welche Seite gerade sichtbar ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Notes,
    Settings,
}

/// Was die Ansicht vom Rest der Anwendung möchte.
///
/// Eine Ansicht gibt höchstens eine Aktion je Bild zurück. Mehr ist nicht
/// nötig — ein Mensch klickt nicht zweimal in derselben sechzehntel Sekunde —
/// und eine Liste würde die Reihenfolge zur Frage machen.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Liste neu aus der Datenbank holen.
    Reload,
    /// Leeres Formular für eine neue Notiz.
    NewDraft,
    /// Eine Notiz in das Formular laden.
    Select(Uuid),
    /// Formular speichern — legt an oder ändert, je nach `draft.id`.
    SaveDraft,
    /// Rückfrage vor dem Löschen öffnen.
    AskDelete(Uuid),
    /// Rückfrage beantwortet: löschen.
    ConfirmDelete(Uuid),
    /// Rückfrage beantwortet: doch nicht.
    CancelDelete,
    SetOrder(Order),
    Show(View),
    /// Geänderte Einstellungen prüfen und speichern.
    SaveConfig,
    ResetConfig,
}

/// Das Formular. `id = None` heißt: neue Notiz.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Draft {
    pub id: Option<Uuid>,
    pub title: String,
    pub body: String,
}

impl Draft {
    pub fn from(note: &Note) -> Self {
        Self {
            id: Some(note.id),
            title: note.title.clone(),
            body: note.body.clone(),
        }
    }

    pub fn is_new(&self) -> bool {
        self.id.is_none()
    }
}

/// Eine Meldung über dem Inhalt.
#[derive(Debug, Clone, PartialEq)]
pub struct Banner {
    pub text: String,
    pub kind: BannerKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerKind {
    Info,
    Error,
}

#[derive(Debug, Default)]
pub struct Model {
    pub view: View,
    pub notes: Vec<Note>,
    pub order: Order,
    pub draft: Draft,
    /// Meldungen je Eingabefeld, direkt aus [`starter_core::ValidationError`].
    pub field_errors: Vec<FieldError>,
    pub banner: Option<Banner>,
    /// Gesetzt, solange die Löschrückfrage offen ist.
    pub confirm_delete: Option<Uuid>,
    /// Die bearbeitete Fassung der Einstellungen. Wird erst bei
    /// [`Action::SaveConfig`] übernommen — eine halb getippte Farbe soll die
    /// Oberfläche nicht sofort umfärben.
    pub config_draft: Config,
    /// Die geltenden Einstellungen.
    pub config: Config,
    pub theme_applied: Option<crate::theme::Applied>,
}

impl Model {
    /// Die Meldung zu einem Feld, falls es eine gibt.
    pub fn field_error(&self, field: &str) -> Option<&str> {
        self.field_errors
            .iter()
            .find(|e| e.field == field)
            .map(|e| e.message.as_str())
    }

    pub fn info(&mut self, text: impl Into<String>) {
        self.banner = Some(Banner {
            text: text.into(),
            kind: BannerKind::Info,
        });
    }

    pub fn clear_messages(&mut self) {
        self.banner = None;
        self.field_errors.clear();
    }
}
