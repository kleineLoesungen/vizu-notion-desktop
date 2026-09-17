// Alle Diagramme in einer Liste: die eigenen Vorlagen und die, die eine
// Quelle aus ihrer Zuordnung hergibt.
//
// Fluss und Metro entstehen von selbst, je Quelle eines — wer eines nie
// braucht, versteckt es. Versteckte stehen in einem Aufklapp-Bereich darunter
// und lassen sich von dort zurückholen. Gemerkt wird das in core, nicht hier.
//
// Zeichnet nur. Welche Ansichten eine Quelle kann, entscheidet core
// (`fetch::views`) — hier steht nur, wie sie heißen.

import type { HiddenDiagram, SourceOverview, Template } from "../bindings";

/** Was ausgewählt wurde: eine Vorlage oder eine Ansicht einer Quelle. */
export type DiagramChoice =
  | { kind: "template"; id: string }
  | { kind: "flow"; id: string }
  | { kind: "metro"; id: string };

type Props = {
  templates: Template[];
  sources: SourceOverview[];
  selected: DiagramChoice | null;
  /** Aus core: was nicht in der Liste stehen soll. */
  hiddenDiagrams: HiddenDiagram[];
  onSelect: (choice: DiagramChoice) => void;
  onHide: (choice: DiagramChoice, hidden: boolean) => void;
  onCreate: () => void;
};

/** Die Art steht als kleine Marke neben dem Namen. */
const KIND_LABEL = { template: "Mermaid", flow: "Fluss", metro: "Metro" } as const;

type Entry = DiagramChoice & { title: string; hint: string };

export function DiagramList({
  templates,
  sources,
  selected,
  hiddenDiagrams,
  onSelect,
  onHide,
  onCreate,
}: Props) {
  const entries: Entry[] = [
    ...templates.map((template) => ({
      kind: "template" as const,
      id: template.id,
      title: template.title,
      hint: template.sources.join(", "),
    })),
    ...sources.flatMap(({ source, views }) =>
      views.map((view) => ({
        kind: view,
        id: source.id,
        title: source.name,
        hint: view === "flow" ? "entlang „next“" : "Zeitachse aus „date“",
      })),
    ),
  ];

  const isHidden = (entry: Entry) =>
    hiddenDiagrams.some((h) => h.kind === entry.kind && h.target === entry.id);
  const shown = entries.filter((entry) => !isHidden(entry));
  const gone = entries.filter(isHidden);

  /** Eine Zeile: der Name zum Auswählen, daneben der Knopf zum Verstecken. */
  const row = (entry: Entry, hidden: boolean) => (
    <li key={`${entry.kind}-${entry.id}`} className="diagram-item">
      <button
        type="button"
        className="source-item"
        // Ohne eigenen Namen läse ein Bildschirmleser „ProjekteFluss“:
        // Titel und Marke stehen ohne Leerzeichen nebeneinander.
        aria-label={`${entry.title} — ${KIND_LABEL[entry.kind]}`}
        aria-current={
          selected?.kind === entry.kind && selected.id === entry.id ? "true" : undefined
        }
        onClick={() => onSelect({ kind: entry.kind, id: entry.id })}
      >
        <span className="source-title">
          {entry.title}
          <span className="kind-badge">{KIND_LABEL[entry.kind]}</span>
        </span>
        <span className="source-excerpt">{entry.hint}</span>
      </button>
      <button
        type="button"
        className="ghost small"
        aria-label={`${entry.title} — ${KIND_LABEL[entry.kind]} ${
          hidden ? "einblenden" : "verstecken"
        }`}
        onClick={() => onHide({ kind: entry.kind, id: entry.id }, !hidden)}
      >
        {hidden ? "Einblenden" : "Verstecken"}
      </button>
    </li>
  );

  return (
    <nav className="template-nav" aria-label="Diagramme">
      <div className="sidebar-head-inline">
        <h2 className="sidebar-title">Diagramme</h2>
        <button type="button" className="ghost small" onClick={onCreate}>
          Neu
        </button>
      </div>

      {entries.length === 0 ? (
        <p className="muted sidebar-empty">
          Noch kein Diagramm. „Neu" legt eine Mermaid-Vorlage an; Fluss und Metro entstehen von
          selbst, sobald eine Quelle die passenden Rollen hat.
        </p>
      ) : (
        <ul className="source-list">{shown.map((entry) => row(entry, false))}</ul>
      )}

      {gone.length > 0 && (
        <details className="hidden-section">
          <summary>
            Versteckt<span className="muted"> {gone.length}</span>
          </summary>
          <ul className="source-list">{gone.map((entry) => row(entry, true))}</ul>
        </details>
      )}
    </nav>
  );
}
