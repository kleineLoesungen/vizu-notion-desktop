// Die Liste links. Zeichnet nur, meldet Auswahl und Wünsche nach oben.

import type { SourceOverview } from "../bindings";
import { formatDateTime } from "../lib/format";

type Props = {
  sources: SourceOverview[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onFetchAll: () => void;
  onCreate: () => void;
  busy: boolean;
};

export function SourceList({ sources, selectedId, onSelect, onFetchAll, onCreate, busy }: Props) {
  return (
    <nav className="source-nav" aria-label="Quellen">
      <div className="sidebar-head">
        <button type="button" className="primary" onClick={onCreate}>
          Neue Quelle
        </button>
        <button
          type="button"
          className="ghost"
          disabled={busy || sources.length === 0}
          onClick={onFetchAll}
        >
          {busy ? "Rufe ab …" : "Alle abrufen"}
        </button>
      </div>

      {sources.length === 0 ? (
        <p className="muted sidebar-empty">Noch keine Quellen.</p>
      ) : (
        <ul className="source-list">
          {sources.map(({ source, fetch }) => (
            <li key={source.id}>
              <button
                type="button"
                className="source-item"
                aria-current={source.id === selectedId ? "true" : undefined}
                onClick={() => onSelect(source.id)}
              >
                <span className="source-title">{source.name}</span>
                <span className="source-excerpt">
                  {fetch ? `${fetch.page_count} Seiten` : "noch nie abgerufen"}
                </span>
                <span className="source-date">
                  {fetch ? formatDateTime(fetch.fetched_at) : "—"}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </nav>
  );
}
