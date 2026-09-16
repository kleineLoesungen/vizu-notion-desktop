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
