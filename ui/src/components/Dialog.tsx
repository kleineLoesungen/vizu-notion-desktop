// Ein modaler Rahmen. Escape oder ein Klick daneben schließt.
//
// Bewusst kein <dialog> mit showModal(): jsdom kennt es nicht, und die Tests
// der Oberfläche laufen ohne Browser.

import { type ReactNode, useEffect } from "react";

type Props = {
  title: string;
  onClose: () => void;
  children: ReactNode;
  /**
   * Für Dialoge mit Tabellen oder Pfaden — Quelle, Einstellungen. Eine
   * Rückfrage bleibt schmal: Eine Zeile Text über die ganze Breite liest sich
   * schlecht.
   */
  wide?: boolean;
};

export function Dialog({ title, onClose, children, wide = false }: Props) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="backdrop" onPointerDown={(e) => e.target === e.currentTarget && onClose()}>
      <div
        className={wide ? "dialog wide" : "dialog"}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <h2>{title}</h2>
        {children}
      </div>
    </div>
  );
}
