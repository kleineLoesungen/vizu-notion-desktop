//! Die grafische Schale als Bibliothek.
//!
//! Das Binary in `src/main.rs` ist nur die Hülle: Protokoll einrichten,
//! Fenster öffnen. Alles Übrige liegt hier, damit
//! `crates/gui/tests/` es ansprechen kann — ein Binärziel lässt sich von
//! einem Abnahmetest aus nicht importieren.

pub mod app;
pub mod model;
pub mod theme;
pub mod views;
pub mod widgets;

pub use app::Gui;
pub use model::{Action, Model, View};
