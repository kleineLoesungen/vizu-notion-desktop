# Aussehen ändern

## Die eine Regel

**Farben werden nur in `crates/gui/src/theme.rs` gesetzt.**

Wer in einer Ansicht `Color32::from_rgb(…)` schreibt, erzeugt eine Farbe, die
sich nicht mehr zentral ändern lässt und in der jeweils anderen Fassung (hell
oder dunkel) falsch aussieht. Das ist dieselbe Regel wie `theme.css` in den
Web-Kits, nur in Rust.

Beim Zeichnen kommen Farben aus `ui.visuals()` — das `Ui` weiß, in welcher
Fassung es gerade zeichnet, der `Context` kennt beide gleichzeitig.

## Was sich ohne Code ändern lässt

Über die Einstellungen, in der Oberfläche oder auf der Kommandozeile:

```bash
starter config set app_name "Notizbuch"
starter config set theme dark          # system | light | dark
starter config set accent "#aa3344"
```

Die Akzentfarbe wirkt auf Auswahl, Verweise und die bestätigende Schaltfläche.
Die Schriftfarbe darauf wird **berechnet**, nicht festgelegt:

```rust
// theme.rs
let luminanz = 0.299 * r + 0.587 * g + 0.114 * b;
if luminanz > 140.0 { Color32::BLACK } else { Color32::WHITE }
```

Ohne diese Rechnung wird weiße Schrift auf `accent = "#ffdd00"` unlesbar. Die
Gewichte stammen aus der wahrgenommenen Helligkeit — Grün trägt am meisten bei,
Blau am wenigsten.

## Eine eigene Marke hinzufügen

1. Feld zu `Config` in `crates/core/src/config.rs` — mit Voreinstellung und
   Prüfung in `Config::validate`.
2. `crates/cli/src/commands/config.rs`, Funktion `apply` — sonst kennt
   `config set` den Schlüssel nicht. *(Der Test `config_kennt_alle_felder`
   erinnert daran.)*
3. `crates/gui/src/views/settings.rs` — Eingabefeld.
4. `crates/gui/src/theme.rs` — anwenden.

Schritt 1 zuerst, immer. Eine Einstellung, die nur die Oberfläche kennt, ist
keine Einstellung.

## Abstände und Größen

Ebenfalls in `theme.rs`, und zwar für **beide** Fassungen:

```rust
for theme in [EguiTheme::Dark, EguiTheme::Light] {
    ctx.style_mut_of(theme, |style| {
        style.spacing.item_spacing   = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
    });
}
```

`ctx.style_mut()` gibt es in egui 0.36 nicht mehr. Wer nur die gerade sichtbare
Fassung setzt, bekommt beim Umschalten der Systemeinstellung das alte Aussehen
zurück.

`theme::apply` vergleicht am Anfang mit dem zuletzt gesetzten Stand und kehrt
sofort zurück, wenn sich nichts geändert hat. Ohne das liefe der Aufbau der
Stile viele Male pro Sekunde.

## Symbole

Quelle ist `assets/icon.svg`. Danach:

```bash
just icons     # braucht macOS (qlmanage, sips, iconutil)
```

Das erzeugt `assets/icons/icon-{16..1024}.png` und `assets/icons/starter.icns`.
**Die Ergebnisse liegen mit im Verwaltungssystem** — ein Bau unter Linux ruft
das Skript nicht auf und braucht die macOS-Werkzeuge nicht.

Die Farbe im SVG ist dieselbe wie die Voreinstellung von `accent`. Wer die eine
ändert, sollte die andere mitändern.

## Schriftzeichen

Die mitgelieferte Schrift kennt nicht jedes Symbol. Ein fehlendes Zeichen wird
als leeres Kästchen gezeichnet — ohne Fehler, ohne Warnung.

| Nicht | Sondern | |
|---|---|---|
| `✕` U+2715 | `×` U+00D7 | Löschen |
| `✓` U+2713 | `✔` oder Text | Bestätigen |

Nach dem Einbau eines Symbols einmal hinsehen. Wer eine eigene Schrift
mitliefern will, tut das über `egui::FontDefinitions` in `Gui::new` — dann wird
das Binary um die Schriftdatei größer.

## Bedienelemente brauchen Namen

Ein Knopf mit der Aufschrift `×` heißt im Barrierefreiheitsbaum auch `×`; ein
Bildschirmleser liest daraus „Multiplikationszeichen" vor. Dafür gibt es:

```rust
let button = widgets::labelled(ui.small_button("×"), "Löschen");
```

Derselbe Name ist das, wonach `crates/gui/tests/gui.rs` sucht. Ein Bedienelement,
das sich nicht testen lässt, weil es keinen Namen hat, ist auch für Menschen
nicht bedienbar, die es nicht sehen können — die beiden Anliegen fallen
zusammen.

Zwei Bedienelemente mit demselben Namen gleichzeitig auf dem Bildschirm sind für
beide mehrdeutig. Deshalb heißt die Schaltfläche in der Löschrückfrage
„Endgültig löschen" und nicht noch einmal „Löschen".

## Fenstergröße und Titel

In `crates/gui/src/main.rs`:

```rust
viewport: egui::ViewportBuilder::default()
    .with_inner_size([1000.0, 680.0])
    .with_min_inner_size([560.0, 400.0])
    .with_title("Starter"),
```

Die Mindestgröße nicht weglassen: Darunter überlappen Liste und Formular.
