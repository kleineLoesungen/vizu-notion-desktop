-- Quellen, ihre Spaltenzuordnung und der Zwischenspeicher der Notion-Daten.
--
-- STRICT erzwingt die Spaltentypen. Ohne STRICT nimmt SQLite in einer
-- TEXT-Spalte klaglos eine Zahl entgegen.

-- Eine Notion-Datenbank unter einem Namen, den Vorlagen benutzen
-- ({{#each Name}}). Der Name ist ohne Rücksicht auf Groß- und Kleinschreibung
-- eindeutig — „Projekte" und „projekte" nebeneinander wären eine Falle.
CREATE TABLE sources (
    id          TEXT NOT NULL PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE COLLATE NOCASE,
    database_id TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
) STRICT;

-- Rolle (title, next, parent …) → Name der Spalte in Notion.
CREATE TABLE source_mappings (
    source_id TEXT NOT NULL REFERENCES sources (id) ON DELETE CASCADE,
    role      TEXT NOT NULL,
    property  TEXT NOT NULL,
    PRIMARY KEY (source_id, role)
) STRICT;

-- Der letzte erfolgreiche Abruf je Quelle. Fehlt die Zeile, wurde noch nie
-- abgerufen. `schema` ist die Antwort von GET /data_sources/{id} als JSON.
CREATE TABLE fetches (
    source_id      TEXT NOT NULL PRIMARY KEY REFERENCES sources (id) ON DELETE CASCADE,
    database_title TEXT NOT NULL,
    database_url   TEXT NOT NULL,
    data_source_id TEXT NOT NULL,
    schema         TEXT NOT NULL,
    page_count     INTEGER NOT NULL,
    request_count  INTEGER NOT NULL,
    fetched_at     TEXT NOT NULL
) STRICT;

-- Die Seiten des letzten Abrufs, so wie Notion sie liefert. `properties` ist
-- das JSON-Objekt der Seite; Relationen sind darin bereits vollständig
-- nachgeladen (has_more). `position` hält die Reihenfolge der Abfrage.
--
-- `title` steht doppelt — im JSON und als eigene Spalte. So findet eine
-- Relation den Namen ihres Ziels, ohne dass dessen ganze Quelle gelesen
-- werden muss.
CREATE TABLE pages (
    source_id        TEXT NOT NULL REFERENCES sources (id) ON DELETE CASCADE,
    page_id          TEXT NOT NULL,
    position         INTEGER NOT NULL,
    title            TEXT NOT NULL,
    url              TEXT NOT NULL,
    last_edited_time TEXT NOT NULL,
    properties       TEXT NOT NULL,
    PRIMARY KEY (source_id, page_id)
) STRICT;

CREATE INDEX pages_position ON pages (source_id, position);
