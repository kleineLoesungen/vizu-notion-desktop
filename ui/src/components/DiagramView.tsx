// Das Diagramm: zeichnen, zoomen, verschieben.
//
// Zeichnet nur. Der Mermaid-Text kommt fertig aus Rust; was hier passiert, ist
// Darstellung — mermaid.js macht daraus ein SVG, und eine CSS-Transformation
// sorgt für Zoom und Verschieben (ohne weiteres Paket).

import { type PointerEvent, useEffect, useRef, useState, type WheelEvent } from "react";
import { renderDiagram } from "../lib/mermaid";

type Props = {
  /** Der Mermaid-Text aus `template::render`. */
  mermaid: string;
  dark: boolean;
};

const MIN_ZOOM = 0.2;
const MAX_ZOOM = 4;

export function DiagramView({ mermaid, dark }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useState({ zoom: 1, x: 0, y: 0 });
  const drag = useRef<{ x: number; y: number } | null>(null);

  useEffect(() => {
    const element = host.current;
    if (!element) return;
    let current = true;
    void (async () => {
      const result = await renderDiagram(element, mermaid, {
        dark,
        isCurrent: () => current,
      });
      if (current) setError(result.ok ? null : result.message);
    })();
    // Ein neuer Text macht ein noch laufendes Zeichnen gegenstandslos.
    return () => {
      current = false;
    };
  }, [mermaid, dark]);

  function zoomAt(delta: number) {
    setView((v) => ({
      ...v,
      zoom: Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, v.zoom * delta)),
    }));
  }

  function onWheel(event: WheelEvent<HTMLDivElement>) {
    // Ohne Zusatztaste scrollt die Seite — wie in der Webapp.
    if (!event.ctrlKey && !event.metaKey) return;
    event.preventDefault();
    zoomAt(event.deltaY < 0 ? 1.1 : 1 / 1.1);
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
          onClick={() => zoomAt(1 / 1.2)}
        >
          −
        </button>
        <span className="zoom-value">{Math.round(view.zoom * 100)} %</span>
        <button type="button" className="ghost" aria-label="Vergrößern" onClick={() => zoomAt(1.2)}>
          +
        </button>
        <button type="button" className="ghost" onClick={() => setView({ zoom: 1, x: 0, y: 0 })}>
          Ansicht zurücksetzen
        </button>
      </div>

      {error && (
        <p className="diagram-error" role="alert">
          Diagramm fehlerhaft: {error}
        </p>
      )}

      {/* biome-ignore lint/a11y/noStaticElementInteractions: Zoomfläche, die Bedienung steht als Knopf daneben */}
      <div
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
