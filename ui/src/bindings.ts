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
 * Fenstertitel und Name in der Oberfläche.
 */
app_name: string, 
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
 * Eine Meldung, die zu genau einem Eingabefeld gehört.
 *
 * `field` ist englisch und entspricht dem Feldnamen im Modell (`name`,
 * `database_id`). Bei Listen steht der Eintrag dabei: `mappings.next` für
 * die Rolle `next`, `sources[2].name` beim Import. `message` ist deutsch und
 * für Menschen.
 */
export type FieldError = { field: string, message: string, };

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
fetch: FetchStatus | null, };

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
