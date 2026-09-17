// Die gerade eingestellte Ansicht unter einem Namen sichern.
//
// Zeichnet nur: Ob der Name taugt, entscheidet core — die Meldung steht am
// Feld, so wie überall.

import { useState } from "react";
import { Dialog } from "./Dialog";
import { FieldMessage } from "./FieldMessage";

type Props = {
  /** Vorschlag: der Titel des Diagramms. */
  suggestion: string;
  /** Wie viele Seiten gerade ausgeblendet sind — das gehört mit zur Ansicht. */
  hiddenCount: number;
  fieldMessage: (field: string) => string | undefined;
  onSave: (name: string) => void;
  onClose: () => void;
};

export function SaveViewDialog({ suggestion, hiddenCount, fieldMessage, onSave, onClose }: Props) {
  const [name, setName] = useState(suggestion);

  return (
    <Dialog title="Ansicht speichern" onClose={onClose}>
      <form
        className="form"
        onSubmit={(e) => {
          e.preventDefault();
          onSave(name);
        }}
      >
        <p className="muted">
          Gespeichert wird, was gerade zu sehen ist: das Diagramm
          {hiddenCount > 0
            ? ` und die ${hiddenCount} ausgeblendeten Seiten.`
            : " samt seinen Einstellungen."}
        </p>
        <label>
          Name
          {/* biome-ignore lint/a11y/noAutofocus: der Dialog hat genau ein Feld */}
          <input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            aria-describedby="view-name-error"
          />
        </label>
        <FieldMessage id="view-name-error" message={fieldMessage("name")} />

        <div className="actions">
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
