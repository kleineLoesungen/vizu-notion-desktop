// Der Vorlagen-Editor: Text links, Vorschau rechts.
//
// Zeichnet nur. Die Vorschau kommt aus demselben Befehl, der auch das fertige
// Diagramm erzeugt (`diagram_preview` → `template::render_body`) — sie kann
// deshalb nicht davon abweichen.

import { useRef } from "react";
import type { Diagram, Inserted, InsertInput, TemplateHelp } from "../bindings";
import { DiagramView } from "./DiagramView";
import { FieldMessage } from "./FieldMessage";
import { type SourceFields, TemplateHelper } from "./TemplateHelper";

type Props = {
  slug: string;
  body: string;
  /** `null`, solange die Vorlage noch nicht zeichnet. */
  preview: Diagram | null;
  dirty: boolean;
  isNew: boolean;
  dark: boolean;
  fieldMessage: (field: string) => string | undefined;
  /** Die eingerichteten Quellen mit ihren Rollen — für Beispiele und Felder. */
  sources: SourceFields[];
  /** Beispiele und Spickzettel aus core. */
  help: TemplateHelp | null;
  onSlugChange: (slug: string) => void;
  onBodyChange: (body: string) => void;
  /** Lässt core einen Baustein einsetzen — mit Kopf und `sources`. */
  onAssemble: (input: InsertInput) => Promise<Inserted | null>;
  onSave: () => void;
  onDelete?: () => void;
  onClose: () => void;
};

/** Setzt die vorhandenen Quellen in ein Beispiel ein. */
function withSources(body: string, sources: string[]): string {
  return body
    .replaceAll("QUELLE", sources[0] ?? "QUELLE")
    .replaceAll("ZWEITE", sources[1] ?? sources[0] ?? "ZWEITE");
}

export function TemplateEditor({
  slug,
  body,
  preview,
  dirty,
  isNew,
  dark,
  fieldMessage,
  sources,
  help,
  onSlugChange,
  onBodyChange,
  onAssemble,
  onSave,
  onDelete,
  onClose,
}: Props) {
  const text = useRef<HTMLTextAreaElement>(null);
  const names = sources.map((s) => s.name);

  /** Die Schreibmarke nach dem Neuzeichnen setzen, damit man weiterschreiben kann. */
  function moveCursor(at: number) {
    const area = text.current;
    if (!area) return;
    requestAnimationFrame(() => {
      area.focus();
      area.setSelectionRange(at, at);
    });
  }

  /** Ein Baustein: core setzt ihn ein und ergänzt, was der Aufbau braucht. */
  async function insertBlock(snippet: string, blockSources: string[]) {
    const cursor = text.current?.selectionStart ?? body.length;
    const result = await onAssemble({ body, cursor, snippet, sources: blockSources });
    if (!result) return;
    onBodyChange(result.body);
    moveCursor(result.cursor);
  }

  /** Setzt Text an der Schreibmarke ein — oder ersetzt, was markiert ist. */
  function insert(snippet: string) {
    const area = text.current;
    if (!area) {
      onBodyChange(body + snippet);
      return;
    }
    const start = area.selectionStart;
    const end = area.selectionEnd;
    onBodyChange(body.slice(0, start) + snippet + body.slice(end));
    // Nach dem Neuzeichnen: Schreibmarke hinter das Eingefügte, damit man
    // gleich weiterschreiben oder das nächste Feld anklicken kann.
    requestAnimationFrame(() => {
      area.focus();
      area.setSelectionRange(start + snippet.length, start + snippet.length);
    });
  }

  return (
    <section className="editor">
      <header className="detail-head">
        <h1>{isNew ? "Neue Vorlage" : (preview?.title ?? slug)}</h1>
        <div className="actions">
          {onDelete && (
            <button type="button" className="ghost destructive" onClick={onDelete}>
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
          <div className="editor-row">
            <label className="grow-x">
              Kurzname
              <input
                value={slug}
                onChange={(e) => onSlugChange(e.target.value)}
                aria-describedby="slug-error"
              />
            </label>
            <label className="grow-x">
              Beispiel einsetzen
              <select
                value=""
                onChange={(e) => {
                  const example = help?.examples.find((x) => x.name === e.target.value);
                  if (example) onBodyChange(withSources(example.body, names));
                }}
              >
                <option value="">auswählen …</option>
                {(help?.examples ?? []).map((example) => (
                  <option key={example.name} value={example.name}>
                    {example.name} ({example.kind})
                  </option>
                ))}
              </select>
            </label>
          </div>
          <FieldMessage id="slug-error" message={fieldMessage("slug")} />

          <TemplateHelper
            sources={sources}
            blocks={help?.blocks ?? []}
            body={body}
            onInsert={insert}
            onInsertBlock={(snippet, used) => void insertBlock(snippet, used)}
          />

          <label className="grow">
            Vorlage
            <textarea
              ref={text}
              className="template-body"
              value={body}
              spellCheck={false}
              onChange={(e) => onBodyChange(e.target.value)}
              aria-describedby="body-error"
            />
          </label>
          <FieldMessage id="body-error" message={fieldMessage("body")} />

          <details className="cheat-sheet">
            <summary>Spickzettel</summary>
            <dl>
              {(help?.hints ?? []).map((hint) => (
                <div key={hint.syntax}>
                  <dt>
                    <code>{hint.syntax}</code>
                  </dt>
                  <dd>{hint.meaning}</dd>
                </div>
              ))}
            </dl>
            <p className="muted">
              Der Kopf zwischen <code>---</code> braucht <code>title</code> und <code>sources</code>
              ; unter <code>styles</code> stehen Form und Farbe je Feld.
            </p>
          </details>
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
