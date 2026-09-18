// Drei kleine Symbole für die Knöpfe in den Listen der Seitenleiste.
//
// Ein Wort wie „Verstecken" braucht 80 Pixel, die dem Titel daneben fehlen.
// Der Name für Bildschirmleser und Tests steht am Knopf (`aria-label`), die
// Farbe kommt über `currentColor` aus theme.css.
//
// `aria-hidden` steht an jedem <svg> selbst, nicht im gemeinsamen Objekt:
// Biome prüft das Element und sähe es im Spread nicht.

const common = {
  width: 14,
  height: 14,
  viewBox: "0 0 16 16",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.5,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
};

/** Auge, durchgestrichen: aus der Liste nehmen. */
export function HideIcon() {
  return (
    <svg {...common} aria-hidden="true">
      <path d="M2 8s2.2-4 6-4 6 4 6 4-2.2 4-6 4-6-4-6-4Z" />
      <circle cx="8" cy="8" r="1.8" />
      <path d="M2.5 13.5 13.5 2.5" />
    </svg>
  );
}

/** Auge: zurück in die Liste. */
export function ShowIcon() {
  return (
    <svg {...common} aria-hidden="true">
      <path d="M2 8s2.2-4 6-4 6 4 6 4-2.2 4-6 4-6-4-6-4Z" />
      <circle cx="8" cy="8" r="1.8" />
    </svg>
  );
}

/** Papierkorb: löschen. */
export function DeleteIcon() {
  return (
    <svg {...common} aria-hidden="true">
      <path d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8.5h5.8l.6-8.5" />
    </svg>
  );
}
