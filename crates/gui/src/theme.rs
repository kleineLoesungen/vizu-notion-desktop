//! Aussehen — die Gestaltungsmarken an einer Stelle.
//!
//! Das Gegenstück zu `assets/css/theme.css` der Web-Kits: **Farben werden
//! nirgends sonst gesetzt.** Wer in einer Ansicht `Color32::from_rgb(…)`
//! schreibt, hat eine Farbe erzeugt, die sich nicht mehr zentral ändern lässt
//! und im hellen oder dunklen Betrieb falsch aussieht.
//!
//! Was hier eingestellt wird, leitet sich aus [`Config`] ab — dieselbe
//! Konfiguration, die `starter config set accent "#aa3344"` schreibt.

use egui::{Color32, Context, Theme as EguiTheme, ThemePreference, Visuals};
use starter_core::{Config, Theme};

/// Merkt sich, was zuletzt gesetzt wurde.
///
/// `ui()` läuft viele Male pro Sekunde. Die Stile jedes Mal neu zu bauen wäre
/// verschwendete Arbeit und ließe egui bei jedem Bild neu umbrechen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Applied {
    theme: Theme,
    accent: [u8; 3],
}

pub fn apply(ctx: &Context, config: &Config, applied: &mut Option<Applied>) {
    let now = Applied {
        theme: config.theme,
        accent: config.accent_rgb(),
    };
    if *applied == Some(now) {
        return;
    }
    *applied = Some(now);

    ctx.set_theme(match config.theme {
        Theme::System => ThemePreference::System,
        Theme::Light => ThemePreference::Light,
        Theme::Dark => ThemePreference::Dark,
    });

    let [r, g, b] = now.accent;
    let accent = Color32::from_rgb(r, g, b);

    // Beide Fassungen setzen, nicht nur die gerade sichtbare: Wechselt der
    // Benutzer die Systemeinstellung, greift egui auf die andere zu, ohne uns
    // zu fragen.
    for (theme, mut visuals) in [
        (EguiTheme::Dark, Visuals::dark()),
        (EguiTheme::Light, Visuals::light()),
    ] {
        visuals.selection.bg_fill = accent;
        visuals.selection.stroke.color = on_accent(accent);
        visuals.hyperlink_color = accent;
        ctx.set_visuals_of(theme, visuals);
    }

    // Abstände gelten für beide Fassungen — style_mut_of verlangt sie einzeln.
    for theme in [EguiTheme::Dark, EguiTheme::Light] {
        ctx.style_mut_of(theme, |style| {
            style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            style.spacing.button_padding = egui::vec2(10.0, 6.0);
        });
    }
}

/// Schwarz oder Weiß — was auf der Akzentfarbe lesbar ist.
///
/// Ohne diese Rechnung wird weiße Schrift auf einem hellen Akzent unlesbar,
/// sobald jemand `accent = "#ffdd00"` einträgt. Die Gewichte stammen aus der
/// wahrgenommenen Helligkeit: Grün trägt am meisten bei, Blau am wenigsten.
pub fn on_accent(accent: Color32) -> Color32 {
    let luminanz =
        0.299 * accent.r() as f32 + 0.587 * accent.g() as f32 + 0.114 * accent.b() as f32;
    if luminanz > 140.0 {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

/// Die Akzentfarbe als egui-Farbe.
pub fn accent(config: &Config) -> Color32 {
    let [r, g, b] = config.accent_rgb();
    Color32::from_rgb(r, g, b)
}

/// Farbe für Fehlermeldungen — aus den Stilen, nicht selbst gemischt.
///
/// Die Farbe kommt vom `Ui`, nicht vom `Context`: der `Context` kennt hell und
/// dunkel gleichzeitig, das `Ui` weiß, in welcher Fassung es gerade zeichnet.
pub fn error_color(ui: &egui::Ui) -> Color32 {
    ui.visuals().error_fg_color
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schrift_auf_hellem_akzent_ist_schwarz() {
        assert_eq!(
            on_accent(Color32::from_rgb(0xff, 0xdd, 0x00)),
            Color32::BLACK
        );
    }

    #[test]
    fn schrift_auf_dunklem_akzent_ist_weiss() {
        assert_eq!(
            on_accent(Color32::from_rgb(0x3b, 0x6e, 0xa5)),
            Color32::WHITE
        );
    }
}
