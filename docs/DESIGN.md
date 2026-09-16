# Aussehen ändern

## Die eine Regel

**Farben werden nur in `ui/src/theme.css` gesetzt.**

Wer in einer Komponente `color: "#3b6ea5"` oder in `app.css` `rgb(…)`
schreibt, erzeugt eine Farbe, die sich nicht zentral ändern lässt und im
dunklen Aussehen falsch ist. `crates/core/tests/layering.rs`
(`farben_stehen_nur_in_theme_css`) schlägt dann an.

Überall sonst: `var(--text)`, `var(--surface)`, `var(--accent)` …

## Was sich ohne Code ändern lässt

Über die Einstellungen, in der Oberfläche oder auf der Kommandozeile:

```bash
vizu-notion config set app_name "Notizbuch"
vizu-notion config set theme dark          # system | light | dark
vizu-notion config set accent "#aa3344"
```

Die Desktop-Anwendung liest die Einstellungen beim Start; nach einer Änderung
über die CLI einmal neu starten.

## Wie das Umschalten funktioniert

`ui/src/lib/theme.ts` setzt drei Dinge am `<html>`:

```html
<html data-theme="dark" data-accent-light="false" style="--accent: #3b6ea5">
```

`theme.css` antwortet darauf:

```css
:root                       { --bg: …hell… }
:root[data-theme="dark"]    { --bg: …dunkel… }
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) { --bg: …dunkel… }    /* "system" */
}
:root[data-accent-light="true"] { --on-accent: …dunkel… }
```

Die Schriftfarbe auf dem Akzent wird **berechnet**, nicht festgelegt — sonst
wird weiße Schrift auf `accent = "#ffdd00"` unlesbar:

```ts
// lib/theme.ts
0.299 * r + 0.587 * g + 0.114 * b > 140   // hell → dunkle Schrift
```

Wer eine Farbe für „dunkel" ändert, ändert sie an **beiden** dunklen Stellen
in `theme.css` (ausdrücklich dunkel und Systemeinstellung). CSS kennt keine
Möglichkeit, einen Block für zwei Selektoren mit einer Medienabfrage zu teilen.

## Eine eigene Marke hinzufügen

1. Variable in `theme.css` — im `:root`-Block **und** in beiden dunklen
   Blöcken.
2. In `app.css` oder einer Komponente `var(--name)` benutzen.

Soll sie einstellbar sein:

1. Feld zu `Config` in `crates/core/src/config.rs` — mit Voreinstellung und
   Prüfung in `Config::validate`.
2. `crates/cli/src/commands/config.rs`, Funktion `apply`.
3. `just bindings`.
4. `ui/src/components/SettingsDialog.tsx` — Eingabefeld.
5. `ui/src/lib/theme.ts`, `applyTheme` — als Variable oder Attribut setzen.

Schritt 1 zuerst, immer. Eine Einstellung, die nur die Oberfläche kennt, ist
keine Einstellung.

## Diagramme

Mermaid bekommt `theme: "dark"` oder `"default"`, je nachdem, was gerade
sichtbar ist (`useIsDark`). Die Schrift übernimmt es vom umgebenden Element.

Wer die Diagramme in der Akzentfarbe will: Mermaid kennt `theme: "base"` mit
`themeVariables`. Die Werte dürfen nicht `var(--…)` sein — Mermaid rechnet
damit. Aus dem Dokument lesen:

```ts
const accent = getComputedStyle(document.documentElement).getPropertyValue("--accent").trim();
mermaid.initialize({ theme: "base", themeVariables: { primaryColor: accent } });
```

Das gehört nach `lib/mermaid.ts`, nicht in eine Komponente.

## Schrift

`--font-ui` in `theme.css` nimmt die Systemschrift: San Francisco auf macOS,
die Desktop-Schrift auf Linux. Eine eigene Schrift gehört als Datei nach
`ui/src/assets/` und per `@font-face` in `app.css` — **nicht** von Google Fonts:
Die CSP sperrt fremde Server, und die Anwendung muss ohne Netz starten.

## Symbole

Quelle ist `assets/icon.svg` — eine Vektordatei, damit jede Größe scharf
bleibt. Das Motiv: vier Knoten, deren Kanten ein N zeichnen, auf Papierweiß
mit schwarzen Haarlinien. Das ist eine **Anlehnung** an die Bildsprache von
Notion, kein Nachbau ihres Zeichens — das gehört ihnen.

```bash
just icons
```

Das erzeugt `crates/desktop/icons/` (`icon.icns` für macOS, PNGs für Linux).
**Die Ergebnisse liegen mit im Verwaltungssystem.** Der farbige Knoten trägt
die Voreinstellung von `accent` (`#3b6ea5`) — wer sie ändert, ändert auch die
eine Zeile in `assets/icon.svg`.

## Bedienelemente brauchen Namen

Ein Knopf mit der Aufschrift `×` heißt für einen Bildschirmleser
„Multiplikationszeichen". Deshalb Text oder `aria-label`:

```tsx
<button type="button" aria-label="Schließen">×</button>
```

Derselbe Name ist das, wonach `ui/src/App.test.tsx` sucht
(`getByRole("button", { name: "Schließen" })`). Ein Bedienelement, das sich
nicht testen lässt, weil es keinen Namen hat, ist auch für Menschen nicht
bedienbar, die es nicht sehen können.

Zwei Bedienelemente mit demselben Namen gleichzeitig sind für beide
mehrdeutig. Deshalb heißt die Schaltfläche in der Löschrückfrage
„Endgültig löschen".

## Fenstergröße und Titel

In `crates/desktop/tauri.conf.json`:

```json
"windows": [{ "title": "Vizu Notion", "width": 1100, "height": 720,
              "minWidth": 640, "minHeight": 420 }]
```

Die Mindestgröße nicht weglassen: Darunter überlappen Liste und Editor. Der
Titel wird zur Laufzeit aus `app_name` gesetzt (`api.setWindowTitle`,
Berechtigung `core:window:allow-set-title`).
