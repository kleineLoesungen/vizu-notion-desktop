-- Gespeicherte Ansichten: ein Diagramm samt dem, was man daran eingestellt hat.
--
-- Ersatz für die Teilen-Links der Webapp (docs/UMSETZUNG.md, E5): Es gibt
-- keinen Server, der einen Link auflösen könnte — also bleibt der Zustand hier
-- und wird beim Öffnen wiederhergestellt.
--
-- `kind` ist „template", „flow" oder „metro"; `target` die Vorlagen- bzw.
-- Quellenkennung. `hidden` ist eine JSON-Liste von Seiten-IDs, `subtitle` die
-- Rolle unter dem Titel im Fluss.
CREATE TABLE views (
    id         TEXT NOT NULL PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE COLLATE NOCASE,
    kind       TEXT NOT NULL,
    target     TEXT NOT NULL,
    hidden     TEXT NOT NULL,
    subtitle   TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;
