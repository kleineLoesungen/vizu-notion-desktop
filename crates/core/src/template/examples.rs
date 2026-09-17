//! Beispiele und Spickzettel zur Vorlagensprache.
//!
//! Beides gehört zur Sprache und damit nach `core`: Die Oberfläche zeigt es im
//! Editor, und die Kommandozeile kann dasselbe ausgeben. Eine zweite Liste in
//! TypeScript wäre eine zweite Wahrheit — und sie stünde voller Hexfarben, die
//! `crates/core/tests/layering.rs` in `ui/` zu Recht verbietet.

use serde::Serialize;
use ts_rs::TS;

/// Eine Vorlage zum Abschauen. `QUELLE` und `ZWEITE` ersetzt die Schale durch
/// die Namen der eingerichteten Quellen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct Example {
    pub name: String,
    /// Welche Art Mermaid-Diagramm dabei herauskommt.
    pub kind: String,
    pub body: String,
}

/// Eine Zeile des Spickzettels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct Hint {
    pub syntax: String,
    pub meaning: String,
}

/// Was der Editor als Beispiel anbietet.
pub fn examples() -> Vec<Example> {
    [
        (
            r##"Kanten aus einer Relation"##,
            r##"flowchart"##,
            r##"---
title: "Verbindungen"
sources:
  - QUELLE
---
flowchart LR
{{#each QUELLE}}
  {{#if next}}{{title}} --> {{next}}{{else}}{{title}}{{/if}}
{{/each}}
"##,
        ),
        (
            r##"Gruppen mit Farben"##,
            r##"flowchart + classDef"##,
            r##"---
title: "Nach Status"
sources:
  - QUELLE
---
flowchart TD
{{#each (group QUELLE "status")}}
  classDef cls-{{classId "status" status}} fill:{{palette @index}},color:#fff
  subgraph {{nodeId "status" status}}
  {{#group-item}}
    {{title}}:::cls-{{classId "status" status}}
  {{/group-item}}
  end
{{/each}}
"##,
        ),
        (
            r##"Formen und Farben je Feld"##,
            r##"flowchart + styles"##,
            r##"---
title: "Gestaltet"
sources:
  - QUELLE
styles:
  title:
    shape: rounded
  parent:
    shape: stadium
    fill: "#4e79a7"
    stroke: "#2d5a8e"
---
flowchart LR
{{#each QUELLE}}
  {{#if parent}}{{parent}} --> {{title}}{{/if}}
{{/each}}
"##,
        ),
        (
            r##"Zeitleiste aus einem Datum"##,
            r##"gantt"##,
            r##"---
title: "Zeitleiste"
sources:
  - QUELLE
---
gantt
  dateFormat YYYY-MM-DD
  axisFormat %m/%Y
  section Termine
{{#each QUELLE}}
  {{#if date}}{{this.title}} :milestone, {{this.date}}, 0d{{/if}}
{{/each}}
"##,
        ),
        (
            r##"Zwei Quellen verbinden"##,
            r##"flowchart + join-rows"##,
            r##"---
title: "Verbunden"
sources:
  - QUELLE
  - ZWEITE
---
flowchart LR
{{#each (join-rows QUELLE "parent" ZWEITE "title" "z")}}
{{#if z_title}}  {{title}} --> {{z_title}}
{{/if}}
{{/each}}
"##,
        ),
        (
            r##"Anzahl je Status"##,
            r##"pie"##,
            r##"---
title: "Verteilung"
sources:
  - QUELLE
---
pie showData
  title Anzahl je Status
{{#each (group QUELLE "status")}}
  "{{this.status}}" : {{len items}}
{{/each}}
"##,
        ),
        (
            r##"Termine auf einer Achse"##,
            r##"timeline"##,
            r##"---
title: "Termine"
sources:
  - QUELLE
---
timeline
  title Termine
{{#each QUELLE}}
{{#if date}}  {{this.date}} : {{this.title}}
{{/if}}
{{/each}}
"##,
        ),
        (
            r##"Karten je Status"##,
            r##"kanban"##,
            r##"---
title: "Tafel"
sources:
  - QUELLE
---
kanban
{{#each (group QUELLE "status")}}
  {{classId "status" status}}["{{this.status}}"]
{{#group-item}}
    {{classId "title" title}}["{{this.title}}"]
{{/group-item}}
{{/each}}
"##,
        ),
        (
            r##"Gedankenkarte aus der Überordnung"##,
            r##"mindmap"##,
            r##"---
title: "Gedankenkarte"
sources:
  - QUELLE
---
mindmap
  root)QUELLE(
{{#each (group QUELLE "parent")}}
    {{this.parent}}
{{#group-item}}
      {{this.title}}
{{/group-item}}
{{/each}}
"##,
        ),
        (
            r##"Zustände und Übergänge"##,
            r##"stateDiagram-v2"##,
            r##"---
title: "Übergänge"
sources:
  - QUELLE
---
stateDiagram-v2
{{#each QUELLE}}
  state "{{this.title}}" as {{classId "title" title}}
{{/each}}
{{#each QUELLE}}
{{#if next}}  {{classId "title" title}} --> {{classId "title" next}}
{{/if}}
{{/each}}
"##,
        ),
    ]
    .into_iter()
    .map(|(name, kind, body)| Example {
        name: name.to_string(),
        kind: kind.to_string(),
        body: body.to_string(),
    })
    .collect()
}

/// Die Bindungen der Vorlagensprache in Kurzform.
pub fn cheat_sheet() -> Vec<Hint> {
    [
        (
            r##"{{feld}}"##,
            r##"Knoten mit dem Wert als Beschriftung — die Kennung entsteht aus dem Wert"##,
        ),
        (
            r##"{{this.feld}}"##,
            r##"der Wert als roher Text, ohne Knoten (Gantt, Beschriftungen)"##,
        ),
        (
            r##"{{this.id}}"##,
            r##"die Seiten-ID — eindeutig, auch bei gleichem Titel"##,
        ),
        (
            r##"{{#each Quelle}} … {{/each}}"##,
            r##"über alle sichtbaren Zeilen einer Quelle"##,
        ),
        (
            r##"{{#if feld}} … {{else}} … {{/if}}"##,
            r##"nur wenn das Feld gefüllt ist"##,
        ),
        (
            r##"{{@index}} · {{@first}} · {{@last}}"##,
            r##"Nummer und Rand der Schleife"##,
        ),
        (
            r##"{{#each (group Quelle "feld")}}"##,
            r##"nach einem Feld gruppieren; innen {{#group-item}}"##,
        ),
        (
            r##"{{palette @index}}"##,
            r##"eine von zehn unterscheidbaren Farben"##,
        ),
        (
            r##"{{len items}}"##,
            r##"Anzahl der Zeilen einer Gruppe — für Kreisdiagramme"##,
        ),
        (
            r##"{{classId "feld" feld}}"##,
            r##"nur die Kennung — für classDef-Zeilen und fremde Diagrammarten"##,
        ),
        (
            r##"{{nodeId "feld" feld}}"##,
            r##"ganzer Knoten — auf subgraph-Zeilen nötig"##,
        ),
        (
            r##"(join-rows A "feld" B "feld" "vorsilbe")"##,
            r##"zwei Quellen über ein Feld verbinden"##,
        ),
        (
            r##"(lookup-by B "feld" wert)"##,
            r##"passende Zeilen einer anderen Quelle, verschachtelt"##,
        ),
    ]
    .into_iter()
    .map(|(syntax, meaning)| Hint {
        syntax: syntax.to_string(),
        meaning: meaning.to_string(),
    })
    .collect()
}
