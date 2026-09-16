// Eine Quelle: Zuordnung, Stand des letzten Abrufs, Verweis nach Notion.
//
// Zeichnet nur. Abrufen und Öffnen sind Rückrufe nach App.tsx.

import type { MouseEvent } from "react";
import type { SourceOverview } from "../bindings";
import { formatDateTime } from "../lib/format";

type Props = {
  entry: SourceOverview;
  busy: boolean;
  onFetch: () => void;
  onEdit: () => void;
  onOpenLink: (url: string) => void;
};

export function SourceDetail({ entry, busy, onFetch, onEdit, onOpenLink }: Props) {
  const { source, fetch } = entry;

  function open(event: MouseEvent, url: string) {
    // Ein <a href> würde das Fenster der Anwendung wegnavigieren.
    event.preventDefault();
    onOpenLink(url);
  }

  return (
    <section className="source-detail">
      <header className="detail-head">
        <h1>{source.name}</h1>
        <div className="actions">
          <button type="button" className="ghost" onClick={onEdit}>
            Bearbeiten
          </button>
          <button type="button" className="primary" disabled={busy} onClick={onFetch}>
            {busy ? "Rufe ab …" : "Von Notion abrufen"}
          </button>
        </div>
      </header>

      <dl className="detail-facts">
        <dt>Datenbank</dt>
        <dd>
          <code>{source.database_id}</code>
          {fetch && (
            <>
              {" — "}
              <a href={fetch.database_url} onClick={(e) => open(e, fetch.database_url)}>
                {fetch.database_title} in Notion öffnen
              </a>
            </>
          )}
        </dd>

        <dt>Abgerufen</dt>
        <dd>
          {fetch
            ? `${formatDateTime(fetch.fetched_at)} — ${fetch.page_count} Seiten in ${fetch.request_count} Anfragen`
            : "noch nie"}
        </dd>
      </dl>

      <h2>Zuordnung</h2>
      {source.mappings.length === 0 ? (
        <p className="muted">Keine Spalte zugeordnet — „Bearbeiten“ legt sie an.</p>
      ) : (
        <table className="mapping-table">
          <thead>
            <tr>
              <th>Rolle</th>
              <th>Spalte in Notion</th>
            </tr>
          </thead>
          <tbody>
            {source.mappings.map((m) => (
              <tr key={m.role}>
                <td>
                  <code>{m.role}</code>
                </td>
                <td>{m.property}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
