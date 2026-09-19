# Die Kommandozeile

`vizu-notion` ist die zweite Schale über derselben Fachlogik wie die
Desktop-Anwendung. Beide arbeiten auf derselben Datenbank und demselben Token.

```bash
vizu-notion token set                       # Token, verdeckt eingegeben
vizu-notion source add Projekte --database 396f6627… --map title=Name --map next=Nächstes
vizu-notion fetch                           # alle Quellen abrufen
vizu-notion source list
```

## Der Token

Er wird **nie** als Argument übergeben — sonst stünde er in der Shell-Historie.

```bash
vizu-notion token set            # fragt verdeckt nach
pass notion | vizu-notion token set   # oder aus einem Passwortspeicher
vizu-notion token status         # zeigt nur Herkunft und ntn_…stuv
vizu-notion token clear
```

Gespeichert wird im **Schlüsselbund des Systems** (Keychain, Secret Service).
Zwei Auswege, wenn es keinen gibt oder ein Skript ihn nicht benutzen soll:

| Variable | Wirkung |
|---|---|
| `VIZU_NOTION_TOKEN` | wird direkt benutzt und hat **Vorrang** vor dem Gespeicherten |
| `VIZU_NOTION_TOKEN_FILE` | Datei statt Schlüsselbund; wird mit Rechten 0600 angelegt |

Ein Token braucht Leserecht auf die Datenbanken, und die Datenbanken müssen in
Notion mit der Integration **geteilt** sein (Seite → ••• → Verbindungen).

## Quellen

Eine Quelle ist eine Notion-Datenbank unter einem Namen. Unter diesem Namen
benutzen Vorlagen sie später (`{{#each Projekte}}`).

```bash
vizu-notion source add Projekte --database <ID> --map title=Name --map next=Nächstes
vizu-notion source add Projekte --database <ID> --auto    # Zuordnung vorschlagen lassen
vizu-notion source list
vizu-notion source show Projekte
vizu-notion source edit Projekte --map date=Start --unmap next
vizu-notion source rm Projekte --yes
```

* **`--database`** nimmt alles, was man aus Notion kopieren kann — am
  einfachsten den Link aus „Link kopieren" (`https://app.notion.com/p/…`),
  aber auch die 32 Zeichen, eine UUID mit Bindestrichen oder `Projekte-396f…`.
  Alles ab `?` fällt weg: Hinter `?v=` steht die Kennung der *Ansicht*, die
  ebenfalls 32 Zeichen hat und leicht mit der Datenbank verwechselt würde.
* **`--map ROLLE=SPALTE`** ordnet eine Notion-Spalte einer Rolle zu. Rollen sind
  Buchstaben, Ziffern und `_` (`title`, `next`, `parent`, `tag`, `status`,
  `date`) — genau so heißen sie in Vorlagen. `id` ist vergeben: das ist immer
  die Seiten-ID.
* **`--auto`** fragt die Spalten bei Notion ab und leitet die Zuordnung daraus
  ab: `title` ist die Titelspalte, `next` eine Relation auf dieselbe Datenbank,
  `date` ein Datum, `tag` eine Mehrfachauswahl. Bei mehreren Bewerbern
  entscheidet der Name; wo die Wahl geraten wäre, wird nichts vorgeschlagen.
  Jede übrige Spalte kommt unter einer Rolle aus ihrem Namen dazu
  („Verantwortlich" → `verantwortlich`), damit Vorlagen sie erreichen. Ein
  eigenes `--map` hat Vorrang — auch eine Spalte, die dort unter einer anderen
  Rolle steht, kommt nicht doppelt. `source inspect <LINK>` zeigt den
  Vorschlag, ohne etwas anzulegen. Braucht Netz und Token — es ist derselbe
  Vorschlag, den der Dialog im Fenster anbietet.
* **`edit`** ersetzt nur, was genannt wird. Zeigt eine Quelle danach auf eine
  **andere** Datenbank, werden die zwischengespeicherten Seiten verworfen.

### Aus der Webapp übernehmen

```bash
vizu-notion source import sources.json
cat sources.json | vizu-notion source import -
```

Liest die `sources.json` der Webapp vizu-notion-local. **Alles oder nichts:**
Ist ein Eintrag ungültig oder gibt es einen Namen schon, wird nichts angelegt,
und der Fehler nennt den Eintrag: `sources[2].name`.

## Abrufen

```bash
vizu-notion fetch                # alle Quellen
vizu-notion fetch Projekte Ziele # nur diese
vizu-notion fetch --json | jq '.[].page_count'
```

Der Abruf holt Schema und **alle** Seiten (er blättert), lädt Relationen mit
mehr als 25 Zielen nach und ersetzt den Zwischenspeicher der Quelle in einer
Transaktion. Höchstens drei Anfragen je Sekunde; ein „zu viele Anfragen" von
Notion wird abgewartet und wiederholt.

Passt eine zugeordnete Spalte nicht zum Schema in Notion, bricht der Abruf mit
Rückgabewert 4 ab und nennt Rolle und vorhandene Spalten — meist ein Tippfehler.

Fortschritt geht auf stderr, die Nutzausgabe auf stdout. Mehrere Quellen werden
nacheinander abgerufen; beim ersten Fehler bricht der Befehl ab, bereits
Abgerufenes bleibt gespeichert.

## Vorlagen und Diagramme

Eine Vorlage ist eine `.mmd`-Datei: ein Kopf mit `title` und `sources`, darunter
ein Mermaid-Diagramm mit Handlebars-Bindungen. Das Format ist **dasselbe wie in
der Webapp** vizu-notion-local, und die Ausgabe ist bytegleich.

```bash
vizu-notion template import diagramm.mmd      # Kurzname = Dateiname ohne .mmd
vizu-notion template import config/           # alle .mmd darin
vizu-notion template list
vizu-notion template show diagramm
vizu-notion template rm diagramm --yes
```

**Auf `subgraph`- und `classDef`-Zeilen greift der Umschreiber nicht** — dort
steht Mermaid-Syntax, keine Bindung. Wer dort ein Feld braucht, ruft den Helfer
selbst auf, sonst landet der rohe Text im Diagramm und Mermaid stolpert über
Anführungszeichen oder Klammern darin:

```
subgraph {{nodeId "title" title}}      ✅  ergibt  subgraph nXXXXXX["Q3 Review & Plan"]
subgraph {{this.title}}                ❌  ergibt  subgraph Q3 "Review" & [Plan]
classDef cls-{{classId "status" status}} fill:#4e79a7   ✅  nur die Kennung
classDef cls-{{nodeId "status" status}} fill:#4e79a7    ❌  Kennung mit Kasten
```

Ein erneuter Import **ersetzt** die Vorlage mit demselben Kurznamen. Fehlt im
Kopf `title` oder `sources`, oder gibt es eine genannte Quelle nicht, wird
nichts gespeichert (Rückgabewert 4).

```bash
vizu-notion render diagramm > diagramm-fertig.mmd
vizu-notion render diagramm --hide <seiten-id>,<seiten-id>
vizu-notion render diagramm --json | jq -r '.nodes[].title'
```

`render` rechnet **ohne Netz** — es benutzt, was `fetch` zuletzt geholt hat.
Auf stdout steht nur der Mermaid-Text; damit lässt er sich weiterreichen, etwa
an [mermaid.live](https://mermaid.live) oder `mmdc`. Mit `--json` kommen
zusätzlich `title` und `nodes`: jede Seite der beteiligten Quellen mit
Kennung, Titel, Quelle und ihren Relationen — die Grundlage des Filterfelds in
der Oberfläche.

Mit `--hide` bleiben Seiten aus dem Diagramm draußen, auch als Ziel einer
Relation. In `nodes` stehen sie weiterhin.

> Dieselben Dinge gehen auch in der Oberfläche: Quellen anlegen und ändern,
> Token speichern, Vorlagen bearbeiten mit Vorschau und das Diagramm als SVG
> speichern. Beide Schalen arbeiten auf derselben Datenbank.

### Eine Vorlage bauen lassen

Was der Assistent im Fenster tut, geht auch hier: Art, eine oder mehrere
Quellen und je Art ein paar Rollen, heraus kommt eine gewöhnliche Vorlage auf
stdout.

```bash
vizu-notion template new flowchart Projekte --link next --group status --color tag
vizu-notion template new flowchart Aufgaben Projekte --link Aufgaben.projekt=Projekte \
    --link Projekte.next --by-source --color-by-source
vizu-notion template new pie Aufgaben --group erledigt --sum punkte > punkte.mmd
vizu-notion template new gantt Projekte Aufgaben --date date
vizu-notion template new mindmap Projekte --group parent
vizu-notion template import punkte.mmd
```

| Art | Optionen |
|---|---|
| `flowchart` | `--link QUELLE.ROLLE[=ZIEL]` Pfeile, auch zu einer anderen Quelle · `--group` Rahmen je Wert · `--by-source` Rahmen je Quelle · `--color` Farbe je Wert · `--color-by-source` Farbe je Quelle |
| `pie` | eine Quelle: `--group` ein Stück je Wert (**Pflicht**); mehrere: ein Stück je Quelle · `--sum` Summe statt Anzahl |
| `gantt` | `--date` das Datum (**Pflicht**) · `--group` Abschnitt je Wert; mehrere Quellen: ein Abschnitt je Quelle |
| `mindmap` | `--group` Zweig je Wert; mehrere Quellen: ein Zweig je Quelle |

Bei einer einzigen Quelle genügt `--link next`.

* **Gezählt wird je Seite**, nicht je Zeile: Eine Seite mit drei Zielen steht im
  Kreisdiagramm einmal, nicht dreimal.
* **Gleiche Werte, gleiche Farbe** — auch über Quellen hinweg. Die Farben
  vergibt core einmal für das ganze Diagramm; eine eigene `classDef` in der
  Vorlage hat Vorrang.
* **Die Auswahl steht im Kopf** (`assistant: {…}`). Das Fenster öffnet sie über
  „Im Assistenten ändern" wieder.
* Ein Quellname mit Leerzeichen taugt nicht für Vorlagen — der Befehl sagt das,
  statt eine kaputte Vorlage zu bauen.

## Ohne Vorlage: Fluss und Metro-Karte

Hat eine Quelle die Rolle `next`, lässt sich ihr Ablauf ohne Vorlage zeichnen —
die Kanten entstehen aus der Relation:

```bash
vizu-notion flow Projekte
vizu-notion flow Projekte --sub status        # zweite Zeile im Knoten
vizu-notion flow Projekte --json | jq '.nodes[0]'
```

Für Menschen kommen die Knoten nach Ebenen sortiert heraus, mit ihren
Nachfolgern. Das JSON enthält zusätzlich **x und y in Punkten** — dieselbe
Anordnung, mit der die Oberfläche zeichnet. Sie steht in `core`, damit beide
Schalen dasselbe Bild ergeben.

Kommt zur Rolle `next` noch `date` dazu, geht auch eine **Metro-Karte**: Jede
Kette von Nachfolgern wird eine Linie, das Datum bestimmt die Stelle auf der
Zeitachse, und `tag` (ersatzweise `parent`) legt Bänder dahinter.

```bash
vizu-notion metro Projekte
vizu-notion metro Projekte --json | jq '.lines[].label'
```

Seiten ohne lesbares Datum stehen in keiner Linie. Sie werden nicht
verschluckt: Die Ausgabe nennt sie am Ende, und `undated` im JSON führt sie auf.

Anders als in einer Vorlage ist in beiden Ansichten jede Seite ein eigener
Knoten, auch bei gleichem Titel: Der Graph kommt aus den Relationen, und die
kennen Seiten, keine Texte.

## Gespeicherte Ansichten

Eine Ansicht hält fest, welches Diagramm gemeint ist und was daran eingestellt
war: ausgeblendete Seiten und beim Fluss die zweite Zeile. Sie ersetzt die
Teilen-Links der Webapp — für die es hier keinen Server gibt.

```bash
vizu-notion view add "Roadmap ohne Altlasten" --metro Projekte --hide 3dcf…,8a12…
vizu-notion view add "Fluss mit Status" --flow Projekte --sub status
vizu-notion view add "Fahrplan" --template fahrplan
vizu-notion view list
vizu-notion view show "Roadmap ohne Altlasten"
vizu-notion view rm "Fahrplan" --yes
```

`view show` zeichnet dasselbe wie `render`, `flow` oder `metro` mit denselben
Optionen — nur muss man sie nicht wieder eintippen. Auch `--json` gilt.

Gespeichert wird die **Kennung** der Vorlage bzw. der Quelle, nicht ihr Name:
Wird die Quelle umbenannt, zeigt die Ansicht weiter auf dieselbe. Wird sie
gelöscht, bleibt die Ansicht stehen und meldet beim Zeichnen, dass ihr Ziel
fehlt — stillschweigend mitlöschen wäre die falsche Voreinstellung.

## Kennungen

Quellen werden über ihren **Namen** angesprochen (ohne Rücksicht auf Groß- und
Kleinschreibung). Es geht auch die Kennung oder ihr **Endstück**, so wie
`source list` es zeigt:

```
ID        NAME      SEITEN  ABGERUFEN
2f40b1c7  Projekte      12  16.09.2026 09:12
9a17ee02  Ziele          5  16.09.2026 09:12
```

Warum das Ende und nicht der Anfang: Eine UUIDv7 beginnt mit dem Zeitstempel.
Zwei Quellen aus derselben Sekunde sähen am Anfang gleich aus. Ist ein Endstück
nicht eindeutig, bricht der Befehl mit Rückgabewert 5 ab und zeigt die Treffer.

## Rückgabewerte

| Wert | Bedeutung | Beispiel |
|---|---|---|
| 0 | in Ordnung | |
| 1 | allgemeiner Fehler | Platte voll, Schlüsselbund verweigert |
| 2 | falscher Aufruf | `--map title` ohne `=` — vergibt clap selbst |
| 3 | nicht gefunden | `vizu-notion source show Gibtsnicht` |
| 4 | Eingabe ungültig | leerer Name, unbekannte Spalte, `accent = "blau"` |
| 5 | Kennung nicht eindeutig | `vizu-notion source show a` bei mehreren Treffern |
| 6 | Notion | nicht erreichbar, nicht geteilt, Fehler von Notion |
| 7 | Token | keiner hinterlegt, oder Notion lehnt ihn ab |

Festgelegt in `crates/cli/src/exit.rs`, geprüft in `crates/cli/tests/cli.rs`.

## `--json`

**Jeder** Befehl kann JSON. Listen sind ein Array, damit `jq '.[]'` ohne Umweg
funktioniert:

```bash
vizu-notion source list --json | jq -r '.[] | "\(.source.name)\t\(.fetch.page_count // "—")"'
```

Zeitstempel stehen in UTC als RFC-3339-Text. Für Menschen rechnet die Ausgabe in
Ortszeit um, für Maschinen nie. Der Token erscheint in keiner Ausgabe.

```json
[
  {
    "source": {
      "id": "01a0a3cb-177a-759b-ade1-12c4280a20d3",
      "name": "Projekte",
      "database_id": "396f6627-0f5d-8034-b55c-ebc685aa5e50",
      "mappings": [{ "role": "title", "property": "Name" }],
      "created_at": "2026-09-16T06:39:51.162336Z",
      "updated_at": "2026-09-16T06:39:51.162336Z"
    },
    "fetch": {
      "source_id": "01a0a3cb-177a-759b-ade1-12c4280a20d3",
      "database_title": "vizu Projekte",
      "database_url": "https://app.notion.com/p/396f66270f5d8034b55cebc685aa5e50",
      "page_count": 12,
      "request_count": 4,
      "fetched_at": "2026-09-16T07:02:11.004112Z"
    }
  }
]
```

### Fehler als JSON

Gehen auf **stderr**, damit stdout leer bleibt und ein Skript nicht versehentlich
eine Fehlermeldung als Nutzdaten liest:

```json
{
  "error": {
    "code": "validation_failed",
    "message": "mappings.next: Spalte „Nachfolger“ gibt es in „vizu Projekte“ nicht (vorhanden: Name, Nächstes, Start)",
    "fields": [
      { "field": "mappings.next", "message": "Spalte „Nachfolger“ gibt es in „vizu Projekte“ nicht (vorhanden: Name, Nächstes, Start)" }
    ]
  }
}
```

| `error.code` | Rückgabewert | Zusatzfeld |
|---|---|---|
| `validation_failed` | 4 | `fields` |
| `not_found` | 3 | |
| `ambiguous_id` | 5 | `matches` |
| `token_missing` | 7 | |
| `notion_unauthorized` | 7 | |
| `notion_not_shared` | 6 | |
| `notion_unreachable` | 6 | |
| `notion_error` | 6 | |
| `secret_store_unavailable` | 1 | |
| `config_invalid` | 1 | |
| `internal_error` | 1 | |

## In Skripten und Pipelines

```bash
# Token aus einem Passwortspeicher, ohne Schlüsselbund
VIZU_NOTION_TOKEN="$(pass notion)" vizu-notion fetch --json

# Eigene Verzeichnisse — der saubere Weg für Tests und CI
VIZU_NOTION_DATA_DIR=/tmp/t/data VIZU_NOTION_CONFIG_DIR=/tmp/t/config vizu-notion source list
```

`source rm` und `view rm` fragen nach, wenn stdin ein Terminal ist. Ist es keins, bricht der
Befehl ab und verweist auf `--yes`. Stillschweigend zu löschen, weil niemand
antworten kann, wäre die falsche Voreinstellung.

## Einstellungen

Dieselbe Datei, die auch die Oberfläche liest und schreibt.

```bash
vizu-notion config show
vizu-notion config set theme dark          # system | light | dark
vizu-notion config set accent "#aa3344"    # #rrggbb
vizu-notion config reset
```

Ein ungültiger Wert wird abgelehnt — der alte Stand bleibt unverändert stehen.

## Wo liegt was

```bash
vizu-notion paths
vizu-notion paths --json | jq -r .db_file
```

| | macOS | Linux |
|---|---|---|
| Daten | `~/Library/Application Support/vizu-notion` | `~/.local/share/vizu-notion` |
| Konfiguration | `~/Library/Application Support/vizu-notion` | `~/.config/vizu-notion` |

Überschreiben mit `--data-dir` / `--config-dir` oder den Umgebungsvariablen
`VIZU_NOTION_DATA_DIR` / `VIZU_NOTION_CONFIG_DIR`.

## Protokoll

Geht auf stderr, nie auf stdout.

```bash
vizu-notion -v fetch           # info
vizu-notion -vv fetch          # debug: jede Anfrage an Notion
VIZU_NOTION_LOG=vizu-notion=trace vizu-notion fetch
```

## Vervollständigung und Handbuch

```bash
vizu-notion completions zsh  > ~/.zfunc/_vizu-notion
vizu-notion completions bash > /etc/bash_completion.d/vizu-notion
vizu-notion man --out ~/.local/share/man/man1
```

Beides erzeugt die Anwendung aus ihrer eigenen Befehlsstruktur — es kann also
nicht veralten. Der Tarball aus `just package-cli` enthält beides fertig.

Diese beiden Befehle brauchen **keine** Datenbank; sie laufen auch dort, wo das
Datenverzeichnis nicht beschreibbar ist.
