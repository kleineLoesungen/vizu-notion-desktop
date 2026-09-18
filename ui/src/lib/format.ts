// Anzeige von Werten, die Rust in maschinenlesbarer Form schickt.
//
// Zeitstempel kommen als RFC-3339-Text in UTC (siehe crates/core/src/timestamp.rs).
// Umgerechnet wird erst hier, für die Anzeige — nie zurück nach Rust.

const dateTime = new Intl.DateTimeFormat("de-DE", {
  day: "2-digit",
  month: "2-digit",
  year: "numeric",
  hour: "2-digit",
  minute: "2-digit",
});

/** `2026-09-15T06:39:51Z` → `15.09.2026, 08:39` in der Zeitzone des Rechners. */
export function formatDateTime(rfc3339: string): string {
  const date = new Date(rfc3339);
  return Number.isNaN(date.getTime()) ? rfc3339 : dateTime.format(date);
}

/** Eine Zeile aus dem erzeugten Mermaid-Text, für die Fehlermeldung. */
export type SourceLine = { number: number; text: string; failing: boolean };

/**
 * Die Zeile, an der Mermaid gescheitert ist, mit einer Zeile davor und danach.
 *
 * Mermaid meldet „Parse error on line 2" — gemeint ist der **erzeugte** Text,
 * nicht die Vorlage, und den sieht man sonst nicht. `null`, wenn die Meldung
 * keine Zeile nennt.
 */
export function failingLines(text: string, message: string): SourceLine[] | null {
  const match = /line (\d+)/i.exec(message);
  if (!match) return null;
  const failing = Number(match[1]);
  const lines = text.split("\n");
  if (failing < 1 || failing > lines.length) return null;
  const out: SourceLine[] = [];
  for (let n = Math.max(1, failing - 1); n <= Math.min(lines.length, failing + 1); n++) {
    out.push({ number: n, text: lines[n - 1] ?? "", failing: n === failing });
  }
  return out;
}
