// Das Flussdiagramm: Knoten und Kanten, wie core sie angeordnet hat.
//
// Zeichnet nur — x, y und die Größe kommen aus `flow::build`. Damit steht
// dasselbe Diagramm in der Kommandozeile (`vizu-notion flow --json`) und im
// Fenster an derselben Stelle. Farben stehen in theme.css.

import { useRef } from "react";
import type { FlowGraph } from "../bindings";
import { svgFile } from "../lib/mermaid";
import { ZoomCanvas } from "./ZoomCanvas";

type Props = {
  graph: FlowGraph;
  /** Bekommt das gezeichnete SVG. Ohne den Rückruf gibt es keinen Knopf. */
  onExport?: (svg: string) => void;
};

/** Muss zu `flow::NODE_WIDTH` und `NODE_HEIGHT` passen. */
const NODE_WIDTH = 180;
const NODE_HEIGHT = 56;
const RADIUS = 8;

export function FlowView({ graph, onExport }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const width = Math.max(graph.width, 1);
  const height = Math.max(graph.height, 1);
  const byId = new Map(graph.nodes.map((node) => [node.id, node]));

  const tools = onExport && (
    <button
      type="button"
      className="ghost"
      onClick={() => {
        // Dieselbe Hilfsfunktion wie bei Mermaid: In einer Komponente steht
        // kein Umgang mit HTML, das bewacht crates/core/tests/layering.rs.
        const svg = host.current && svgFile(host.current);
        if (svg) onExport(svg);
      }}
    >
      SVG speichern
    </button>
  );

  return (
    <ZoomCanvas contentWidth={width} contentHeight={height} tools={tools}>
      <div ref={host}>
        <svg
          className="flow-svg"
          width={width}
          height={height}
          viewBox={`0 0 ${width} ${height}`}
          role="img"
          aria-label={`Flussdiagramm mit ${graph.nodes.length} Knoten`}
        >
          <title>Flussdiagramm</title>
          <defs>
            {/* Die Pfeilspitze erbt die Farbe der Kante über context-stroke. */}
            <marker
              id="flow-arrow"
              viewBox="0 0 10 10"
              refX="9"
              refY="5"
              markerWidth="6"
              markerHeight="6"
              orient="auto-start-reverse"
            >
              <path d="M 0 0 L 10 5 L 0 10 z" className="flow-arrow-head" />
            </marker>
          </defs>

          {graph.edges.map((edge) => {
            const from = byId.get(edge.from);
            const to = byId.get(edge.to);
            if (!from || !to) return null;
            return (
              <path
                key={`${edge.from}-${edge.to}`}
                className="flow-edge"
                markerEnd="url(#flow-arrow)"
                d={edgePath(from.x, from.y, to.x, to.y)}
              />
            );
          })}

          {graph.nodes.map((node) => (
            <g key={node.id} className="flow-node">
              <rect
                x={node.x}
                y={node.y}
                width={NODE_WIDTH}
                height={NODE_HEIGHT}
                rx={RADIUS}
                ry={RADIUS}
              />
              <text x={node.x + NODE_WIDTH / 2} y={node.y + (node.subtitle ? 24 : 33)}>
                {truncate(node.title)}
              </text>
              {node.subtitle && (
                <text className="flow-subtitle" x={node.x + NODE_WIDTH / 2} y={node.y + 41}>
                  {truncate(node.subtitle)}
                </text>
              )}
            </g>
          ))}
        </svg>
      </div>
    </ZoomCanvas>
  );
}

/** Von Unterkante zu Oberkante, mit einem weichen Knick dazwischen. */
function edgePath(fromX: number, fromY: number, toX: number, toY: number): string {
  const x1 = fromX + NODE_WIDTH / 2;
  const y1 = fromY + NODE_HEIGHT;
  const x2 = toX + NODE_WIDTH / 2;
  const y2 = toY;
  const mid = (y1 + y2) / 2;
  return `M ${x1} ${y1} C ${x1} ${mid}, ${x2} ${mid}, ${x2} ${y2}`;
}

/** Ein langer Titel sprengt sonst den Knoten. */
function truncate(text: string, max = 24): string {
  return text.length > max ? `${text.slice(0, max - 1)}…` : text;
}
