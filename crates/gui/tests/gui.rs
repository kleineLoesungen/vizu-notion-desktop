//! Abnahmetests der Oberfläche — ohne Fenster.
//!
//! `egui_kittest` baut den Barrierefreiheitsbaum auf, den egui ohnehin für
//! Bildschirmleser erzeugt. Darüber lassen sich Schaltflächen anklicken und
//! Texte lesen, ohne etwas zu rendern.
//!
//! **Bewusst keine Bildvergleiche.** Die bräuchten einen Grafiktreiber im
//! Testlauf, wären auf jedem Rechner leicht anders und schlügen bei jeder
//! Schriftänderung fehl. Geprüft wird, was da steht und was ein Klick bewirkt
//! — nicht, wie es aussieht.
//!
//! Der Zugriff auf [`Gui::model`] prüft am Ende die Wirkung: die Oberfläche
//! soll nicht nur anders aussehen, sondern tatsächlich etwas gespeichert
//! haben.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use starter_core::{App, Paths};
use starter_gui::Gui;

/// Eine Oberfläche auf einem Wegwerfverzeichnis.
///
/// Das `TempDir` muss am Leben bleiben, solange der Test läuft — deshalb kommt
/// es mit zurück.
fn harness() -> (tempfile::TempDir, Harness<'static, Gui>) {
    let dir = tempfile::tempdir().expect("Wegwerfverzeichnis");
    let core = App::open(Paths::under(dir.path())).expect("Anwendung öffnet");
    let harness = Harness::new_ui_state(|ui, gui: &mut Gui| gui.draw(ui), Gui::new(core));
    (dir, harness)
}

/// Titel eintippen und speichern.
fn notiz_anlegen(harness: &mut Harness<'static, Gui>, titel: &str) {
    harness.run();
    let feld = harness
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .next()
        .expect("Titelfeld ist das erste Textfeld");
    feld.focus();
    feld.type_text(titel);
    harness.run();

    harness.get_by_label("Speichern").click();
    harness.run();
}

#[test]
fn legt_eine_notiz_an() {
    let (_dir, mut harness) = harness();
    notiz_anlegen(&mut harness, "Einkauf");

    let model = harness.state().model();
    assert_eq!(model.notes.len(), 1, "Notiz ist in der Datenbank");
    assert_eq!(model.notes[0].title, "Einkauf");
    assert!(model.field_errors.is_empty());

    // Nach dem Speichern steht die Notiz auch in der Liste links.
    harness.get_by_label("Einkauf");
}

#[test]
fn ein_leerer_titel_erscheint_am_feld_nicht_als_meldung_oben() {
    // Ein Eingabefehler gehört an das Feld, um das es geht — sonst sucht der
    // Benutzer, was gemeint ist.
    let (_dir, mut harness) = harness();
    harness.run();
    harness.get_by_label("Speichern").click();
    harness.run();

    let model = harness.state().model();
    assert_eq!(model.field_error("title"), Some("darf nicht leer sein"));
    assert!(model.banner.is_none(), "keine Meldung oben");
    assert!(model.notes.is_empty(), "nichts gespeichert");

    harness.get_by_label("darf nicht leer sein");
}

#[test]
fn loeschen_fragt_nach_und_ein_abbruch_loescht_nicht() {
    let (_dir, mut harness) = harness();
    notiz_anlegen(&mut harness, "Einkauf");

    harness.get_by_label("Löschen").click();
    harness.run();
    assert!(
        harness.state().model().confirm_delete.is_some(),
        "Rückfrage offen"
    );

    harness.get_by_label("Abbrechen").click();
    harness.run();

    let model = harness.state().model();
    assert!(model.confirm_delete.is_none());
    assert_eq!(model.notes.len(), 1, "nichts gelöscht");
}

#[test]
fn loeschen_nach_bestaetigung_entfernt_die_notiz() {
    let (_dir, mut harness) = harness();
    notiz_anlegen(&mut harness, "Einkauf");

    harness.get_by_label("Löschen").click();
    harness.run();
    harness.get_by_label("Endgültig löschen").click();
    harness.run();

    assert!(harness.state().model().notes.is_empty());
}

#[test]
fn einstellungen_werden_gespeichert_und_gelten_sofort() {
    let (_dir, mut harness) = harness();
    harness.run();

    harness.get_by_label("Einstellungen").click();
    harness.run();

    harness.get_by_label("Speichern").click();
    harness.run();

    assert!(
        harness.state().model().banner.is_some(),
        "Rückmeldung erscheint"
    );
}

#[test]
fn eine_kaputte_akzentfarbe_wird_nicht_uebernommen() {
    let (_dir, mut harness) = harness();
    harness.run();
    harness.get_by_label("Einstellungen").click();
    harness.run();

    // Direkt am Entwurf drehen — das Tippen in ein bestimmtes von mehreren
    // Textfeldern wäre ein Test der Feldreihenfolge, nicht der Regel.
    harness.state_mut().model_mut().config_draft.accent = "blau".into();
    harness.get_by_label("Speichern").click();
    harness.run();

    let model = harness.state().model();
    assert!(model.field_error("accent").is_some());
    assert_eq!(model.config.accent, "#3b6ea5", "alter Wert gilt weiter");
}
