// Alter der Daten und der Knopf, sie neu zu holen — im Kopf jedes Diagramms.
//
// Zeichnet nur. Welche Quellen beteiligt sind, weiß App.tsx; hier steht, wie
// alt die Daten sind und wann der Knopf gesperrt ist.

import { formatDateTime } from "../lib/format";

type Props = {
  /** Namen der beteiligten Quellen. */
  names: string[];
  /** Quellname → Zeitpunkt des letzten Abrufs. */
  fetched: Record<string, string | null>;
  busy: boolean;
  onFetch: () => void;
};

export function SourceRefresh({ names, fetched, busy, onFetch }: Props) {
  // So alt sind die Daten: der älteste Abruf zählt. Fehlt einer ganz, ist die
  // Antwort „noch nicht abgerufen", nicht das Datum der anderen.
  let oldest: string | null | undefined;
  for (const name of names) {
    const at = fetched[name] ?? null;
    oldest = oldest === null || at === null ? null : !oldest || at < oldest ? at : oldest;
  }

  return (
    <div className="head-group">
      <span className="muted">
        {oldest ? `Daten vom ${formatDateTime(oldest)}` : "Quellen noch nicht abgerufen"}
      </span>
      <button
        type="button"
        className="ghost"
        disabled={busy || names.length === 0}
        onClick={onFetch}
      >
        {busy ? "Rufe ab …" : names.length === 1 ? "Quelle abrufen" : "Quellen abrufen"}
      </button>
    </div>
  );
}
