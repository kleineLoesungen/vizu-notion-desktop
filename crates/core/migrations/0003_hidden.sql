-- Diagramme, die in der Liste nicht auftauchen sollen.
--
-- Fluss und Metro entstehen von selbst, je Quelle eines — wer sie nicht
-- braucht, blendet sie aus. Kein Löschen: Die Vorlage bzw. die Quelle bleibt,
-- nur der Eintrag in der Liste rutscht in den Bereich „Versteckt".
--
-- `kind` ist „template", „flow" oder „metro"; `target` die Vorlagen- bzw.
-- Quellenkennung. Ohne Fremdschlüssel: Wird eine Vorlage gelöscht, bleibt hier
-- eine Zeile stehen, die niemanden stört — sie zeigt auf nichts mehr.
CREATE TABLE hidden_diagrams (
    kind      TEXT NOT NULL,
    target    TEXT NOT NULL,
    hidden_at TEXT NOT NULL,
    PRIMARY KEY (kind, target)
) STRICT;
