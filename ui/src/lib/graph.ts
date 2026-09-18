// Nachbarschaft im Knotengraphen.
//
// Die Relationen kommen aus Rust (`NodeInfo.relations`): je Seite die
// Seiten-IDs, auf die sie zeigt. Wer „verwandte Knoten" sucht, will auch die
// Gegenrichtung — deshalb wird hier beides betrachtet.

import type { NodeInfo } from "../bindings";

/**
 * Ein Knoten und alles, was einen Schritt entfernt liegt — in beide Richtungen.
 *
 * Ein Schritt, nicht mehr: Bei zwei Schritten hängt in einer verbundenen
 * Datenbank schnell alles mit allem zusammen, und die Auswahl hilft nicht mehr.
 */
export function withNeighbours(nodes: NodeInfo[], id: string): Set<string> {
  const keep = new Set<string>([id]);
  const node = nodes.find((n) => n.id === id);
  for (const target of node?.relations ?? []) keep.add(target);
  for (const other of nodes) {
    if (other.relations.includes(id)) keep.add(other.id);
  }
  return keep;
}

/** Knoten nach Quelle, in der Reihenfolge ihres ersten Auftretens. */
export function bySource(nodes: NodeInfo[]): [string, NodeInfo[]][] {
  const groups = new Map<string, NodeInfo[]>();
  for (const node of nodes) {
    const list = groups.get(node.source);
    if (list) {
      list.push(node);
    } else {
      groups.set(node.source, [node]);
    }
  }
  return [...groups];
}

/**
 * Wie viele Knoten derselben Quelle denselben Titel tragen.
 *
 * Die Knotenkennung im Diagramm entsteht aus dem **Wert**, nicht aus der
 * Seiten-ID — so war es schon in der Webapp. Gleichnamige Seiten werden im
 * Diagramm deshalb zu einem Knoten. Wer eine davon ausblendet, sieht zunächst
 * nichts, weil die anderen den Knoten weiter zeichnen; die Zahl im Filterfeld
 * sagt, woran das liegt.
 */
export function sameTitleCount(group: NodeInfo[], node: NodeInfo): number {
  return group.filter((other) => other.title === node.title).length;
}

// --- Nach Feldwert filtern ------------------------------------------------------
//
// Eine Gruppe im Diagramm ist ein Feldwert: Status „Done", Tag „Web", Ziel
// „Wachstum". Die Werte je Seite kommen aus Rust (`NodeInfo.fields`); hier wird
// nur gerechnet, welche Seiten-IDs dafür aus- oder eingeblendet werden. Gezeichnet
// wird weiter in Rust, und eine gespeicherte Ansicht merkt es sich von selbst.

/** Wie eine leere Spalte im Filterfeld heißt. */
export const EMPTY_VALUE = "(leer)";

/** Die Rollen, nach denen sich filtern lässt — die Gruppenbildner zuerst. */
export function filterableRoles(nodes: NodeInfo[]): string[] {
  const roles = new Set<string>();
  for (const node of nodes) {
    for (const [role, values] of Object.entries(node.fields)) {
      // Nach dem Titel filtert die Liste darunter schon, Seite für Seite.
      if (role !== "title" && values && values.length > 0) roles.add(role);
    }
  }
  const first = ["status", "tag", "parent"];
  return [...roles].sort((a, b) => {
    const ia = first.indexOf(a);
    const ib = first.indexOf(b);
    if (ia !== ib) return (ia === -1 ? 99 : ia) - (ib === -1 ? 99 : ib);
    return a.localeCompare(b);
  });
}

/** Die Werte einer Seite für eine Rolle; eine leere Spalte zählt als eigener Wert. */
function valuesFor(node: NodeInfo, role: string): string[] | undefined {
  const values = node.fields[role];
  // Eine Seite aus einer Quelle ohne diese Rolle hat hier nichts zu sagen.
  if (values === undefined) return undefined;
  return values.length > 0 ? values : [EMPTY_VALUE];
}

export type ValueCount = { value: string; total: number; visible: number };

/** Jeder Wert einer Rolle: wie viele Seiten ihn tragen und wie viele davon zu sehen sind. */
export function valuesOf(nodes: NodeInfo[], role: string, hidden: Set<string>): ValueCount[] {
  const counts = new Map<string, ValueCount>();
  for (const node of nodes) {
    for (const value of valuesFor(node, role) ?? []) {
      const entry = counts.get(value) ?? { value, total: 0, visible: 0 };
      entry.total += 1;
      if (!hidden.has(node.id)) entry.visible += 1;
      counts.set(value, entry);
    }
  }
  // Der leere Wert zuletzt, sonst alphabetisch.
  return [...counts.values()].sort((a, b) =>
    a.value === EMPTY_VALUE ? 1 : b.value === EMPTY_VALUE ? -1 : a.value.localeCompare(b.value),
  );
}

/**
 * Die neue Menge ausgeblendeter Seiten, wenn ein Wert ein- oder ausgeblendet wird.
 *
 * Ausblenden trifft **jede** Seite, die den Wert trägt — auch eine, die daneben
 * noch andere Werte hat. Nur so verschwindet die Gruppe im Diagramm wirklich:
 * Bliebe ein Projekt mit den Zielen „Wachstum" und „Qualität" stehen, stünde es
 * weiter in der Gruppe „Wachstum". Das Häkchen zeigt deshalb, ob von einem Wert
 * noch etwas zu sehen ist (`valuesOf(…).visible`), nicht, ob man ihn angeklickt hat.
 */
export function setValueVisible(
  nodes: NodeInfo[],
  hidden: Set<string>,
  role: string,
  value: string,
  visible: boolean,
): Set<string> {
  const next = new Set(hidden);
  for (const node of nodes) {
    if (!valuesFor(node, role)?.includes(value)) continue;
    if (visible) {
      next.delete(node.id);
    } else {
      next.add(node.id);
    }
  }
  return next;
}
