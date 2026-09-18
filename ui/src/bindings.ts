// ERZEUGT aus den Rust-Typen — nicht von Hand ändern.
// Neu schreiben mit:  just bindings
// Quelle: crates/desktop/tests/bindings.rs

export type ApiError = { code: ErrorCode, message: string, fields?: Array<FieldError>, matches?: Array<string>, };

/**
 * Wo die Dateien liegen — dieselben Pfade wie `vizu-notion paths --json`.
 */
export type AppInfo = { version: string, data_dir: string, config_file: string, db_file: string, log_file: string, };

export type ColumnMapping = { 
/**
 * Wie die Vorlage das Feld nennt: `title`, `next`, `parent` …
 */
role: string, 
/**
 * Wie die Spalte in Notion heißt — genau so geschrieben.
 */
property: string, };

export type Config = { 
/**
 * Hell, dunkel oder der Systemeinstellung folgen.
 */
theme: Theme, 
/**
 * Akzentfarbe als `#rrggbb`. Siehe docs/DESIGN.md.
 */
accent: string, };

/**
 * Maschinenlesbare Fehlerart.
 *
 * Dieselben Namen stehen im JSON der Kommandozeile (`error.code`) und in der
 * Fehlerhülle der Desktop-Schale. Ein Skript und die Oberfläche können sich
 * darauf verlassen — ein Name wird deshalb nie umbenannt, nur ergänzt.
 */
export type ErrorCode = "not_found" | "validation_failed" | "ambiguous_id" | "config_invalid" | "internal_error" | "token_missing" | "notion_unauthorized" | "notion_not_shared" | "notion_unreachable" | "notion_error" | "secret_store_unavailable";

/**
 * Stand des letzten Abrufs einer Quelle.
 */
export type FetchStatus = { source_id: string, database_title: string, database_url: string, page_count: number, request_count: number, fetched_at: string, };

/**
 * Die Spalten einer Notion-Datenbank, mit einem Vorschlag für die Zuordnung.
 *
 * Für die Einrichtung: Beim Anlegen einer Quelle gibt es noch keinen Abruf,
 * aus dem die Spalten kämen — hier werden sie live geholt. Zwei Anfragen, mehr
 * nicht; die Seiten bleiben unangetastet.
 */
export type DatabaseSchema = { 
/**
 * Wie die Datenbank in Notion heißt — als Vorschlag für den Namen.
 */
title: string, 
/**
 * Die eigene Datenquelle. Eine Relation, die hierhin zeigt, verbindet
 * Seiten derselben Datenbank — nur so eine kann `next` sein.
 */
data_source_id: string, properties: Array<Property>, 
/**
 * Was core aus den Spalten für die Zuordnung ableitet.
 */
suggestion: Array<ColumnMapping>, };

export type FlowEdge = { from: string, to: string, };

export type FlowGraph = { nodes: Array<FlowNode>, edges: Array<FlowEdge>, 
/**
 * Größe der Zeichnung in Punkten — die Oberfläche passt danach ein.
 */
width: number, height: number, 
/**
 * Jede Seite der Quelle, auch die ausgeblendeten: das Filterfeld.
 */
all_nodes: Array<NodeInfo>, 
/**
 * Rollen, die als zweite Zeile taugen — für die Auswahl in der Oberfläche.
 */
subtitle_roles: Array<string>, };

export type FlowNode = { 
/**
 * Die Seiten-ID — dieselbe wie im Filterfeld.
 */
id: string, title: string, 
/**
 * Ein zweiter Wert unter dem Titel, etwa der Status. Leer, wenn keiner
 * gewählt ist.
 */
subtitle: string, x: number, y: number, };

/**
 * Ein Baustein: ein Stück Vorlage für ein wiederkehrendes Muster, das man an
 * der Schreibmarke einsetzt statt es abzutippen.
 *
 * Platzhalter: `QUELLE` und `ZWEITE` sind Quellnamen, `FELD` eine Rolle der
 * ersten Quelle. Die Oberfläche fragt nur nach dem, was der Baustein braucht.
 */
export type Block = { name: string, 
/**
 * Wozu er gut ist, in einem Satz.
 */
purpose: string, 
/**
 * Braucht eine Rolle für `FELD`.
 */
needs_field: boolean, 
/**
 * Braucht eine zweite Quelle für `ZWEITE`.
 */
needs_second: boolean, body: string, };

/**
 * Ein fertiges Diagramm.
 */
export type Diagram = { title: string, 
/**
 * Der Mermaid-Text, so wie ihn auch `vizu-notion render` ausgibt.
 */
mermaid: string, 
/**
 * Jede Seite der beteiligten Quellen — auch die ausgeblendeten, damit das
 * Filterfeld sie wieder einblenden kann.
 */
nodes: Array<NodeInfo>, };

/**
 * Eine Vorlage zum Abschauen. `QUELLE` und `ZWEITE` ersetzt die Schale durch
 * die Namen der eingerichteten Quellen.
 */
export type Example = { name: string, 
/**
 * Welche Art Mermaid-Diagramm dabei herauskommt.
 */
kind: string, body: string, };

/**
 * Eine Meldung, die zu genau einem Eingabefeld gehört.
 *
 * `field` ist englisch und entspricht dem Feldnamen im Modell (`name`,
 * `database_id`). Bei Listen steht der Eintrag dabei: `mappings.next` für
 * die Rolle `next`, `sources[2].name` beim Import. `message` ist deutsch und
 * für Menschen.
 */
export type FieldError = { field: string, message: string, };

/**
 * Ein verstecktes Diagramm.
 */
export type HiddenDiagram = { 
/**
 * `template`, `flow` oder `metro`.
 */
kind: string, 
/**
 * Kennung der Vorlage bzw. der Quelle.
 */
target: string, };

/**
 * Eine Zeile des Spickzettels.
 */
export type Hint = { syntax: string, meaning: string, };

export type MetroLine = { label: string, color: string, stations: Array<MetroStation>, 
/**
 * Woher die Linie abzweigt: die Station, deren zweiter Nachfolger sie
 * beginnt. Ohne diesen Punkt hinge eine Abzweigung in der Luft.
 */
entry: MetroPoint | null, 
/**
 * Wohin die Linie mündet, wenn ihr Nachfolger schon zu einer anderen
 * Linie gehört.
 */
exit: MetroPoint | null, };

export type MetroMap = { lines: Array<MetroLine>, zones: Array<MetroZone>, ticks: Array<MetroTick>, width: number, height: number, 
/**
 * Jede Seite der Quelle, auch die ausgeblendeten: das Filterfeld.
 */
all_nodes: Array<NodeInfo>, 
/**
 * Seiten ohne lesbares Datum. Sie stehen in keiner Linie — die Oberfläche
 * sagt es, statt sie stillschweigend zu verschlucken.
 */
undated: Array<string>, };

/**
 * Ein Punkt, an dem eine Linie eine andere trifft.
 */
export type MetroPoint = { x: number, y: number, };

export type MetroStation = { 
/**
 * Die Seiten-ID — dieselbe wie im Filterfeld.
 */
id: string, title: string, 
/**
 * Wie in Notion, als Text: `2026-04-01`.
 */
date: string, x: number, y: number, kind: StationKind, 
/**
 * Hier zweigt eine Linie ab oder mündet eine ein — eine Umsteigestation.
 */
interchange: boolean, 
/**
 * Beschriftung über oder unter der Station — sonst überlagern sie sich.
 */
label_above: boolean, };

/**
 * Eine Marke auf der Zeitachse.
 */
export type MetroTick = { label: string, x: number, };

/**
 * Ein Band hinter mehreren Spuren: alle Linien mit demselben `tag`/`parent`.
 */
export type MetroZone = { label: string, y: number, height: number, };

/**
 * Ein Knoten für das Filterfeld — jede Zeile, auch ausgeblendete.
 */
export type NodeInfo = { id: string, title: string, source: string, 
/**
 * Seiten-IDs, auf die diese Seite über irgendeine Relation zeigt.
 */
relations: Array<string>, };

/**
 * Eine Spalte aus dem Schema einer Datenquelle.
 *
 * Geht auch über die IPC-Grenze: Die Oberfläche bietet beim Zuordnen die
 * Spalten zur Auswahl an, die der letzte Abruf gesehen hat.
 */
export type Property = { name: string, 
/**
 * Kurzkennung der Spalte, z. B. `%5EGHx` — bereits für Adressen kodiert.
 */
id: string, 
/**
 * `title`, `relation`, `multi_select`, …
 */
kind: string, 
/**
 * Bei einer Relation: die Datenquelle, auf die sie zeigt. Nur so ist ein
 * Verweis auf dieselbe Datenbank („Nächstes") von einem auf eine andere
 * („Ziel") zu unterscheiden.
 */
relation_to: string | null, };

export type Source = { id: string, name: string, 
/**
 * Die Notion-Kennung, klein und mit Bindestrichen.
 */
database_id: string, 
/**
 * Nach Rolle sortiert.
 */
mappings: Array<ColumnMapping>, created_at: string, updated_at: string, };

/**
 * Was von außen hereinkommt — ungeprüft.
 */
export type SourceInput = { name: string, 
/**
 * Kennung, Seitenname mit Kennung oder Adresse aus Notion.
 */
database_id: string, mappings: Array<ColumnMapping>, };

/**
 * Eine Quelle mit dem Stand ihres letzten Abrufs — für Listen.
 */
export type SourceOverview = { source: Source, 
/**
 * `null`: noch nie abgerufen.
 */
fetch: FetchStatus | null, 
/**
 * Welche fertigen Ansichten die Zuordnung hergibt — ohne Vorlage.
 */
views: Array<ViewKind>, };

export type StationKind = "start" | "stop" | "terminus" | "single";

export type Template = { id: string, 
/**
 * Kurzname für die Kommandozeile — der Dateiname ohne `.mmd`.
 */
slug: string, 
/**
 * Aus dem Kopf: die Beschriftung in der Oberfläche.
 */
title: string, 
/**
 * Aus dem Kopf: die benutzten Quellen.
 */
sources: Array<string>, 
/**
 * Der ganze Text, Kopf inbegriffen.
 */
body: string, created_at: string, updated_at: string, };

/**
 * Was der Editor an Hilfe anzeigt.
 */
export type TemplateHelp = { examples: Array<Example>, 
/**
 * Stücke zum Einsetzen an der Schreibmarke.
 */
blocks: Array<Block>, hints: Array<Hint>, };

/**
 * Was von außen hereinkommt — ungeprüft.
 */
export type TemplateInput = { slug: string, body: string, };

export type View = { id: string, name: string, 
/**
 * `template`, `flow` oder `metro`.
 */
kind: string, 
/**
 * Kennung der Vorlage bzw. der Quelle.
 */
target: string, 
/**
 * Seiten-IDs, die nicht gezeichnet werden.
 */
hidden: Array<string>, 
/**
 * Beim Fluss: die Rolle unter dem Titel.
 */
subtitle: string | null, created_at: string, updated_at: string, };

/**
 * Was von außen hereinkommt — ungeprüft.
 */
export type ViewInput = { name: string, kind: string, target: string, hidden: Array<string>, subtitle: string | null, };

export type Theme = "system" | "light" | "dark";

export type TokenOrigin = "environment" | "store";

/**
 * Was sich über den Token sagen lässt, ohne ihn zu zeigen.
 */
export type TokenStatus = { 
/**
 * `None`: kein Token.
 */
origin: TokenOrigin | null, 
/**
 * Z. B. `ntn_…a1b2`.
 */
hint: string | null, 
/**
 * Wo die Anwendung speichert.
 */
store: string, 
/**
 * Der Speicher ließ sich nicht lesen.
 */
store_error: string | null, };

/**
 * Eine Ansicht, die sich allein aus der Zuordnung ergibt.
 */
export type ViewKind = "flow" | "metro";
