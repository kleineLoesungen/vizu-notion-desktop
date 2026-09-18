// Welche Knoten im Diagramm stehen.
//
// Zeichnet nur: Die Liste kommt aus Rust (`Diagram.nodes`, auch die
// ausgeblendeten), jede Änderung geht als Rückruf nach oben. Gefiltert wird
// dann wieder in Rust — eine zweite Filterlogik hier wäre die zweite Wahrheit.

import { useState } from "react";
import type { NodeInfo } from "../bindings";
import { formatDateTime } from "../lib/format";
import { bySource, filterableRoles, sameTitleCount, valuesOf } from "../lib/graph";

type Props = {
  nodes: NodeInfo[];
  hidden: Set<string>;
  /** Quellname → Zeitpunkt des letzten Abrufs. */
  fetched: Record<string, string | null>;
  onToggle: (id: string) => void;
  onOnlyRelated: (id: string) => void;
  onSetSource: (source: string, visible: boolean) => void;
  /** Alle Seiten dieser Quelle mit diesem Wert ein- oder ausblenden — eine ganze Gruppe. */
  onSetValue: (source: string, role: string, value: string, visible: boolean) => void;
  onReset: () => void;
};

export function FilterPanel({
  nodes,
  hidden,
  fetched,
  onToggle,
  onOnlyRelated,
  onSetSource,
  onSetValue,
  onReset,
}: Props) {
  const [search, setSearch] = useState("");
  const needle = search.trim().toLowerCase();
  const groups = bySource(nodes);

  // Gruppen im Diagramm sind Feldwerte — und zwar die einer Quelle. Je
  // Quelle eine eigene Wahl, vorgewählt die Rolle, die am ehesten Gruppen
  // bildet (status, tag, parent — in dieser Folge).
  const [chosenRoles, setChosenRoles] = useState<Record<string, string>>({});

  return (
    <aside className="filter-panel" aria-label="Knoten filtern">
      <div className="filter-head">
        <h2>Seiten im Diagramm</h2>
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
            <ValueFilter
              source={source}
              nodes={group}
              hidden={hidden}
              role={chosenRoles[source]}
              onRole={(role) => setChosenRoles({ ...chosenRoles, [source]: role })}
              onSetValue={onSetValue}
            />
            <ul className="filter-pages" aria-label={`Seiten von ${source}`}>
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

type ValueFilterProps = {
  source: string;
  /** Nur die Seiten dieser Quelle — ihre Werte, ihre Gruppen. */
  nodes: NodeInfo[];
  hidden: Set<string>;
  role: string | undefined;
  onRole: (role: string) => void;
  onSetValue: (source: string, role: string, value: string, visible: boolean) => void;
};

/**
 * „Nach Feld" für eine Quelle: die Werte einer Rolle mit Häkchen und Anzahl.
 * Ein Wert ist eine Gruppe im Diagramm — Status „Done", Tag „Web".
 */
function ValueFilter({ source, nodes, hidden, role, onRole, onSetValue }: ValueFilterProps) {
  const roles = filterableRoles(nodes);
  const current = role && roles.includes(role) ? role : roles[0];
  if (!current) return null;
  const values = valuesOf(nodes, current, hidden);

  return (
    <section className="filter-values" aria-label={`${source} nach Feld filtern`}>
      <label className="filter-role">
        <span className="muted">Gruppen nach</span>
        <select value={current} onChange={(e) => onRole(e.target.value)}>
          {roles.map((r) => (
            <option key={r} value={r}>
              {r}
            </option>
          ))}
        </select>
      </label>
      <ul>
        {values.map((v) => (
          <li key={v.value}>
            <label>
              <input
                type="checkbox"
                checked={v.visible > 0}
                // Teils zu sehen — etwa weil einzelne Seiten darunter
                // ausgeblendet sind — zeigt das Kästchen halb gefüllt.
                ref={(box) => {
                  if (box) box.indeterminate = v.visible > 0 && v.visible < v.total;
                }}
                onChange={() => onSetValue(source, current, v.value, v.visible === 0)}
              />
              <span className="filter-title">{v.value}</span>
              <span className="muted filter-count">{v.total}</span>
            </label>
          </li>
        ))}
      </ul>
    </section>
  );
}
