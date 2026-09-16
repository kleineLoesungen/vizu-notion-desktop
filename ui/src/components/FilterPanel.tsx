// Welche Knoten im Diagramm stehen.
//
// Zeichnet nur: Die Liste kommt aus Rust (`Diagram.nodes`, auch die
// ausgeblendeten), jede Änderung geht als Rückruf nach oben. Gefiltert wird
// dann wieder in Rust — eine zweite Filterlogik hier wäre die zweite Wahrheit.

import { useState } from "react";
import type { NodeInfo } from "../bindings";
import { formatDateTime } from "../lib/format";
import { bySource, sameTitleCount } from "../lib/graph";

type Props = {
  nodes: NodeInfo[];
  hidden: Set<string>;
  /** Quellname → Zeitpunkt des letzten Abrufs. */
  fetched: Record<string, string | null>;
  onToggle: (id: string) => void;
  onOnlyRelated: (id: string) => void;
  onSetSource: (source: string, visible: boolean) => void;
  onReset: () => void;
};

export function FilterPanel({
  nodes,
  hidden,
  fetched,
  onToggle,
  onOnlyRelated,
  onSetSource,
  onReset,
}: Props) {
  const [search, setSearch] = useState("");
  const needle = search.trim().toLowerCase();
  const groups = bySource(nodes);

  return (
    <aside className="filter-panel" aria-label="Knoten filtern">
      <div className="filter-head">
        <h2>Knoten</h2>
        <button type="button" className="ghost" disabled={hidden.size === 0} onClick={onReset}>
          Alle zeigen
        </button>
      </div>

      <input
        type="search"
        aria-label="Knoten suchen"
        placeholder="Suchen …"
        value={search}
        onChange={(e) => setSearch(e.target.value)}
      />

      {groups.map(([source, group]) => {
        const shown = group.filter((n) => !hidden.has(n.id)).length;
        // Die Suche filtert nur die Anzeige im Feld, nicht das Diagramm.
        const visible = needle
          ? group.filter((n) => n.title.toLowerCase().includes(needle))
          : group;
        const at = fetched[source];
        return (
          <details key={source} className="filter-group" open>
            <summary>
              <span className="filter-source">{source}</span>
              <span className="muted">
                {shown}/{group.length}
              </span>
            </summary>
            <div className="filter-group-head">
              <span className="muted">
                {at ? `abgerufen ${formatDateTime(at)}` : "noch nie abgerufen"}
              </span>
              <button
                type="button"
                className="ghost small"
                onClick={() => onSetSource(source, shown < group.length)}
              >
                {shown < group.length ? "Alle" : "Keine"}
              </button>
            </div>
            <ul>
              {visible.map((node) => {
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
              {visible.length === 0 && <li className="muted">Kein Knoten passt zur Suche.</li>}
            </ul>
          </details>
        );
      })}
    </aside>
  );
}
