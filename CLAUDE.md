# Regeln für dieses Projekt

Diese Datei ist für KI-Assistenten gedacht (Claude Code, Cursor, …) und
gleichzeitig als Kurzreferenz für Menschen brauchbar. Sie beschreibt, was in
diesem Projekt gilt und wo die Fallstricke liegen.

---

## Der Stack in einem Satz

Rust mit drei Kisten in einem Arbeitsbereich — `core` (Fachlogik, SQLite
eingebettet), `cli` (clap) und `desktop` (**Tauri 2**) — plus eine
Weboberfläche in `ui/` (**React 19, TypeScript, Vite**), die im Webview des
Betriebssystems läuft. Ziel: **macOS und Linux**. Browserpakete wie
**mermaid**, marked oder DOMPurify kommen über npm ins Bündel.

---

## Wo was liegt

```
crates/core/       Fachlogik und Prüfung. SQLite. Kennt weder clap noch tauri.
crates/cli/        Kommandozeile `vizu-notion`. Dünn.
crates/desktop/    Tauri-Schale. Dünn. tauri.conf.json, capabilities/, icons/.
ui/src/api.ts      Die EINZIGE Stelle, die mit Rust spricht.
ui/src/App.tsx     Zustand der Oberfläche. Die einzige Komponente, die api benutzt.
ui/src/components/ Zeichnen nur. Props rein, Rückrufe raus.
ui/src/lib/        Reine Hilfsfunktionen: Markdown, Mermaid, Datum, Farbschema.
ui/src/bindings.ts ERZEUGT aus Rust-Typen. Nie von Hand ändern → `just bindings`.
ui/src/theme.css   Die EINZIGE Datei mit Farben.
site/              Projektwebseite (vizu.se-wi.com). Eigenständig, nicht Teil der App;
                   .github/workflows/pages.yml veröffentlicht nur diesen Ordner.
```

Es gibt **kein `src-tauri/`**. Die Tauri-Schale liegt in `crates/desktop`,
neben den anderen Kisten; die Tauri-CLI findet `tauri.conf.json` dort von
selbst. `npm run tauri …` immer aus der Wurzel aufrufen.

---

## Festgenagelte Versionen — bitte nicht „aktualisieren"

Rust-Versionen stehen in `[workspace.dependencies]` der obersten `Cargo.toml`.
Nur dort. npm-Versionen stehen **exakt** (ohne `^`) in `package.json`.

| Paket | Version | Grund |
|---|---|---|
| **tauri** / **@tauri-apps/api** / **@tauri-apps/cli** | **2.11** | Rust-Kiste und npm-Pakete müssen in derselben Nebenversion bleiben, sonst meldet `tauri dev` „version mismatch". Plugins (`tauri-plugin-opener` ↔ `@tauri-apps/plugin-opener`) ebenso paarweise. |
| **ts-rs** | **12** | Erzeugt `ui/src/bindings.ts`. Die API hat sich zwischen 10, 11 und 12 geändert (`Config`-Argument). |
| **mermaid** | **12** | Lädt nach (`import("mermaid")`). Die alte `mermaid.init()`/`contentLoaded`-API existiert nicht mehr — `parse` und `render` benutzen. |
| **lodash-es** (overrides) | **4.18.1** | Mermaid 12 zieht über chevrotain eine verwundbare Fassung herein. Der Eintrag unter `overrides` in `package.json` hebt sie an. Erst entfernen, wenn `npm audit` ohne ihn sauber ist. |
| **React** | **19** | `ReactDOM.render` gibt es nicht mehr — `createRoot`. |
| **TypeScript** | **7** | Der native Compiler. `tsc --noEmit` prüft nur, gebaut wird von Vite. |
| **Vite** / **Vitest** | **8** / **5** | Test-Einstellungen stehen in `vite.config.ts` unter `test`. |
| **Biome** | **2.5** | Formatierer und Linter in einem. Kein ESLint, kein Prettier. |
| **rusqlite** | **0.40** mit `bundled` | SQLite ins Binary. **Nicht** auf die Systembibliothek umstellen. |
| **Rust** | **1.98.1** | In `rust-toolchain.toml`. |

Wer eine Version anheben will: erst `just dupes` und `npm outdated`, dann
`just check`.

---

## Tauri 2 — die Fallstricke

Das ist der Teil, bei dem Sprachmodelle am zuverlässigsten danebenliegen: Die
meisten Beispiele im Netz stammen aus **Tauri 1**, und dort hieß fast alles
anders. Was aus dem Gedächtnis kommt, kompiliert oft nicht — oder kompiliert
und wird zur Laufzeit verweigert.

1. **Tauri 1 ist vorbei.** Die häufigsten Verwechslungen:

   | Tauri 1 ❌ | Tauri 2 ✅ |
   |---|---|
   | `import { invoke } from "@tauri-apps/api/tauri"` | `import { invoke } from "@tauri-apps/api/core"` |
   | `"allowlist": { … }` in tauri.conf.json | `crates/desktop/capabilities/*.json` |
   | `import { open } from "@tauri-apps/api/shell"` | Plugin: `@tauri-apps/plugin-opener` + `tauri-plugin-opener` |
   | `import { appWindow } from "@tauri-apps/api/window"` | `getCurrentWindow()` |
   | `"distDir"`, `"devPath"` | `"frontendDist"`, `"devUrl"` |
   | `window.__TAURI__` | nur mit `withGlobalTauri` — hier absichtlich aus; importieren |

2. **Befehle sind `async fn`.** Ein synchroner `#[tauri::command] fn` läuft
   auf dem Hauptfaden und friert das Fenster ein, solange er arbeitet. Ein
   `async`-Befehl mit geliehenem Argument (`State<'_, …>`) **muss** `Result`
   zurückgeben, sonst meldet der Compiler einen schwer lesbaren
   Lebensdauerfehler.

3. **Argumentnamen werden zu camelCase.** Rust `fn source_fetch(source_id: Uuid)`
   hieße in JavaScript `invoke("source_fetch", { sourceId })`. Ein falscher Name
   ergibt zur Laufzeit „missing required key". Die Befehle hier haben deshalb
   einwortige Argumente (`id`, `input`, `order`, `config`). Befehlsnamen
   selbst bleiben snake_case.

4. **Neuer Befehl = drei Stellen.** `commands.rs` (Funktion),
   `lib.rs` (`generate_handler![…]`), `ui/src/api.ts` (`call<…>("name", …)`).
   `crates/desktop/tests/ipc_contract.rs` schlägt fehl, wenn eine fehlt.

5. **Plugins und Fenster-API brauchen eine Berechtigung.** Was nicht in
   `crates/desktop/capabilities/default.json` steht, wird zur Laufzeit mit
   „… not allowed" abgelehnt — obwohl alles kompiliert. Beispiel: Der
   Speichern-Dialog des SVG-Exports braucht `dialog:allow-save`. Eigene
   Befehle aus `commands.rs` sind ohne Eintrag erlaubt.

6. **Der Fehlertyp muss `Serialize` sein.** `vizu_notion_core::Error` ist es
   absichtlich nicht. Befehle geben `ApiResult<T>` zurück; `ApiError` hat
   dieselbe Form wie `error` im JSON der CLI (`code`, `message`, `fields`).

7. **Kein Netz, kein CDN.** Die CSP in `tauri.conf.json` erlaubt nur eigene
   Dateien. `<script src="https://cdn…">` lädt nicht — ohne sichtbaren Fehler.
   Pakete über npm installieren und importieren.

   **Inline-Stile und die Nonce.** Mermaid (und viele Diagramm-, Editor- und
   Chart-Pakete) setzen Stile inline. Dafür steht `'unsafe-inline'` in
   `style-src`. Tauri hängt beim Bündeln aber eine Nonce an `style-src` — und
   sobald eine Nonce da ist, **ignoriert der Browser `'unsafe-inline'`**. Im
   Entwicklungsbetrieb sieht alles richtig aus, im fertigen Bündel sind die
   Diagramme schwarze Flächen. Deshalb steht in `tauri.conf.json`
   `"dangerousDisableAssetCspModification": ["style-src"]`. Nicht entfernen —
   im gebauten Bündel nachgesehen, mit der Einstellung stimmt die Darstellung.
   Für `script-src` bleibt die Nonce an — dort ist sie ein Schutz.

8. **Verweise öffnen nicht von selbst im Browser.** Ein `<a href>` im Webview
   navigiert das Anwendungsfenster weg. `MarkdownView` fängt Klicks ab,
   `api.openExternal` öffnet im Standardbrowser.

9. **Pfade kommen aus `core`, nicht aus Tauri.** `app.path().app_data_dir()`
   liefert `~/Library/Application Support/de.kleineloesungen.vizu-notion` — die
   CLI sucht in `…/vizu-notion`. Beide Schalen benutzen `vizu_notion_core::Paths`,
   sonst sehen sie verschiedene Datenbanken.

10. **`ui/dist` muss existieren, bevor Rust kompiliert.** `generate_context!`
    bettet es ein. Nach `just clean` erst `npm run build:ui`. `just test` tut
    das von selbst.

Im Zweifel nicht raten, sondern nachlesen — die Quellen liegen lokal:

```bash
grep -rn "pub fn get_ipc_response" ~/.cargo/registry/src/*/tauri-2.11.*/src/test/
ls node_modules/@tauri-apps/api/*.d.ts
ls crates/desktop/gen/schemas/          # alle Berechtigungen, nach dem ersten Bau
```

---

## Notion — die Fallstricke

Die zweite Stelle, an der Wissen aus dem Gedächtnis danebenliegt: Die
Notion-API hat sich mit der Version **`2025-09-03`** spürbar geändert, und die
meisten Beispiele im Netz stammen von davor.

1. **Eine Datenbank ist nicht mehr die Datenquelle.** Eine `database` hat eine
   Liste von `data_sources`; Schema und Seiten hängen an der Datenquelle:

   | alt (2022-06-28) ❌ | jetzt (2025-09-03) ✅ |
   |---|---|
   | `GET /databases/{id}` liefert `properties` | `GET /data_sources/{id}` liefert sie |
   | `POST /databases/{id}/query` | `POST /data_sources/{id}/query` |

   `fetch::download` holt deshalb erst die Datenbank, dann ihre erste
   Datenquelle. Mehrere Datenquellen kommen vor — dann steht eine Warnung im
   Protokoll und es wird die erste benutzt.

2. **Relationen sind bei mehr als 25 Zielen abgeschnitten.** Die Seite trägt
   dann `has_more: true` und nur die ersten 25. Wer das übersieht, verliert
   stillschweigend Kanten — `complete_relations` holt sie nach
   (`GET /pages/{id}/properties/{property_id}`). Die Webapp tat das nicht; das
   war einer ihrer Fehler.

3. **Drei Anfragen je Sekunde.** `notion::Client` taktet selbst und wiederholt
   bei `429` nach `Retry-After`. Wer daran vorbei anfragt, bekommt Fehler, die
   erst bei großen Datenbanken auftreten.

4. **IPv6 kann minutenlang hängen.** In manchen Netzen antwortet
   `api.notion.com` über IPv6 nicht, und der Verbindungsversuch läuft ins Leere.
   `notion::transport` löst deshalb **IPv4 zuerst** auf und setzt
   `timeout_connect` und `timeout_global`. Nicht entfernen.

5. **Ein Token steht nie im Code, in einem Test oder im Protokoll.** Er liegt
   im Schlüsselbund (`secret`), ersatzweise in `VIZU_NOTION_TOKEN`. Tests
   sprechen **nicht** mit Notion: Sie geben `notion::Transport` eine Attrappe
   und lesen festgehaltene Antworten aus
   `crates/core/tests/fixtures/notion/` (siehe README dort).

   **Die Umgebung wird einmal gelesen, beim Start:** `App` merkt sich
   `VIZU_NOTION_TOKEN` in `env_token`, und `app.token()` ist die einzige
   Stelle, an der Umgebung und Speicher zusammenkommen. Wer stattdessen
   mitten im Programm `std::env::var` liest, baut einen Test, der davon
   abhängt, was in der Shell des Entwicklers steht. Tests ohne Token rufen
   `App::forget_env_token()` (`App::in_memory` hat von vornherein keinen);
   Tests, die ein Programm starten, nehmen `.env_remove(…)`.

6. **Leere Werte sind nicht `null`.** Ein leeres `status`, `formula`,
   `people` oder `rollup` kommt als Objekt mit leerem Inhalt. `rows::text_of`
   kennt die Formen; eigene Abfragen auf `value["…"]` gehen daran vorbei.

7. **Der Link zu einer Seite ist `url` aus der Antwort**, nicht
   `notion.so/{id}` zusammengebaut.

---

## Sprache im Code

* **Bezeichner englisch** — Typen, Funktionen, Felder, Tabellen, Befehle,
  Komponenten, CSS-Klassen: `SourceInput`, `source_fetch`, `SourceList`,
  `.source-item`.
* **Kommentare, Doku, Oberflächentexte und Testnamen deutsch.**
  `fn ein_titel_aus_leerzeichen_gilt_als_leer()`,
  `it("zeigt einen Eingabefehler am Feld, nicht oben")`.

---

## Die eine wichtige Schichtregel

```
crates/core/        →  FACHLOGIK UND PRÜFUNG. Kennt WEDER clap NOCH tauri NOCH stdout.
crates/cli/         →  dünn. Argumente parsen, core rufen, Ausgabe formatieren.
crates/desktop/     →  dünn. IPC annehmen, core rufen, JSON zurück. Kein SQL.
ui/src/api.ts       →  die einzige Brücke nach Rust.
ui/src/App.tsx      →  Zustand. Wünsche der Komponenten → api.
ui/src/components/  →  zeichnet nur. Sieht api nicht.
```

**`crates/core/tests/layering.rs` schlägt fehl**, sobald

* in `core` ein `println!`, `clap`, `tauri` oder `std::process::exit` steht,
* in `desktop` ein `execute(`, `query_row`, `prepare(` oder `Validator` steht,
* außerhalb von `api.ts` `@tauri-apps/api/…`, `@tauri-apps/plugin-…` oder
  `invoke(` steht,
* eine Komponente `api` importiert oder `innerHTML` setzt,
* irgendwo `dangerouslySetInnerHTML` oder ein CDN auftaucht,
* außerhalb von `theme.css` eine Hexfarbe oder `rgb(` steht.

Den Code verschieben, nicht den Test lockern.

Der Grund ist nicht Sauberkeit: **Alles, was in `core` steht, gilt für die
Kommandozeile und die Oberfläche gleichermaßen.** Eine Prüfung in TypeScript
fehlt der CLI. Genau das ist der Fehler, den dieses Kit verhindern soll.

### Prüfung gehört in die Fachlogik — nicht nach TypeScript

```rust
// ✅ crates/core/src/source.rs
pub fn create(conn: &Connection, input: SourceInput) -> Result<Source> {
    let input = input.clean()?;   // trimmt und prüft, wirft Error::Validation
    …
}
```

```tsx
// ❌ in einer Komponente
if (draft.name.trim() === "") setError("Name fehlt");
```

Die Oberfläche schickt ab und zeigt an, was zurückkommt. Eine *zusätzliche*
Vorabprüfung für schnellere Rückmeldung ist erlaubt, ersetzt aber nie die in
`core` — und die Meldung kommt trotzdem aus Rust.

---

## Die IPC-Grenze

```
Komponente ──onFetch()──▶ App.tsx ──api.sources.fetch(id)──▶ invoke("source_fetch", {id})
                                                                          │
  ◀── FetchStatus oder ApiError { code, message, fields? } ◀── commands.rs ┴─▶ vizu_notion_core
```

**Typen werden nicht doppelt geschrieben.** Rust-Typen, die über IPC gehen,
tragen `#[derive(TS)]`. Daraus entsteht `ui/src/bindings.ts`:

```bash
just bindings        # nach jeder Änderung an einem solchen Rust-Typ
```

`crates/desktop/tests/bindings.rs` schlägt fehl, wenn die Datei veraltet ist.
Ein neuer Typ wird dort in `generate()` eingetragen.

Zeitstempel sind in TypeScript `string` (RFC 3339, UTC). Umgerechnet wird nur
zur Anzeige (`lib/format.ts`).

---

## Zwei Sorten Fehler

| Sorte | Woran erkennbar | Wohin damit |
|---|---|---|
| Eingabefehler | Rust: `err.fields()` ist `Some`. TS: `apiError.isValidation` | CLI: Rückgabewert 4. Oberfläche: **am Feld** (`fieldMessage("title")`). |
| Alles andere | sonst | CLI: Rückgabewert 1. Oberfläche: Leiste oben (`banner`). |

`ErrorCode` (`not_found`, `validation_failed`, …) steht in
`crates/core/src/error.rs` und gilt für CLI-JSON und Oberfläche gleich.
Namen werden nie umbenannt, nur ergänzt.

---

## Die Oberfläche

* **Komponenten zeichnen nur.** Sie bekommen Daten als Props und melden
  Wünsche über Rückrufe (`onSave`, `onDelete`). `App.tsx` ruft `api`.
* **Kein globaler Zustandsspeicher** (Redux, Zustand, …). Ein `useState` in
  `App.tsx` reicht für ein Werkzeug dieser Größe. Wird es mehr, zuerst
  Hilfsfunktionen in `lib/` auslagern.
* **Kein CSS-Framework.** Farben und Abstände als Variablen in `theme.css`,
  Aufbau in `app.css`. Tailwind bräuchte einen eigenen Build-Schritt und
  verteilt Farben über jede Komponente — das Gegenteil der Farbregel.
* **HTML aus Benutzertext nur über `lib/markdown.ts`.** Dort bereinigt
  DOMPurify. Ein Skript im Webview hätte Zugriff auf jeden IPC-Befehl.
* **Große Pakete nachladen.** `import("mermaid")` statt `import mermaid` —
  sonst startet die Anwendung mit mehreren Megabyte JavaScript, die sie
  vielleicht nie braucht. Siehe `lib/mermaid.ts`.
* **Bedienelemente brauchen Namen** (`aria-label` oder sichtbarer Text). Die
  Tests suchen danach (`getByRole("button", { name: "Speichern" })`), und ein
  Bildschirmleser ebenso. Zwei gleichnamige Knöpfe gleichzeitig sind für
  beide mehrdeutig — deshalb „Endgültig löschen" in der Rückfrage.
* **Zielbrowser ist WebKit** — Safari auf macOS, WebKitGTK auf Linux. Kein
  Chrome. APIs, die nur Chromium kann (File System Access, Web Serial, …),
  gibt es nicht. Im Zweifel auf caniuse.com unter „Safari" nachsehen.

---

## Die Vorlagen-Engine ist an die Webapp gebunden

`crates/core/src/template/` erzeugt denselben Mermaid-Text wie die Webapp
vizu-notion-local. Das ist kein Nebenbei, sondern der Grund, warum vorhandene
`.mmd`-Vorlagen hier unverändert laufen:

* `crates/core/tests/template.rs` vergleicht mit Dateien, die die
  Original-Logik (handlebars.js 4.7) erzeugt hat. **Diese Dateien werden nicht
  angepasst, wenn ein Test fehlschlägt** — sie sind die Vorgabe.
* Die Knotenkennung ist FNV-1a über **UTF-16**-Codeeinheiten, nicht über
  UTF-8-Bytes. Anders gerechnet, ändern sich alle Kennungen mit Emoji.
* Die Maskierung ist die von Handlebars.js, als eigene `escape_fn`.
* `serde_json` läuft mit `preserve_order`; sonst stünden die `classDef`-Zeilen
  alphabetisch statt in der Reihenfolge der `styles`.

Auch die Eigenheiten der Webapp sind übernommen, etwa dass ein Quellname mit
Bindestrich nicht als Quelle erkannt wird. Was davon warum bleibt, steht in
docs/UMSETZUNG.md, Abschnitt 3.1.

## Kurzkennungen: hinten, nicht vorn

Die Kennungen sind UUIDv7. Die **beginnt mit dem Zeitstempel** — zwei Quellen
aus derselben Sekunde teilen sich die ersten zwölf Zeichen. Darum zeigt
`vizu-notion source list` die **letzten** acht Zeichen, und `ids::resolve_suffix`
sucht per Endstück. Eine Quelle spricht man aber vor allem über ihren **Namen**
an — so heißt sie auch in Vorlagen. Ist ein Endstück nicht eindeutig, gibt es `Error::Ambiguous` —
nie einen zufälligen Treffer.

---

## Pfade, Zeit, Daten

* **Die Fallunterscheidung macOS/Linux steht nur in `crates/core/src/paths.rs`.**
* `VIZU_NOTION_DATA_DIR` und `VIZU_NOTION_CONFIG_DIR` überschreiben die Verzeichnisse
  — für CLI **und** Desktop (`just dev-sandbox`, `just sandbox`). In
  Rust-Tests stattdessen `Paths::under(tempdir)`.
* **Gespeichert wird in UTC**, als RFC-3339-Text.
* **Eine veröffentlichte Migration wird nie geändert.** Neue Datei, nächste
  Nummer, in `crates/core/src/db.rs` eintragen.
* Die Tabellen sind `STRICT`.
* **WAL** — Desktop-Anwendung und CLI dürfen gleichzeitig laufen.

---

## Was hier NICHT benutzt wird

Bitte nicht „nachrüsten":

| Nicht | Stattdessen | Warum |
|---|---|---|
| `src-tauri/` | `crates/desktop/` | Eine Kiste unter vielen, dieselben Regeln |
| Electron | Tauri | 150 MB Chromium je Anwendung |
| Tauri 1-APIs, `allowlist` | Tauri 2, capabilities | siehe oben |
| SQL-Plugin (`tauri-plugin-sql`) | `vizu_notion_core` | SQL aus dem Webview umgeht die Prüfung in core — und die CLI sähe es nie |
| `tauri-plugin-store` für Einstellungen | `config.toml` über `core` | Die CLI muss dieselben Einstellungen lesen |
| Next.js, Remix, SSR | Vite + React | Es gibt keinen Server |
| Redux, Zustand, MobX | `useState` in `App.tsx` | Zu klein dafür |
| Tailwind, styled-components | `theme.css` + `app.css` | Farben an einer Stelle |
| ESLint + Prettier | Biome | Ein Werkzeug, keine Plugin-Versionskonflikte |
| Jest | Vitest | Teilt die Vite-Konfiguration |
| Pakete von einem CDN | npm | Die Anwendung muss ohne Netz starten |
| `dangerouslySetInnerHTML` | `lib/markdown.ts` | Bereinigung an einer Stelle |
| Auto-Update-Plugin | Release auf GitHub | Braucht Signaturschlüssel und Server |
| `unsafe` | — | `unsafe_code = "forbid"` im Arbeitsbereich |

---

## Tests

| Wo | Was |
|---|---|
| `crates/core/tests/source.rs` | Quellen: Prüfung, Namen, Import. **Hier liegt der Schwerpunkt.** |
| `crates/core/tests/fetch.rs` | Abruf und Zwischenspeicher, gegen festgehaltene Notion-Antworten |
| `crates/core/tests/rows.rs` | Notion-Seiten → Zeilen: Spaltenarten, Relationen, Ausblenden |
| `crates/core/tests/template.rs` | **Die Vorlagen-Engine gegen die Referenz der Webapp — bytegleich** |
| `crates/core/tests/notion.rs` | Takt, Wiederholung bei 429, Deutung der Fehler |
| `crates/core/tests/secret.rs` | Token: Herkunft, Prüfung, Speicher |
| `crates/core/tests/config.rs` | Einstellungen lesen, schreiben, ablehnen |
| `crates/core/tests/layering.rs` | Der Schichtwächter — Rust **und** TypeScript |
| `crates/cli/tests/cli.rs` | Ausgabeformat, Rückgabewerte, Schnappschüsse |
| `crates/desktop/tests/ipc_contract.rs` | Befehle über `tauri::test` ohne Fenster; Befehlsliste Rust ↔ api.ts |
| `crates/desktop/tests/bindings.rs` | `ui/src/bindings.ts` ist aktuell |
| `ui/src/App.test.tsx` | Oberfläche durchklicken, Rust durch `mockIPC` ersetzt |
| `ui/src/lib/lib.test.ts` | Markdown-Bereinigung, Verweise, Farben, Fehlerhülle |

**Eine Regel wird einmal geprüft — in `core`.** Die Attrappe in
`App.test.tsx` hat keine eigenen Regeln; sie lehnt nur ab, was der Test ihr
sagt.

Ändert sich ein CLI-Ausgabeformat absichtlich: `just snapshots`.

---

## Vor jedem Commit

```bash
just check
```

Das ist `cargo fmt --check`, Clippy, Biome, `tsc`, alle Rust- und
Vitest-Tests, `cargo deny` und `npm audit`. Warnungen sind Fehler.

---

## Eine neue Ressource anlegen

`source` ist die Vorlage. Der Weg von der Tabelle bis in beide Schalen:

1. `crates/core/migrations/000X_….sql` — neue Datei, `STRICT`.
2. `crates/core/src/db.rs` — in `MIGRATIONS` eintragen.
3. `crates/core/src/source.rs` kopieren, umbenennen; `#[derive(TS)]` an die
   Typen, die über IPC gehen.
4. `crates/core/src/lib.rs` — Modul veröffentlichen.
5. Tests in `crates/core/tests/` — **zuerst**.
6. CLI: `crates/cli/src/args.rs` und `commands/` — beide Ausgabefassungen.
7. Desktop: `commands.rs` → `lib.rs` (`generate_handler!`) →
   `tests/bindings.rs` (Typ eintragen) → `just bindings`.
8. Oberfläche: `api.ts` → Komponente in `components/` → Zustand und Aufruf in
   `App.tsx`.

Ausführlich mit Code: `docs/RECIPES.md`.
