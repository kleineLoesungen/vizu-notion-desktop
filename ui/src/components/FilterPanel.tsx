// Welche Knoten im Diagramm stehen.
//
// Zeichnet nur: Die Liste kommt aus Rust (`Diagram.nodes`, auch die
// ausgeblendeten), jede Änderung geht als Rückruf nach oben. Gefiltert wird
// dann wieder in Rust — eine zweite Filterlogik hier wäre die zweite Wahrheit.

import type { NodeInfo } from "../bindings";
import { bySource, sameTitleCount } from "../lib/graph";

type Props = {
  nodes: NodeInfo[];
  hidden: Set<string>;
  onToggle: (id: string) => void;
  onOnlyRelated: (id: string) => void;
  onSetSource: (source: string, visible: boolean) => void;
  onReset: () => void;
};

export function FilterPanel({
  nodes,
  hidden,
  onToggle,
  onOnlyRelated,
  onSetSource,
  onReset,
}: Props) {
  const groups = bySource(nodes);

  return (
    <aside className="filter-panel" aria-label="Knoten filtern">
      <div className="filter-head">
        <h2>Knoten</h2>
        <button type="button" className="ghost" disabled={hidden.size === 0} onClick={onReset}>
          Alle zeigen
        </button>
      </div>

      {groups.map(([source, group]) => {
        const shown = group.filter((n) => !hidden.has(n.id)).length;
        return (
          <section key={source} className="filter-group">
            <header>
              <span className="filter-source">{source}</span>
              <span className="muted">
                {shown}/{group.length}
              </span>
              <button
                type="button"
                className="ghost small"
                onClick={() => onSetSource(source, shown < group.length)}
              >
                {shown < group.length ? "Alle" : "Keine"}
              </button>
            </header>
            <ul>
              {group.map((node) => {
                const shared = sameTitleCount(group, node);
                return (
                  <li key={node.id}>
                    <label>
                      <input
                        type="checkbox"
                        checked={!hidden.has(node.id)}
                        onChange={() => onToggle(node.id)}
                      />
                      <span className="filter-title">{node.title || "ohne Titel"}</span>
                      {shared > 1 && (
                        <span
                          className="filter-shared"
                          title={`${shared} Seiten heißen gleich und teilen sich einen Knoten im Diagramm`}
                        >
                          ×{shared}
                        </span>
                      )}
                    </label>
                    <button
                      type="button"
                      className="ghost small"
                      title="Nur diesen Knoten und seine direkten Nachbarn zeigen"
                      aria-label={`Verwandte von ${node.title || "ohne Titel"} zeigen`}
                      onClick={() => onOnlyRelated(node.id)}
                    >
                      ⌖
                    </button>
                  </li>
                );
              })}
            </ul>
          </section>
        );
      })}
    </aside>
  );
}
