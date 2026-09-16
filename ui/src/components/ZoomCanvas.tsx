// Die Zeichenfläche: einpassen, zoomen, verschieben.
//
// Zeichnet nur den Rahmen — was darin steht, kommt als Kind herein: ein SVG
// von mermaid.js oder eines, das die Oberfläche selbst zeichnet. Beide sollen
// sich gleich bedienen lassen, deshalb steht die Mechanik genau einmal hier.

import {
  type PointerEvent,
  type ReactNode,
  useCallback,
  useEffect,
  useRef,
  useState,
  type WheelEvent,
} from "react";

type Props = {
  /** Größe des Inhalts in Punkten. 0 heißt: noch nichts zu messen. */
  contentWidth: number;
  contentHeight: number;
  /** Knöpfe, die zusätzlich in die Leiste gehören. */
  tools?: ReactNode;
  children: ReactNode;
};

const MIN_ZOOM = 0.02;
const MAX_ZOOM = 4;
/** Rand zwischen Inhalt und Fläche, in Punkten. */
const PADDING = 24;
/** Beim Einpassen wird höchstens so weit vergrößert. */
const FIT_MAX = 1.5;

export function ZoomCanvas({ contentWidth, contentHeight, tools, children }: Props) {
  const canvas = useRef<HTMLDivElement>(null);
  const [view, setView] = useState({ zoom: 1, x: 0, y: 0 });
  const drag = useRef<{ x: number; y: number } | null>(null);

  /** Ganz sichtbar und mittig. */
  const fitToView = useCallback(() => {
    const area = canvas.current?.getBoundingClientRect();
    if (!area?.width || !area.height || !contentWidth || !contentHeight) return;
    const zoom = Math.min(
      FIT_MAX,
      Math.max(
        MIN_ZOOM,
        Math.min(
          (area.width - 2 * PADDING) / contentWidth,
          (area.height - 2 * PADDING) / contentHeight,
        ),
      ),
    );
    setView({
      zoom,
      x: (area.width - contentWidth * zoom) / 2,
      y: (area.height - contentHeight * zoom) / 2,
    });
  }, [contentWidth, contentHeight]);

  // Neuer Inhalt, neue Ansicht: Nach einem Filterschritt ist das Diagramm
  // kleiner, und die alte Ansicht zeigte ins Leere.
  useEffect(() => {
    fitToView();
  }, [fitToView]);

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
        <button type="button" className="ghost" onClick={fitToView}>
          Einpassen
        </button>
        {tools}
      </div>

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
          {children}
        </div>
      </div>

      <p className="muted diagram-hint">Strg oder Cmd + Mausrad zoomt, Ziehen verschiebt.</p>
    </div>
  );
}

/** Unter zehn Prozent wäre „0 %" gerundet — eine Nachkommastelle rettet es. */
function zoomLabel(zoom: number): string {
  const percent = zoom * 100;
  return `${percent < 10 ? percent.toFixed(1) : Math.round(percent)} %`;
}
