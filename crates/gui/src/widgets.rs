//! Wiederkehrende Bedienelemente.
//!
//! Das Gegenstück zu `templates/components/macros.html` der Web-Kits: was in
//! mehr als einer Ansicht vorkommt, steht hier — einmal.

use egui::Response;

/// Gibt einem Bedienelement einen sprechenden Namen.
///
/// Ein Knopf mit der Aufschrift „✕" heißt im Barrierefreiheitsbaum auch „✕";
/// ein Bildschirmleser liest daraus „Multiplikationszeichen" vor. Mit dieser
/// Funktion bekommt er einen Namen, der etwas bedeutet.
///
/// Derselbe Name ist das, wonach `crates/gui/tests/gui.rs` sucht. Ein
/// Bedienelement, das sich nicht testen lässt, weil es keinen Namen hat, ist
/// auch für Menschen nicht bedienbar, die es nicht sehen können — die beiden
/// Anliegen fallen zusammen.
pub fn labelled(response: Response, label: &str) -> Response {
    response
        .ctx
        .accesskit_node_builder(response.id, |node| node.set_label(label));
    response
}
