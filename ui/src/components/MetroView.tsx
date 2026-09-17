// Die Metro-Karte: Zeitachse oben, eine Spur je Linie.
//
// Zeichnet nur — Linien, Stationen, Bänder und Marken kommen mit ihren
// Koordinaten aus `metro::build`. Die Farben der Linien stehen in core
// (crates/core/src/palette.rs), weil die Kommandozeile dieselben ausgibt;
// alles andere kommt aus theme.css.

import { useRef } from "react";
import type { MetroLine, MetroMap, MetroStation } from "../bindings";
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
              <polyline points={path(line)} stroke={line.color} />
              {line.stations.map((station) => (
                <g
                  key={station.id}
                  className={`metro-station${station.interchange ? " interchange" : ""}`}
                >
                  <circle
                    cx={station.x}
                    cy={station.y}
                    r={station.interchange ? R + 2 : R}
                    stroke={station.interchange ? undefined : line.color}
                    // Anfang und Ende voll, alles dazwischen offen — so sieht
                    // man, wo eine Linie beginnt und wo sie endet. Eine
                    // Umsteigestation gehört keiner Linie allein, sie bleibt
                    // offen (siehe app.css).
                    className={filled(station) && !station.interchange ? "filled" : ""}
                    fill={filled(station) && !station.interchange ? line.color : undefined}
                  />
                  {/* Über der Station steht der Titel oben und das Datum
                      darunter; unter ihr umgekehrt herum. Beides zusammen
                      hält Abstand zur Linie. */}
                  <text
                    x={station.x}
                    y={station.y + (station.label_above ? -LABEL_GAP - 14 : LABEL_GAP + 8)}
                  >
                    {station.title}
                  </text>
                  <text
                    className="metro-date"
                    x={station.x}
                    y={station.y + (station.label_above ? -LABEL_GAP : LABEL_GAP + 22)}
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

/**
 * Die Stationen der Linie, davor der Abzweigpunkt und dahinter die
 * Einmündung — sonst begänne eine Abzweigung im Nichts.
 *
 * Der Spurwechsel geschieht wie im Liniennetzplan über eine kurze Schräge
 * von 45 Grad und läuft dann waagerecht weiter, statt als lange Diagonale
 * quer über die Karte.
 */
function path(line: MetroLine): string {
  const points = line.stations.map((s) => [s.x, s.y] as const);
  const first = points[0];
  const last = points[points.length - 1];
  if (line.entry && first) {
    const knee = line.entry.x + Math.abs(first[1] - line.entry.y);
    points.unshift([line.entry.x, line.entry.y]);
    if (knee < first[0]) points.splice(1, 0, [knee, first[1]]);
  }
  if (line.exit && last) {
    const knee = line.exit.x - Math.abs(line.exit.y - last[1]);
    if (knee > last[0]) points.push([knee, last[1]]);
    points.push([line.exit.x, line.exit.y]);
  }
  return points.map(([x, y]) => `${x},${y}`).join(" ");
}
