// Die Vorlagen in der Seitenleiste. Zeichnet nur.

import type { SourceOverview, Template } from "../bindings";

type Props = {
  templates: Template[];
  /** Quellen, die ohne Vorlage etwas zeichnen können. */
  sources: SourceOverview[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onSelectFlow: (sourceId: string) => void;
  onCreate: () => void;
};

export function TemplateList({
  templates,
  sources,
  selectedId,
  onSelect,
  onSelectFlow,
  onCreate,
}: Props) {
  const flows = sources.filter((s) => s.views.includes("flow"));
  return (
    <nav className="template-nav" aria-label="Vorlagen">
      <div className="sidebar-head-inline">
        <h2 className="sidebar-title">Diagramme</h2>
        <button type="button" className="ghost small" onClick={onCreate}>
          Neu
        </button>
      </div>
      {templates.length === 0 && flows.length === 0 ? (
        <p className="muted sidebar-empty">
          Noch keine Vorlage. „Neu" legt eine an — oder <code>vizu-notion template import</code>
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

      {flows.length > 0 && (
        <>
          <h3 className="sidebar-title">Ohne Vorlage</h3>
          <ul className="source-list">
            {flows.map(({ source }) => (
              <li key={`flow-${source.id}`}>
                <button
                  type="button"
                  className="source-item"
                  aria-current={source.id === selectedId ? "true" : undefined}
                  onClick={() => onSelectFlow(source.id)}
                >
                  <span className="source-title">Fluss: {source.name}</span>
                  <span className="source-excerpt">entlang „next"</span>
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
    </nav>
  );
}
