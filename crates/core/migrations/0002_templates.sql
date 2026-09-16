-- Vorlagen und die Titel fremder Seiten.

-- Eine Mermaid-Vorlage: der ganze Text einer `.mmd`-Datei, samt Frontmatter.
-- `slug` ist der Dateiname ohne Endung und das, was `vizu-notion render`
-- entgegennimmt.
CREATE TABLE templates (
    id         TEXT NOT NULL PRIMARY KEY,
    slug       TEXT NOT NULL UNIQUE COLLATE NOCASE,
    body       TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- Titel von Seiten, auf die eine Relation zeigt, die aber in keiner Quelle
-- liegen. Ohne sie stünde in einem Diagramm die Seiten-ID statt des Namens.
-- Wird beim Abruf gefüllt, damit das Zeichnen ohne Netz auskommt.
CREATE TABLE page_titles (
    page_id    TEXT NOT NULL PRIMARY KEY,
    title      TEXT NOT NULL,
    fetched_at TEXT NOT NULL
) STRICT;
