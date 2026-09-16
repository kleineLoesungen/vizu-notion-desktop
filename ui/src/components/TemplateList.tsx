// Die Vorlagen in der Seitenleiste. Zeichnet nur.

import type { Template } from "../bindings";

type Props = {
  templates: Template[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onCreate: () => void;
};

export function TemplateList({ templates, selectedId, onSelect, onCreate }: Props) {
  return (
    <nav className="template-nav" aria-label="Vorlagen">
      <div className="sidebar-head-inline">
        <h2 className="sidebar-title">Diagramme</h2>
        <button type="button" className="ghost small" onClick={onCreate}>
          Neu
        </button>
      </div>
      {templates.length === 0 ? (
        <p className="muted sidebar-empty">
          Noch keine Vorlage. Einlesen mit <code>vizu-notion template import</code>
        </p>
      ) : (
        <ul className="source-list">
          {templates.map((template) => (
            <li key={template.id}>
              <button
                type="button"
                className="source-item"
                aria-current={template.id === selectedId ? "true" : undefined}
                onClick={() => onSelect(template.id)}
              >
                <span className="source-title">{template.title}</span>
                <span className="source-excerpt">{template.sources.join(", ")}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </nav>
  );
}
