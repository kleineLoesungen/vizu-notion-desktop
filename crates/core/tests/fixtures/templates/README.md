# Referenzfälle für die Vorlagen-Engine

Jeder Ordner ist ein Fall: `template.mmd` (Vorlage wie in der Webapp),
`sources.json` (Quellname → Datei mit den Zeilen) und `expected.mmd`.

`expected.mmd` stammt **nicht** aus diesem Repo, sondern aus der
Original-Logik der Webapp
vizu-notion-local:
`server/utils/templates.ts` und die Nachbearbeitung aus
`server/routes/api/mermaid/[templateId].get.ts`, mit handlebars 4.7.9 und
gray-matter 4.0.3. Das Skript dazu liegt als `referenz.ts.txt` daneben.
Beginnt `expected.mmd` mit `FEHLER:`, lehnt die Webapp die Vorlage ab — dann
zählt nur, *dass* abgelehnt wird, nicht der Wortlaut.

Die Dateien werden nicht von Hand geändert. Neuer Fall: Ordner anlegen, das
Skript mit Node 22 gegen eine Kopie der Webapp laufen lassen
(`node referenz.ts <dieser-ordner>`), Ergebnis ansehen, einchecken.

| Fall | Prüft |
|---|---|
| `basic` | `#each`, `#if`, `@index`, `@first`, `this.id` |
| `styles` | `styles` im Frontmatter, `Quelle.feld`, `classDef`, `class`-Zeilen |
| `group` | `group`, `group-item`, `palette`, `nodeId` von Hand |
| `join` | `join-rows` über zwei Quellen |
| `lookup` | `lookup-by`, `../` |
| `escape` | HTML-Maskierung, Klammern im Label, Umlaute, Emoji (UTF-16-Hash) |
| `hyphen` | Quellname mit Bindestrich |
| `edge` | `else`, `@last`, `@../index`, `@root`, Kommentare, fehlende Felder |
| `crlf` | Windows-Zeilenenden |
| `example` | `config/mermaid.example` der Webapp — wird abgelehnt (siehe docs/UMSETZUNG.md 3.1) |
| `example-fixed` | dasselbe mit dem Kommentar hinter dem Frontmatter |
