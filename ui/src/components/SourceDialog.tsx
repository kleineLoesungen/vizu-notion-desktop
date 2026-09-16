// Eine Quelle anlegen oder ändern.
//
// Zeichnet nur: Was gültig ist, entscheidet crates/core. Die Meldungen kommen
// von dort und stehen hier an dem Feld, zu dem sie gehören — auch bei einer
// Rolle, denn Rust benennt das Feld `mappings.<rolle>`.

import { useState } from "react";
import type { ColumnMapping, Property, Source, SourceInput } from "../bindings";
import { Dialog } from "./Dialog";
import { FieldMessage } from "./FieldMessage";

type Props = {
  /** `null`: neue Quelle. */
  source: Source | null;
  /** Spalten aus dem letzten Abruf, als Vorschlagsliste. */
  properties: Property[];
  fieldMessage: (field: string) => string | undefined;
  onSave: (input: SourceInput) => void;
  onDelete?: () => void;
  onClose: () => void;
};

/** Rollen, die Metro-Karte, Fluss und die Vorlagen der Webapp kennen. */
const KNOWN_ROLES = ["title", "date", "next", "parent", "tag", "status", "assignee"];

export function SourceDialog({
  source,
  properties,
  fieldMessage,
  onSave,
  onDelete,
  onClose,
}: Props) {
  const [name, setName] = useState(source?.name ?? "");
  const [databaseId, setDatabaseId] = useState(source?.database_id ?? "");
  const [mappings, setMappings] = useState<ColumnMapping[]>(
    source?.mappings ?? [{ role: "title", property: "" }],
  );

  function change(index: number, patch: Partial<ColumnMapping>) {
    setMappings(mappings.map((m, i) => (i === index ? { ...m, ...patch } : m)));
  }

  return (
    <Dialog title={source ? "Quelle ändern" : "Neue Quelle"} onClose={onClose}>
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
          Name
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            aria-describedby="name-error"
          />
        </label>
        <FieldMessage id="name-error" message={fieldMessage("name")} />

        <label>
          Notion-Datenbank
          <input
            value={databaseId}
            placeholder="Kennung oder Adresse aus Notion"
            onChange={(e) => setDatabaseId(e.target.value)}
            aria-describedby="database_id-error"
          />
        </label>
        <FieldMessage id="database_id-error" message={fieldMessage("database_id")} />

        <fieldset className="mapping-editor">
          <legend>Zuordnung</legend>
          <p className="muted">
            Links der Name, unter dem Vorlagen das Feld benutzen (<code>{"{{title}}"}</code>),
            rechts die Spalte in Notion.
          </p>
          {properties.length === 0 && (
            <p className="muted">
              Noch kein Abruf — die Spaltennamen müssen deshalb genau so eingetippt werden, wie sie
              in Notion heißen.
            </p>
          )}

          <datalist id="notion-properties">
            {properties.map((p) => (
              <option key={p.name} value={p.name}>
                {p.kind}
              </option>
            ))}
          </datalist>
          <datalist id="known-roles">
            {KNOWN_ROLES.map((role) => (
              <option key={role} value={role} />
            ))}
          </datalist>

          {mappings.map((mapping, index) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: die Zeilen haben keine eigene Kennung
            <div key={index} className="mapping-row">
              <input
                aria-label={`Rolle ${index + 1}`}
                list="known-roles"
                value={mapping.role}
                onChange={(e) => change(index, { role: e.target.value })}
              />
              <span aria-hidden="true">→</span>
              <input
                aria-label={`Spalte ${index + 1}`}
                list="notion-properties"
                value={mapping.property}
                onChange={(e) => change(index, { property: e.target.value })}
              />
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
            onClick={() => setMappings([...mappings, { role: "", property: "" }])}
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
