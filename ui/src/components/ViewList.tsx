// Gespeicherte Ansichten in der Seitenleiste.
//
// Eine Ansicht ist ein Diagramm samt dem, was daran eingestellt war:
// ausgeblendete Seiten und — beim Fluss — die zweite Zeile. Sie ersetzt die
// Teilen-Links der Webapp, für die es hier keinen Server gibt.
//
// Zeichnet nur. Wiederhergestellt wird in App.tsx, gespeichert in core.

import type { View } from "../bindings";

type Props = {
  views: View[];
  /** Die gerade geöffnete Ansicht, falls eine geöffnet wurde. */
  activeId: string | null;
  onOpen: (view: View) => void;
  onDelete: (view: View) => void;
};

const KIND_LABEL: Record<string, string> = {
  template: "Mermaid",
  flow: "Fluss",
  metro: "Metro",
};

export function ViewList({ views, activeId, onOpen, onDelete }: Props) {
  if (views.length === 0) return null;

  return (
    <details className="views-section">
      <summary>
        Ansichten<span className="muted"> {views.length}</span>
      </summary>
      <ul className="source-list">
        {views.map((view) => (
          <li key={view.id} className="diagram-item">
            <button
              type="button"
              className="source-item"
              aria-label={`Ansicht ${view.name}`}
              aria-current={activeId === view.id ? "true" : undefined}
              onClick={() => onOpen(view)}
            >
              <span className="source-title">{view.name}</span>
              <span className="source-excerpt">
                {KIND_LABEL[view.kind] ?? view.kind}
                {view.hidden.length > 0 && ` · ${view.hidden.length} ausgeblendet`}
              </span>
            </button>
            <button
              type="button"
              className="ghost small"
              aria-label={`Ansicht ${view.name} löschen`}
              onClick={() => onDelete(view)}
            >
              Löschen
            </button>
          </li>
        ))}
      </ul>
    </details>
  );
}
