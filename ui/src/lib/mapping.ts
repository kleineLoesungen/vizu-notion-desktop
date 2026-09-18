// Den Vorschlag aus core in eine Zuordnung einarbeiten, ohne etwas zu
// überschreiben, das jemand selbst eingetragen hat.
//
// Dieselbe Regel wie `vizu-notion source add --auto` auf der Kommandozeile
// (crates/cli/src/commands/source.rs): Was von Hand kam, hat Vorrang; der
// Vorschlag ergänzt nur. Deshalb braucht es keinen Knopf „Vorschlag
// übernehmen" — der Vorschlag kann immer von selbst kommen.

import type { ColumnMapping } from "../bindings";

/**
 * * Eine Rolle mit eingetragener Spalte bleibt, wie sie ist.
 * * Eine Rolle ohne Spalte bekommt die vorgeschlagene.
 * * Eine Rolle, die noch fehlt, kommt hinten dazu — außer ihre Spalte steht
 *   schon unter einer anderen Rolle. Wer „Start" selbst als `date`
 *   eingetragen hat, will sie nicht zusätzlich als `start` sehen.
 *
 * Die Reihenfolge der vorhandenen Zeilen bleibt — wer gerade in einer Zeile
 * tippt, soll sie nicht wandern sehen.
 */
export function fillFromSuggestion(
  current: ColumnMapping[],
  suggestion: ColumnMapping[],
): ColumnMapping[] {
  const suggested = new Map(suggestion.map((m) => [m.role, m.property]));
  const filled = current.map((row) =>
    row.property.trim() === "" && suggested.has(row.role)
      ? { ...row, property: suggested.get(row.role) ?? "" }
      : row,
  );
  const roles = new Set(filled.map((row) => row.role));
  const columns = new Set(filled.map((row) => row.property).filter((p) => p.trim() !== ""));
  const missing = suggestion.filter((m) => !roles.has(m.role) && !columns.has(m.property));
  return [...filled, ...missing];
}
