// Eine Quelle anlegen oder ändern.
//
// Zeichnet nur: Was gültig ist, entscheidet crates/core. Die Meldungen kommen
// von dort und stehen hier an dem Feld, zu dem sie gehören — auch bei einer
// Rolle, denn Rust benennt das Feld `mappings.<rolle>`.
//
// Die Spalten kommen aus Notion: entweder aus dem letzten Abruf (`properties`)
// oder frisch geholt (`schema`). Sobald sie bekannt sind, wählt man sie aus
// einer Liste, statt sie abzutippen — und core schlägt die ganze Zuordnung
// vor (`source::suggest`), damit niemand raten muss.

import { useEffect, useState } from "react";
import type { ColumnMapping, DatabaseSchema, Property, Source, SourceInput } from "../bindings";
import { fillFromSuggestion } from "../lib/mapping";
import { Dialog } from "./Dialog";
import { FieldMessage } from "./FieldMessage";

type Props = {
  /** `null`: neue Quelle. */
  source: Source | null;
  /** Spalten aus dem letzten Abruf. Leer, solange nie abgerufen wurde. */
  properties: Property[];
  /** Frisch von Notion geholt, samt Vorschlag. `null`: noch nicht geholt. */
  schema: DatabaseSchema | null;
  inspecting: boolean;
  fieldMessage: (field: string) => string | undefined;
  /** Lässt App.tsx die Spalten dieser Datenbank holen. */
  onInspect: (databaseId: string) => void;
  onSave: (input: SourceInput) => void;
  onDelete?: () => void;
  onClose: () => void;
};

/** Rollen, die Metro-Karte, Fluss und die Vorlagen der Webapp kennen. */
const KNOWN_ROLES: [string, string][] = [
  ["title", "Titel der Seite"],
  ["date", "Datum — Zeitachse der Metro-Karte"],
  ["next", "Nachfolger — Linien und Pfeile"],
  ["parent", "übergeordnete Seite"],
  ["tag", "Schlagwort — Bänder der Metro-Karte"],
  ["status", "Status"],
  ["assignee", "zuständige Person"],
];

const EMPTY_ROW: ColumnMapping = { role: "", property: "" };

export function SourceDialog({
  source,
  properties,
  schema,
  inspecting,
  fieldMessage,
  onInspect,
  onSave,
  onDelete,
  onClose,
}: Props) {
  const [name, setName] = useState(source?.name ?? "");
  const [databaseId, setDatabaseId] = useState(source?.database_id ?? "");
  const [mappings, setMappings] = useState<ColumnMapping[]>(
    source?.mappings ?? [{ role: "title", property: "" }],
  );

  // Frisch geholte Spalten sind die besseren: Sie sind von jetzt, die aus dem
  // Abruf können alt sein.
  const columns = schema?.properties ?? properties;

  // Was gerade geholt wurde, arbeitet sich von selbst ein: leerer Name und
  // leere Spalten werden gefüllt, fehlende Rollen kommen dazu. Überschrieben
  // wird nichts, was schon eingetragen ist — deshalb braucht es keinen Knopf.
  useEffect(() => {
    if (!schema) return;
    setName((current) => current || schema.title);
    setMappings((current) => fillFromSuggestion(current, schema.suggestion));
  }, [schema]);

  function change(index: number, patch: Partial<ColumnMapping>) {
    setMappings(mappings.map((m, i) => (i === index ? { ...m, ...patch } : m)));
  }

  return (
    <Dialog title={source ? "Quelle ändern" : "Neue Quelle"} onClose={onClose} wide>
      <form
        className="form"
        onSubmit={(e) => {
          e.preventDefault();
          // Leere Zeilen sind Bedienung, kein Fehler — sie fliegen hier raus.
          onSave({
            name,
            database_id: databaseId,
            mappings: mappings.filter((m) => m.role.trim() !== "" || m.property.trim() !== ""),
          });
        }}
      >
        <label>
          Link zur Notion-Datenbank
          <input
            value={databaseId}
            placeholder="https://app.notion.com/p/…"
            onChange={(e) => setDatabaseId(e.target.value)}
            // Wer einen Link einfügt, will die Spalten sehen — der Klick auf
            // „Spalten holen" wäre nur ein Schritt mehr. Beim Tippen nicht:
            // Da wäre jeder Zwischenstand eine Anfrage an Notion.
            onPaste={(e) => {
              const pasted = e.clipboardData.getData("text").trim();
              if (pasted !== "" && !inspecting) onInspect(pasted);
            }}
            aria-describedby="database_id-hint database_id-error"
          />
        </label>
        <p id="database_id-hint" className="muted field-hint">
          In Notion die Datenbank öffnen, oben rechts ••• → „Link kopieren“ und hier einfügen. Die
          Kennung allein geht auch.
        </p>
        <div className="row">
          <button
            type="button"
            className="ghost"
            disabled={inspecting || databaseId.trim() === ""}
            onClick={() => onInspect(databaseId)}
          >
            {inspecting ? "Frage Notion …" : "Spalten holen"}
          </button>
          {schema && (
            <span className="muted">
              „{schema.title}“ — {schema.properties.length} Spalten
            </span>
          )}
        </div>
        <FieldMessage id="database_id-error" message={fieldMessage("database_id")} />

        {/* Nach dem Link: Den Namen schlägt die Datenbank selbst vor. */}
        <label>
          Name
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            aria-describedby="name-error"
          />
        </label>
        <FieldMessage id="name-error" message={fieldMessage("name")} />

        <fieldset className="mapping-editor">
          <legend>Zuordnung</legend>
          <p className="muted">
            Links der Name, unter dem Vorlagen das Feld benutzen (<code>{"{{title}}"}</code>),
            rechts die Spalte in Notion. Eine Notion-Spalte „Nächstes“ gehört also nach rechts, die
            Rolle <code>next</code> nach links.
          </p>
          {columns.length === 0 && (
            <p className="muted">
              Noch keine Spalten bekannt. „Spalten holen“ fragt Notion — sonst müssen die Namen
              genau so eingetippt werden, wie sie dort heißen.
            </p>
          )}

          <datalist id="known-roles">
            {KNOWN_ROLES.map(([role, hint]) => (
              <option key={role} value={role}>
                {hint}
              </option>
            ))}
          </datalist>

          {/* Die beiden Felder sehen gleich aus; ohne Überschrift ist nicht zu
              sehen, welche Seite welche ist. */}
          <div className="mapping-row mapping-head" aria-hidden="true">
            <span className="muted">Rolle</span>
            <span />
            <span className="muted">Spalte in Notion</span>
          </div>

          {mappings.map((mapping, index) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: die Zeilen haben keine eigene Kennung
            <div key={index} className="mapping-row">
              <input
                aria-label={`Rolle ${index + 1}`}
                list="known-roles"
                placeholder="next"
                value={mapping.role}
                onChange={(e) => change(index, { role: e.target.value })}
              />
              <span aria-hidden="true">→</span>
              {columns.length > 0 ? (
                <select
                  aria-label={`Spalte ${index + 1}`}
                  value={mapping.property}
                  onChange={(e) => change(index, { property: e.target.value })}
                >
                  <option value="">— Spalte wählen —</option>
                  {/* Eine Spalte, die es in Notion nicht mehr gibt, bleibt
                      trotzdem sichtbar — sonst verschwände sie stillschweigend
                      aus der Zuordnung. */}
                  {mapping.property !== "" && !columns.some((p) => p.name === mapping.property) && (
                    <option value={mapping.property}>{mapping.property} (unbekannt)</option>
                  )}
                  {columns.map((p) => (
                    <option key={p.name} value={p.name}>
                      {p.name} ({p.kind})
                    </option>
                  ))}
                </select>
              ) : (
                <input
                  aria-label={`Spalte ${index + 1}`}
                  placeholder="Nächstes"
                  value={mapping.property}
                  onChange={(e) => change(index, { property: e.target.value })}
                />
              )}
              <button
                type="button"
                className="ghost small"
                aria-label={`Zeile ${index + 1} entfernen`}
                onClick={() => setMappings(mappings.filter((_, i) => i !== index))}
              >
                ×
              </button>
              <FieldMessage
                id={`mappings.${mapping.role}-error`}
                message={fieldMessage(`mappings.${mapping.role}`)}
              />
            </div>
          ))}
          <FieldMessage id="mappings-error" message={fieldMessage("mappings")} />
          <button
            type="button"
            className="ghost"
            onClick={() => setMappings([...mappings, EMPTY_ROW])}
          >
            Zeile hinzufügen
          </button>
        </fieldset>

        <div className="actions">
          {onDelete && (
            <button type="button" className="ghost destructive" onClick={onDelete}>
              Löschen
            </button>
          )}
          <button type="button" className="ghost" onClick={onClose}>
            Abbrechen
          </button>
          <button type="submit" className="primary">
            Speichern
          </button>
        </div>
      </form>
    </Dialog>
  );
}
