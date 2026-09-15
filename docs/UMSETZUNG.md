# Umsetzungsplan: vizu-notion-local als Desktop-Anwendung

Vorlage ist die Webapp [vizu-notion-local](https://github.com/kleineLoesungen/vizu-notion-local)
(Nuxt 4, Vue 3, rund 7.000 Zeilen). Ziel ist dieselbe Kernfunktion auf dem
Stack dieses Repos (`core` · `cli` · `desktop` · `ui/`, siehe `CLAUDE.md`):

> Notion-Datenbanken über Konfiguration, ohne Codeänderung, als Diagramm
> darstellen — vor allem als **Mermaid-Diagramm aus Vorlagen mit Live-Daten**.

Stand: 2026-09-15. Die Entscheidungen stehen in
[Abschnitt 7](#7-entscheidungen) und sind im Text mit **(E1)** usw. markiert.
Alle Empfehlungen wurden am 2026-09-15 so angenommen.

---

## 1. Was die Webapp tut — und wo es hier hingehört

| Webapp | Funktion | Hier |
|---|---|---|
| `server/utils/config.ts` | `sources.json` lesen und prüfen (ajv) | `core::source` — Prüfung mit `Validator` |
| `server/utils/notion.ts`, `rate-limiter.ts` | Notion abfragen, blättern, 3 Anfragen/s, LRU-Cache 1 h | `core::notion` (HTTP) + `core::cache` (SQLite statt Arbeitsspeicher) |
| `server/utils/relations.ts`, `mermaid/[templateId].get.ts` | Eigenschaften flach machen, Relationen zu Titeln auflösen, Mehrfachrelationen ausmultiplizieren | `core::rows` — reine Funktionen |
| `server/utils/templates.ts` | `.mmd` lesen (Frontmatter + Handlebars), `{{feld}}` → `nXXXXXX["…"]` umschreiben, Helfer, `classDef` | `core::template` |
| `mermaid/preview.post.ts`, `pages/mermaid-editor.vue` | Editor mit Vorschau | `core::template` + `ui` |
| `composables/useMetrovizData.ts` | Seiten → Linien, Stationen, Zonen | `core::metro` (Daten) |
| `vendor/metroviz/*` | Metro-Karte zeichnen (MIT) | `ui/src/lib/metroviz/` |
| `composables/useFlowData.ts`, `FlowDiagram.vue` | Fluss-Layout (Kahn) + SVG | `core::flow` (Layout) + `ui` (SVG) |
| `useFilterState.ts`, `FilterPanel.vue` | Knoten aus-/einblenden, Eigenschaftsfilter, „Show related" | Zustand in `App.tsx`; Nachbarschaft und Filter in `core` |
| `share.post.ts`, `state-encoding.ts` | Teilen-Link im lokalen Netz | entfällt → gespeicherte Ansichten **(E5)** |
| `useExport.ts` | SVG herunterladen | Speichern-Dialog + Befehl in Rust |
| `NodeDetailPanel.vue`, `NotionLinksList.vue` | Details, Link zu notion.so | `ui` + `api.openExternal` |

Die Faustregel aus `CLAUDE.md` gilt unverändert: **Was die CLI auch können
muss, steht in `core`.** Deshalb erzeugt `core` den Mermaid-Text und die
Diagrammdaten, und `ui/` zeichnet nur. `vizu-notion render <vorlage>` liefert
damit denselben Text wie das Fenster.

---

## 2. Aufbau

```
                 Notion-API (HTTPS, Token)
                          │
┌─────────────────────────┴────────────────────────────────────────────┐
│ crates/core                                                          │
│                                                                      │
│  notion ──▶ cache (SQLite) ──▶ rows ──┬──▶ template ──▶ Mermaid-Text │
│  (HTTP, Blättern,                     ├──▶ metro    ──▶ MetroData    │
│   Rate-Limit)                         └──▶ flow     ──▶ FlowGraph    │
│                                                                      │
│  source · template_store · view · secret      (Prüfung: Validator)   │
└──────────────┬──────────────────────────────────────┬────────────────┘
               │                                      │
      crates/cli (clap)                     crates/desktop (Tauri)
      source / fetch / render                 async-Befehle, spawn_blocking
                                                      │ IPC
                                              ui/ (React)
                                              mermaid.js · metroviz · SVG
```

### 2.1 Neue Rust-Abhängigkeiten

Alle in `[workspace.dependencies]`, jede einzeln mit `just check` und `cargo deny`.

| Kiste | Wofür | Anmerkung |
|---|---|---|
| `ureq` 3.4 (`rustls-no-provider`, `platform-verifier`, `gzip`, `json`) + `rustls` 0.23 (`ring`) | Notion-HTTP | Blockierend, kein tokio in `core`, kein OpenSSL, Systemzertifikate. Läuft im Desktop über `spawn_blocking`. Braucht CDLA-Permissive-2.0 in `deny.toml` (S3). |
| `handlebars` 6.4 | Vorlagen | bytegleich zur Webapp (S1) |
| `serde-saphyr` 1.2 | Frontmatter | `serde_yaml` ist eingestellt (S2) |
| `serde_json` mit `preserve_order` | Reihenfolge der `styles` | sonst weicht `classDef` ab (S1) |
| `regex` | Umschreiber der Vorlagen | dieselben Muster wie in `templates.ts` |
| `keyring` *(nur bei E2 = Schlüsselbund)* | Token | Unter Linux Secret Service, was neben WebKitGTK kaum ins Gewicht fällt |

### 2.2 Datenbank (Migration `0001_init.sql` neu, weil noch unveröffentlicht)

```sql
sources          (id TEXT PK, name TEXT UNIQUE, database_id TEXT, created_at, updated_at) STRICT
column_mappings  (source_id, role TEXT, property TEXT, PRIMARY KEY (source_id, role)) STRICT
templates        (id TEXT PK, slug TEXT UNIQUE, body TEXT, created_at, updated_at) STRICT   -- ganze .mmd inkl. Frontmatter
pages            (source_id, page_id TEXT, properties TEXT /*JSON*/, fetched_at, PRIMARY KEY (source_id, page_id)) STRICT
page_titles      (page_id TEXT PK, title TEXT, fetched_at) STRICT                           -- Relationsziele außerhalb der Quellen
fetches          (source_id PK, fetched_at, schema TEXT /*JSON*/) STRICT
views            (id TEXT PK, name TEXT, state TEXT /*JSON*/, created_at, updated_at) STRICT  -- (E5)
```

Die `note`-Tabelle und alles darum entfallen (Phase 1).

---

## 3. Kompatibilität mit der Webapp

**Empfehlung (E3):** Vorhandene `sources.json` und `.mmd`-Dateien lassen sich
importieren und liefern **bytegleichen Mermaid-Text**. Dafür:

* **Goldene Dateien:** Die Node-Pipeline der Webapp läuft einmalig auf
  festgehaltenen Eingaben und schreibt den erwarteten `diagramString`. Die
  Dateien liegen unter `crates/core/tests/fixtures/`, und `core` muss sie
  genau treffen. Node wird dafür nur ein einziges Mal gebraucht, nicht im
  Build. Für die Vorlagen-Engine sind sie seit Spike S1 vorhanden; für
  `core::rows` (Notion-Antworten → Zeilen) kommen sie in Phase 2 dazu.
* **Knotenkennung genau nachbauen:** FNV-1a über **UTF-16-Codeeinheiten**
  (`charCodeAt`), nicht über UTF-8-Bytes, sonst weichen Kennungen bei
  Emoji ab. Ausgabe: `n` + base36, auf 6 Stellen aufgefüllt. Scope
  `quelle\0gruppe`.
* **HTML-Escaping:** Handlebars.js maskiert ``& < > " ' ` =``. In Rust wird
  genau diese Maskierung als eigene `escape_fn` gesetzt.

### 3.1 Eigenheiten der Webapp — übernehmen oder beheben? **(E6)**

Beim Lesen aufgefallen. Jede Behebung ändert die Ausgabe gegenüber der Webapp.

| Eigenheit | Folge | Vorschlag |
|---|---|---|
| `title`/`rich_text` nimmt nur das **erste** Textstück (`[0].plain_text`) | Titel mit Formatierung werden abgeschnitten | beheben (alle Stücke verbinden) |
| Notion-Typen `status`, `people`, `formula`, `rollup`, `unique_id`, `created_time` fehlen in der Mermaid-Pipeline → `""` | Die README nennt `status` als Rolle, in Mermaid-Vorlagen ist sie aber leer | beheben |
| Relationstyp wird an der **ersten Seite** erkannt | Leere erste Seite → Relation wird nicht aufgelöst | beheben: Schema aus `retrieve database` nehmen |
| Relationen mit mehr als 25 Verknüpfungen (`has_more`) werden nicht nachgeladen | fehlende Kanten ohne Hinweis | beheben (S3) |
| Knoten gleichen Textes verschmelzen innerhalb einer Quelle | gewollt (dokumentiert) | übernehmen |
| Metro/Flow und Mermaid rufen Notion über zwei getrennte Wege ab | doppelte Logik | zusammenführen in `core::rows` |

In Spike S1 kamen weitere Befunde dazu. Sie betreffen die **Vorlagen-Engine**
selbst; eine Änderung dort verschiebt Knotenkennungen in bestehenden
Diagrammen. Deshalb: Engine bleibt bytegleich, behoben wird in Beispiel und Doku.

| Befund | Folge | Behandlung |
|---|---|---|
| `config/mermaid.example` beginnt mit `{{!-- … --}}` **vor** dem Frontmatter | Die mitgelieferte Vorlage wird mit „title fehlt" abgelehnt | eigenes Beispiel mit Kommentar hinter dem Frontmatter |
| Handlebars in `%%`-Kommentarzeilen wird ausgewertet | `%% Use {{fieldName}} …` erzeugt Knotentext im Kommentar, `{{#each …}}` im Kommentar öffnet echte Blöcke | in Beispiel und Doku `\{{…}}` schreiben |
| `classDef cls-{{nodeId "parent" parent}}` (so im Beispiel empfohlen) | ergibt `classDef cls-nXXXX["Wert"]` — ungültiges Mermaid | Beispiel und Doku korrigieren; ggf. eigener Helfer `classId` (nur Kennung) |
| Quellnamen mit Bindestrich (`my-source`) erkennt der Umschreiber nicht als Quelle | Knotenkennungen ohne Quellbezug; `Quelle.feld`-Stile greifen nicht | dokumentieren; beim Anlegen einer Quelle warnen |
| Innerhalb von `group-item` fließt der Gruppenschlüssel in die Kennung ein | Dieselbe Seite bekommt in und außerhalb einer Gruppe verschiedene Knoten | dokumentieren |
| `{{this.feld}}` wird HTML-maskiert (`&amp;`, `&quot;`) | Entitäten stehen im Mermaid-Text | dokumentieren, `{{{this.feld}}}` als Ausweg nennen |

### 3.2 Notion-API-Version

Die Webapp nutzt `@notionhq/client` 2.3 (API `2022-06-28`). Mit `2025-09-03`
hat Notion **Datenquellen** eingeführt (`/v1/data_sources/{id}/query`).
Geplant ist `2025-09-03`: Der Endpunkt ist erreichbar (S3). Die Datenbank-ID aus
`sources.json` wird beim Abruf über `retrieve database` in ihre Datenquelle(n)
aufgelöst. Die Version steht als Konstante an **einer** Stelle in `core::notion`.

---

## 4. Phasen

Jede Phase endet mit grünem `just check` und einem Commit. Tests in `core` kommen zuerst.

### Phase 0 — Spikes (Risiken klären, Wegwerfcode)

| # | Frage | Erfolgreich, wenn |
|---|---|---|
| S1 | Kann handlebars-rust die Vorlagen der Webapp? Bindestriche in Helfer- und Quellnamen (`group-item`, `join-rows`, `my-source`), Teilausdrücke, `@root`, `../`, `@first`, Block-Helfer | `mermaid.example` und zwei echte Vorlagen rendern gleich |
| S2 | YAML-Frontmatter-Kiste | `styles` mit `Quelle.feld`-Schlüsseln wird gelesen, `cargo deny` bleibt grün |
| S3 | Notion-API-Version, Blättern, 429/`Retry-After` mit `ureq` | echte Datenbank vollständig abgerufen |
| S4 | Metroviz im Webview: CSP, Farbregel (`color-utils.js` enthält Hexfarben → `theme.css`-Variablen oder Farben aus `core`) | Karte zeichnet im **gebündelten** Build, nicht nur in `just dev` |

Fällt S1 durch, ist die Alternative eine eigene, kleine Handlebars-Teilmenge in
`core`. Das kostet mehr Aufwand, das Format der Vorlagen bleibt aber gleich.

#### Ergebnisse (2026-09-15)

**S1 — bestanden.** Ein Nachbau von `templates.ts` mit `handlebars` 6.4.4
(MIT) trifft die Node-Referenz in **10 von 11 Fällen bytegleich**; im elften
lehnen beide die Vorlage ab. Geprüft: alle Helfer, Bindestriche in Helfer- und
Quellnamen, `@root`, `../`, `@../index`, `@first`/`@last`, `else`, Kommentare,
Leerzeilenbehandlung um Blöcke, Windows-Zeilenenden, Umlaute und Emoji im
Hash. Die Fälle liegen als goldene Dateien unter
`crates/core/tests/fixtures/templates/` (siehe README dort).

Was beim Nachbau zu beachten ist:

* Die Maskierung muss als eigene `escape_fn` wie Handlebars.js arbeiten
  (`&amp; &lt; &gt; &quot; &#x27; &#x60; &#x3D;`).
* `serde_json` braucht das Merkmal `preserve_order` — sonst stehen die
  `classDef`-Zeilen alphabetisch statt in der Reihenfolge der `styles`.
* `group-item` legt einen eigenen `BlockContext` an; `_groupKey` wird über
  `RenderContext::evaluate` gelesen.
* **Nicht unterstützt:** `{{liste.length}}`. handlebars-rust bricht mit Fehler
  ab, Handlebars.js liefert die Länge. Kommt in keiner dokumentierten Vorlage
  vor; der Fehler wird mit verständlicher Meldung durchgereicht.

**S2 — bestanden.** `serde-saphyr` 1.2 (MIT/Apache, reines Rust, YAML 1.2
wie js-yaml) liest das Frontmatter einschließlich `Quelle.feld`-Schlüsseln.
Frontmatter wird wie bei gray-matter nur am Dateianfang erkannt.

**S3 — bestanden, mit Befund (IPv6).** `ureq` 3.4 mit
`rustls-no-provider` + `platform-verifier` + `rustls` (ring) spricht TLS mit
`api.notion.com`; die Systemzertifikate werden benutzt (wichtig hinter
Firmen-Proxys mit eigener Zertifizierungsstelle). `Notion-Version: 2025-09-03`
und `/v1/data_sources/{id}/query` werden angenommen (401 ohne Token).

* `cargo deny` meldet `webpki-root-certs` mit **CDLA-Permissive-2.0** (die
  Mozilla-Zertifikatsliste, eine freizügige Datenlizenz ohne Weitergabepflichten
  für den Code). Sie kommt über `rustls-platform-verifier` herein und ist mit
  rustls nicht vermeidbar → in Phase 1 mit Begründung in `deny.toml` erlaubt.
* **Test gegen eine echte Datenbank** (17 Seiten; `title`, `rich_text`,
  `multi_select`, `date`, `url`, `files`):
  * `GET /databases/{id}` liefert mit `2025-09-03` **keine `properties`**
    mehr, sondern `data_sources: [{id, name}]`. Das Schema kommt aus
    `GET /data_sources/{id}`, die Seiten aus `POST /data_sources/{id}/query`.
  * `POST /databases/{id}/query` (Weg der Webapp) antwortet mit
    `2025-09-03` **400 `invalid_request_url`**; mit `2022-06-28` geht er noch.
  * Seiten tragen `parent: {type, data_source_id, database_id}`.
  * `multi_select` ist eine Liste von `{id, name, color}`. **Achtung:** Die
    Webapp macht daraus *einen* Text `"A, B"` — nur Relationen werden in
    mehrere Zeilen aufgefächert. Bleibt so (goldene Dateien), wird dokumentiert.
* **Befund Netzwerk:** Beim ersten Lauf hing der Abruf minutenlang im
  Verbindungsaufbau über **IPv6** (`SYN_SENT`). ureq wartet ohne
  `timeout_connect` unbegrenzt an der ersten Adresse und versucht IPv4 nie.
  Mit 10 s Verbindungs-Timeout lief es durch, die erste Anfrage kostete aber
  genau diese 10 s. **Für Phase 1:** eigener `ureq`-Resolver, der
  IPv4-Adressen zuerst probiert, plus Verbindungs- und Gesamt-Timeout; ein
  Abbruch wird als eigener Fehler („Notion nicht erreichbar") gemeldet.
* **Testdatensatz (2026-09-16):** Über eine eigene interne Integration mit
  Schreibrecht hat ein Skript unter einer Testseite drei verknüpfte
  Datenbanken angelegt (Ziele 5, Projekte 12, Aufgaben 130). Die Antworten
  liegen anonymisiert unter `crates/core/tests/fixtures/notion/`. Befunde:
  * **Blättern** geprüft: 130 Aufgaben kommen in zwei Stapeln, mit beiden
    API-Versionen gleich.
  * `status`-, `formula`- und selbstbezügliche `relation`-Spalten lassen sich
    über die API anlegen und werden gelesen (`status.name` z. B. „Done").
  * **Relationen tragen `has_more`.** Notion liefert im Seitenobjekt höchstens
    25 verknüpfte Seiten; darüber hinaus muss
    `GET /pages/{id}/properties/{property_id}` blättern. Die Webapp ignoriert
    das und verliert stillschweigend Relationen → **in Phase 1 beheben**.
  * Seitenadressen lauten `https://app.notion.com/p/…`. Die Webapp baut
    `https://notion.so/{id}` — die Anwendung übernimmt stattdessen das Feld
    `url` aus der Antwort.
  * Ein 429 trat bei 178 Anfragen mit 3/s nicht auf; der Fall wird mit einer
    nachgebauten Antwort getestet.

**S4 — auf den Beginn von Phase 5 verschoben** (E4: Metro kommt zuletzt, das
Risiko blockiert Phase 1–4 nicht). Befunde vorab:

* Metroviz erwartet `d3` und `i18next` als globale Objekte (die Webapp setzt
  beides vor dem Import) und benutzt `innerHTML` — zulässig in `ui/src/lib/`,
  nicht in Komponenten.
* `metroviz.css` enthält 145 Farbangaben. Der Farbwächter prüft `.css` → die
  Farben wandern als Variablen nach `theme.css`. `.js` prüft er nicht; die 36
  Farben dort (Linienpalette) sollen trotzdem aus `core` kommen, weil `core`
  die Linien ohnehin baut.

### Phase 1 — Fundament: Quellen, Token, Abruf

* `note` entfernen: core, CLI, Desktop, UI, Tests, Schnappschüsse, Migration.
* `core::source` mit Prüfung: Name eindeutig und nicht leer, `database_id` 32
  Hex-Zeichen (Bindestriche werden entfernt), Rollen nicht leer. Alle
  Feldfehler auf einmal.
* `core::secret`: Token setzen, lesen, löschen **(E2)**. `VIZU_NOTION_TOKEN` überschreibt.
* `core::notion` + `core::cache`: Schema holen, Mappings gegen das Schema
  prüfen (Feldfehler je Rolle), Seiten blättern, in `pages` speichern.
* CLI: `source add|list|show|rm|import <sources.json>`, `token set|clear`,
  `fetch [quelle]`. Beide Ausgabeformate, neue Rückgabewerte in `exit.rs`
  und `docs/CLI.md` (z. B. 5 = Notion nicht erreichbar oder Token ungültig).
* Tests: Notion-Antworten als Dateien. Der HTTP-Teil sitzt hinter einem Trait,
  damit Tests ohne Netz laufen.

### Phase 2 — Mermaid-Pipeline in core

* `core::rows`: Eigenschaften flach machen, Relationen auflösen (Titel aus
  Cache, sonst `retrieve page` → `page_titles`), ausblenden per `hidden`,
  Mehrfachrelationen ausmultiplizieren.
* `core::template`: Frontmatter, Umschreiber, Helfer (`nodeId`, `group`,
  `group-item`, `palette`, `join-rows`, `lookup-by`), `classDef`- und
  `class`-Zeilen. Rückgabe `RenderedDiagram { mermaid, rows }`.
* Vorlagen verwalten: `template import <datei|verzeichnis>`, `list`, `show`,
  `rm`. Prüfung beim Speichern: Frontmatter vollständig, Quellen existieren,
  Handlebars kompiliert.
* CLI: `render <vorlage> [--hide id,…] [--json]` gibt Mermaid-Text aus.
* Goldene Tests aus Abschnitt 3.

**Nach Phase 2 ist die Kernfunktion auf der Kommandozeile fertig.**

### Phase 3 — Oberfläche: Mermaid-Ansicht

* Befehle: `source_list`, `source_fetch` (über `spawn_blocking`),
  `template_list`, `diagram_render { template, hidden }`, `settings_*`.
* Aufbau: links Quellen und Vorlagen mit Abrufzeitpunkt und
  „Aktualisieren", Mitte das Diagramm, rechts ein einklappbares Filterfeld.
* `lib/mermaid.ts` erweitern: ein ganzes Diagramm rendern (nicht nur
  Markdown-Blöcke), Zoom und Verschieben ohne d3 (CSS-Transform über dem SVG).
* Filterfeld: Knoten nach Quelle gruppiert, Gruppe an/aus, „Show related"
  (1-Hop-Nachbarn kommen aus `core`, weil die Relationen dort liegen).
* Klick auf einen Knoten → Detailfeld → „In Notion öffnen" über `api.openExternal`.
* Fehler: Vorlagenfehler im Diagrammbereich, Notion-Fehler oben,
  Mapping-Fehler am Feld.

### Phase 4 — Verwalten in der Oberfläche

* Quellen anlegen und bearbeiten (Formular, Rollen als Liste, Eigenschaften
  als Auswahl aus dem abgerufenen Schema).
* Token in den Einstellungen.
* Vorlagen-Editor: Textfeld links, Vorschau rechts (verzögert, veraltete
  Aufträge verwerfen, wie in `MarkdownView`), Anzeige des erzeugten
  Mermaid-Textes zum Kopieren.
* SVG-Export: `tauri-plugin-dialog` für den Speicherort, Schreiben über einen
  Befehl in Rust. Downloads über `<a download>` gehen im Webview nicht.

### Phase 5 — Flussdiagramm und Metro-Karte **(E4)**

* `core::flow`: Graph und Layout (Kahn-Ebenen wie `useFlowData.ts`), `ui`
  zeichnet SVG.
* `core::metro`: Portierung von `useMetrovizData.ts` (Linien, Stationen,
  Zonen, Umstiege, mehrere Quellen, „Milestones/Line"). `ui/src/lib/metroviz/`
  enthält die übernommenen JS-Dateien mit Herkunftsvermerk und Lizenz.
* Automatische Auswahl wie in der Webapp: `next` → Fluss, `date`+`next` → Metro.

### Phase 6 — Ansichten, Feinschliff, Auslieferung

* Gespeicherte Ansichten als Ersatz für Teilen-Links **(E5)**: Diagrammart,
  Vorlage, ausgeblendete Knoten, Filter, Zusatzquellen.
* Symbol ersetzen (`assets/icon.png`, `just icons`), README und `docs/` auf
  vizu-notion umschreiben, `CLAUDE.md` um Notion-Fallstricke ergänzen.
* Bündel für macOS (`.dmg`) und Linux (`.deb`/`.AppImage`).

---

## 5. Was bewusst nicht übernommen wird

| Webapp | Warum nicht |
|---|---|
| Docker, `docker-compose`, Makefile | Desktop-Anwendung |
| Tailwind | `CLAUDE.md`: `theme.css` + `app.css` |
| Vue, Vue Flow, d3 | React; Fluss wird eigenes SVG, Zoom ohne d3 |
| Teilen-Links übers Netz | kein Server |
| „Neustart nach Konfigurationsänderung" | Änderungen gelten sofort |
| `.planning/` der Webapp | neuer Stack, neue Planung |

---

## 6. Risiken

| Risiko | Gegenmittel |
|---|---|
| handlebars-rust verhält sich anders als Handlebars.js | Spike S1, goldene Dateien, notfalls eigene Teilmenge |
| Mermaid 12 rendert anders als Mermaid 11 der Webapp | Mermaid-Text ist die Vertragsgrenze; Darstellung einmal mit echten Vorlagen ansehen |
| Metroviz-Stile im Bündel (Nonce/CSP, siehe `CLAUDE.md` Punkt 7) | Spike S4 im **gebauten** Bündel |
| Notion-Rate-Limit bei vielen Relationszielen | Cache in SQLite, Titel nur für fehlende Kennungen holen, 429 mit `Retry-After` |
| Große Datenbanken blockieren die Oberfläche | Abruf nur in `spawn_blocking`, Sperre nicht über den Netzaufruf halten |

---

## 7. Entscheidungen

Angenommen am 2026-09-15: jeweils die Empfehlung.

| # | Frage | Entscheidung |
|---|---|---|
| **E1** | Quellen und Vorlagen: in SQLite mit Bearbeitung in Oberfläche und CLI (plus Import von `sources.json`/`.mmd`) **oder** weiter als Dateien im Konfigurationsverzeichnis mit „Neu laden"? | SQLite + Import. Dateien als Export für Weitergabe. |
| **E2** | Notion-Token: Schlüsselbund des Systems **oder** `config.toml` (Rechte 0600)? | Schlüsselbund; `VIZU_NOTION_TOKEN` für CLI und Tests |
| **E3** | Müssen vorhandene `.mmd`-Vorlagen bytegleich dasselbe Mermaid ergeben? Gibt es echte Vorlagen und Datenbanken als Testmaterial? | Ja, mit goldenen Dateien |
| **E4** | Metro-Karte und Fluss in der ersten Fassung **oder** zuerst nur Mermaid? | Zuerst Mermaid (Phasen 1–4), dann Fluss, Metro zuletzt |
| **E5** | Ersatz für Teilen-Links: gespeicherte Ansichten, Export als Datei, oder weglassen? | Gespeicherte Ansichten |
| **E6** | Eigenheiten aus 3.1 beheben (Ausgabe weicht dann ab) oder genau übernehmen? | Beheben, Abweichungen in den goldenen Dateien markieren |
| **E7** | Oberflächensprache: Deutsch laut `CLAUDE.md`, die Webapp war englisch | Deutsch |
