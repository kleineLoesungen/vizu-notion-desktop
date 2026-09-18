// Ein Mermaid-Diagramm anzeigen.
//
// Zeichnet nur. Der Mermaid-Text kommt fertig aus Rust; mermaid.js macht daraus
// ein SVG, und die Zeichenfläche darum herum kümmert sich um Zoom und
// Verschieben.

import { useEffect, useRef, useState } from "react";
import { failingLines } from "../lib/format";
import { renderDiagram, svgFile } from "../lib/mermaid";
import { ZoomCanvas } from "./ZoomCanvas";

type Props = {
  /** Der Mermaid-Text aus `template::render`. */
  mermaid: string;
  dark: boolean;
  /** Bekommt das gezeichnete SVG. Ohne den Rückruf gibt es keinen Knopf. */
  onExport?: (svg: string) => void;
};

export function DiagramView({ mermaid, dark, onExport }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<string | null>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });

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
      if (!result.ok) {
        setSize({ width: 0, height: 0 });
        return;
      }
      // Mermaid begrenzt die Breite seines SVG auf die des Fensters. Zum
      // Einpassen zählt die natürliche Größe aus der viewBox.
      const svg = element.querySelector("svg");
      const box = svg?.viewBox.baseVal;
      if (svg && box?.width && box.height) {
        svg.style.width = `${box.width}px`;
        svg.style.height = `${box.height}px`;
        svg.style.maxWidth = "none";
        setSize({ width: box.width, height: box.height });
      }
    })();
    // Ein neuer Text macht ein noch laufendes Zeichnen gegenstandslos.
    return () => {
      current = false;
    };
  }, [mermaid, dark]);

  const excerpt = error ? failingLines(mermaid, error) : null;

  const tools = onExport && (
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
  );

  return (
    <>
      {error && (
        <div className="diagram-error" role="alert">
          {`Mermaid nimmt das Diagramm nicht an:\n${error}`}
          {excerpt && (
            // Die Zeile im erzeugten Text, an der es scheitert — daran sieht
            // man meist sofort, welches Feld der Vorlage den Wert geliefert hat.
            <pre className="diagram-excerpt">
              {excerpt.map((line) => (
                <span key={line.number} className={line.failing ? "failing" : undefined}>
                  {`${String(line.number).padStart(3)} │ ${line.text}\n`}
                </span>
              ))}
            </pre>
          )}
        </div>
      )}
      {error && (
        <details>
          <summary>Erzeugter Mermaid-Text</summary>
          <pre className="diagram-source">{mermaid}</pre>
        </details>
      )}

      <ZoomCanvas contentWidth={size.width} contentHeight={size.height} tools={tools}>
        <div ref={host} className="diagram-svg" />
      </ZoomCanvas>
    </>
  );
}
