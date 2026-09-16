// Die Metro-Karte: Zeitachse oben, eine Spur je Linie.
//
// Zeichnet nur — Linien, Stationen, Bänder und Marken kommen mit ihren
// Koordinaten aus `metro::build`. Die Farben der Linien stehen in core
// (crates/core/src/palette.rs), weil die Kommandozeile dieselben ausgibt;
// alles andere kommt aus theme.css.

import { useRef } from "react";
import type { MetroMap, MetroStation } from "../bindings";
import { svgFile } from "../lib/mermaid";
import { ZoomCanvas } from "./ZoomCanvas";

type Props = {
  map: MetroMap;
  onExport?: (svg: string) => void;
};

/** Halbmesser einer Station. */
const R = 9;
/** Abstand der Beschriftung von der Station. */
const LABEL_GAP = 18;

export function MetroView({ map, onExport }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const width = Math.max(map.width, 1);
  const height = Math.max(map.height, 1);

  const tools = onExport && (
    <button
      type="button"
      className="ghost"
      onClick={() => {
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
          className="metro-svg"
          width={width}
          height={height}
          viewBox={`0 0 ${width} ${height}`}
          role="img"
          aria-label={`Metro-Karte mit ${map.lines.length} Linien`}
        >
          <title>Metro-Karte</title>

          {map.zones.map((zone) => (
            <g key={`${zone.label}-${zone.y}`} className="metro-zone">
              <rect x={0} y={zone.y} width={width} height={zone.height} />
              <text x={12} y={zone.y + 18}>
                {zone.label}
              </text>
            </g>
          ))}

          <g className="metro-axis">
            {map.ticks.map((tick) => (
              <g key={`${tick.label}-${tick.x}`}>
                <line x1={tick.x} y1={28} x2={tick.x} y2={height} />
                <text x={tick.x} y={20}>
                  {tick.label}
                </text>
              </g>
            ))}
          </g>

          {map.lines.map((line) => (
            <g key={`${line.label}-${line.stations[0]?.id}`} className="metro-line">
              <polyline
                points={line.stations.map((s) => `${s.x},${s.y}`).join(" ")}
                stroke={line.color}
              />
              {line.stations.map((station) => (
                <g key={station.id} className="metro-station">
                  <circle
                    cx={station.x}
                    cy={station.y}
                    r={R}
                    stroke={line.color}
                    // Anfang und Ende voll, alles dazwischen offen — so sieht
                    // man, wo eine Linie beginnt und wo sie endet.
                    className={filled(station) ? "filled" : ""}
                    fill={filled(station) ? line.color : undefined}
                  />
                  <text
                    x={station.x}
                    y={station.y + (station.label_above ? -LABEL_GAP : LABEL_GAP + 8)}
                  >
                    {station.title}
                  </text>
                  <text
                    className="metro-date"
                    x={station.x}
                    y={station.y + (station.label_above ? -LABEL_GAP + 14 : LABEL_GAP + 22)}
                  >
                    {station.date}
                  </text>
                </g>
              ))}
            </g>
          ))}
        </svg>
      </div>
    </ZoomCanvas>
  );
}

function filled(station: MetroStation): boolean {
  return station.kind !== "stop";
}
