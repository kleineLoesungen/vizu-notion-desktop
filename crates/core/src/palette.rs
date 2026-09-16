//! Farben, die nicht aus der Oberfläche kommen können.
//!
//! Die Oberfläche hält ihre Farben in `ui/src/theme.css`, und der Wächter in
//! `crates/core/tests/layering.rs` besteht darauf. Diese hier sind aber
//! **Daten**: Der Helfer `palette` setzt sie in einen Mermaid-Text, und die
//! Metro-Karte färbt ihre Linien damit. Beides muss in der Kommandozeile
//! dasselbe ergeben wie im Fenster — also gehören sie nach `core`.

/// Zehn gut unterscheidbare Farben (Tableau 10).
///
/// **Diese Liste wird nicht geändert.** Die Webapp benutzt sie in derselben
/// Reihenfolge; eine andere Reihenfolge färbte bestehende Diagramme um.
pub const TABLEAU_10: [&str; 10] = [
    "#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f", "#edc948", "#b07aa1", "#ff9da7",
    "#9c755f", "#bab0ac",
];

/// Die Farbe für eine Nummer — nach zehn beginnt sie von vorn.
pub fn nth(index: usize) -> &'static str {
    TABLEAU_10[index % TABLEAU_10.len()]
}
