// Das Diagramm: zeichnen, einpassen, zoomen, verschieben.
//
// Zeichnet nur. Der Mermaid-Text kommt fertig aus Rust; was hier passiert, ist
// Darstellung — mermaid.js macht daraus ein SVG, und eine CSS-Transformation
// sorgt für Zoom und Verschieben (ohne weiteres Paket).
//
// Nach jedem Zeichnen wird **eingepasst**: Ein Diagramm aus 130 Aufgaben ist
// sonst so groß, dass man im leeren Raum daneben landet. Sobald jemand selbst
// zoomt oder schiebt, bleibt seine Ansicht stehen.

import {
  type PointerEvent,
  useCallback,
  useEffect,
  useRef,
  useState,
  type WheelEvent,
} from "react";
import { renderDiagram, svgFile } from "../lib/mermaid";

type Props = {
  /** Der Mermaid-Text aus `template::render`. */
  mermaid: string;
  dark: boolean;
  /** Bekommt das gezeichnete SVG. Ohne den Rückruf gibt es keinen Knopf. */
  onExport?: (svg: string) => void;
};

type View = { zoom: number; x: number; y: number };

// Weit genug heraus, dass auch ein Diagramm aus 130 Knoten ganz ins Fenster
// passt. Lesbar ist es dort nicht mehr — aber darum geht es beim Einpassen
// auch nicht, sondern um den Überblick.
const MIN_ZOOM = 0.02;
const MAX_ZOOM = 4;
/** Rand zwischen Diagramm und Fläche, in Pixeln. */
const PADDING = 24;
/** Beim Einpassen wird höchstens so weit vergrößert. */
const FIT_MAX = 1.5;

// Unter zehn Prozent wäre „0 %" gerundet — eine Nachkommastelle rettet die
// Anzeige.
function zoomLabel(zoom: number): string {
  const percent = zoom * 100;
  return `${percent < 10 ? percent.toFixed(1) : Math.round(percent)} %`;
}

export function DiagramView({ mermaid, dark, onExport }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useState<View>({ zoom: 1, x: 0, y: 0 });
  const drag = useRef<{ x: number; y: number } | null>(null);

  /** Ganz sichtbar und mittig. Gibt `false` zurück, wenn nichts zu messen war. */
  const fitToView = useCallback(() => {
    const svg = host.current?.querySelector("svg");
    const area = canvas.current;
    if (!svg || !area) return false;

    // Mermaid schreibt die natürliche Größe in viewBox. `getBoundingClientRect`
    // wäre die schon skalierte Größe und damit im Kreis gerechnet.
    const box = svg.viewBox.baseVal;
    const width = box?.width || svg.clientWidth;
    const height = box?.height || svg.clientHeight;
    const available = area.getBoundingClientRect();
    if (!width || !height || !available.width || !available.height) return false;

    const zoom = Math.min(
      FIT_MAX,
      Math.max(
        MIN_ZOOM,
        Math.min(
          (available.width - 2 * PADDING) / width,
          (available.height - 2 * PADDING) / height,
        ),
      ),
    );
    setView({
      zoom,
      x: (available.width - width * zoom) / 2,
      y: (available.height - height * zoom) / 2,
    });
    return true;
  }, []);

  useEffect(() => {
    const element = host.current;
    if (!element) return;
    let current = true;
    void (async () => {
      const result = await renderDiagram(element, mermaid, {
        dark,
        isCurrent: () => current,
      });
      if (!current) return;
      setError(result.ok ? null : result.message);
      if (result.ok) {
        // Mermaid begrenzt die Breite seines SVG auf die des Fensters. Für das
        // Einpassen zählt die natürliche Größe aus der viewBox — sonst wäre
        // die Rechnung im Kreis geführt.
        const svg = element.querySelector("svg");
        const box = svg?.viewBox.baseVal;
        if (svg && box?.width && box.height) {
          svg.style.width = `${box.width}px`;
          svg.style.height = `${box.height}px`;
          svg.style.maxWidth = "none";
        }
        fitToView();
      }
    })();
    // Ein neuer Text macht ein noch laufendes Zeichnen gegenstandslos.
    return () => {
      current = false;
    };
  }, [mermaid, dark, fitToView]);

  function zoomBy(factor: number) {
    setView((v) => ({
      ...v,
      zoom: Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, v.zoom * factor)),
    }));
  }

  function onWheel(event: WheelEvent<HTMLDivElement>) {
    // Ohne Zusatztaste scrollt die Seite — wie in der Webapp.
    if (!event.ctrlKey && !event.metaKey) return;
    event.preventDefault();
    zoomBy(event.deltaY < 0 ? 1.1 : 1 / 1.1);
  }

  function onPointerDown(event: PointerEvent<HTMLDivElement>) {
    drag.current = { x: event.clientX - view.x, y: event.clientY - view.y };
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function onPointerMove(event: PointerEvent<HTMLDivElement>) {
    const start = drag.current;
    if (!start) return;
    setView((v) => ({ ...v, x: event.clientX - start.x, y: event.clientY - start.y }));
  }

  function onPointerUp(event: PointerEvent<HTMLDivElement>) {
    drag.current = null;
    event.currentTarget.releasePointerCapture(event.pointerId);
  }

  return (
    <div className="diagram-area">
      <div className="diagram-tools">
        <button
          type="button"
          className="ghost"
          aria-label="Verkleinern"
          onClick={() => zoomBy(1 / 1.2)}
        >
          −
        </button>
        <span className="zoom-value">{zoomLabel(view.zoom)}</span>
        <button type="button" className="ghost" aria-label="Vergrößern" onClick={() => zoomBy(1.2)}>
          +
        </button>
        <button type="button" className="ghost" onClick={() => fitToView()}>
          Einpassen
        </button>
        {onExport && (
          <button
            type="button"
            className="ghost"
            disabled={!!error}
            onClick={() => {
              const svg = host.current && svgFile(host.current);
              if (svg) onExport(svg);
            }}
          >
            SVG speichern
          </button>
        )}
      </div>

      {error && (
        <div className="diagram-error" role="alert">
          {`Mermaid nimmt das Diagramm nicht an:\n${error}`}
        </div>
      )}

      {error && (
        <details>
          <summary>Erzeugter Mermaid-Text</summary>
          <pre className="diagram-source">{mermaid}</pre>
        </details>
      )}

      {/* biome-ignore lint/a11y/noStaticElementInteractions: Zoomfläche, die Bedienung steht als Knopf daneben */}
      <div
        ref={canvas}
        className="diagram-canvas"
        onWheel={onWheel}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
      >
        <div
          className="diagram-stage"
          style={{ transform: `translate(${view.x}px, ${view.y}px) scale(${view.zoom})` }}
        >
          <div ref={host} className="diagram-svg" />
        </div>
      </div>

      <p className="muted diagram-hint">Strg oder Cmd + Mausrad zoomt, Ziehen verschiebt.</p>
    </div>
  );
}
