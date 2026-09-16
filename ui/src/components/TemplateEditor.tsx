// Der Vorlagen-Editor: Text links, Vorschau rechts.
//
// Zeichnet nur. Die Vorschau kommt aus demselben Befehl, der auch das fertige
// Diagramm erzeugt (`diagram_preview` → `template::render_body`) — sie kann
// deshalb nicht davon abweichen.

import type { Diagram } from "../bindings";
import { DiagramView } from "./DiagramView";
import { FieldMessage } from "./FieldMessage";

type Props = {
  slug: string;
  body: string;
  /** `null`, solange die Vorlage noch nicht zeichnet. */
  preview: Diagram | null;
  dirty: boolean;
  isNew: boolean;
  dark: boolean;
  fieldMessage: (field: string) => string | undefined;
  onSlugChange: (slug: string) => void;
  onBodyChange: (body: string) => void;
  onSave: () => void;
  onDelete?: () => void;
  onClose: () => void;
};

export function TemplateEditor({
  slug,
  body,
  preview,
  dirty,
  isNew,
  dark,
  fieldMessage,
  onSlugChange,
  onBodyChange,
  onSave,
  onDelete,
  onClose,
}: Props) {
  return (
    <section className="editor">
      <header className="detail-head">
        <h1>{isNew ? "Neue Vorlage" : (preview?.title ?? slug)}</h1>
        <div className="actions">
          {onDelete && (
            <button type="button" className="ghost danger" onClick={onDelete}>
              Löschen
            </button>
          )}
          <button type="button" className="ghost" onClick={onClose}>
            Schließen
          </button>
          <button type="button" className="primary" disabled={!dirty} onClick={onSave}>
            Speichern
          </button>
        </div>
      </header>

      <div className="editor-panes">
        <div className="editor-text">
          <label>
            Kurzname
            <input
              value={slug}
              onChange={(e) => onSlugChange(e.target.value)}
              aria-describedby="slug-error"
            />
          </label>
          <FieldMessage id="slug-error" message={fieldMessage("slug")} />

          <label className="grow">
            Vorlage
            <textarea
              className="template-body"
              value={body}
              spellCheck={false}
              onChange={(e) => onBodyChange(e.target.value)}
              aria-describedby="body-error"
            />
          </label>
          <FieldMessage id="body-error" message={fieldMessage("body")} />
        </div>

        <div className="editor-preview">
          {preview ? (
            <DiagramView mermaid={preview.mermaid} dark={dark} />
          ) : (
            <p className="muted">Keine Vorschau — siehe Meldung am Textfeld.</p>
          )}
        </div>
      </div>
    </section>
  );
}
