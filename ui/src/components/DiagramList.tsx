// Alle Diagramme in einer Liste: die eigenen Vorlagen und die, die eine
// Quelle aus ihrer Zuordnung hergibt.
//
// Nach Art in Aufklapp-Bereiche geteilt: Fluss und Metro entstehen von selbst,
// je Quelle einer — bei fünf Quellen sind das zehn Einträge, die man selten
// alle sehen will. Sie starten deshalb zugeklappt, die eigenen Vorlagen offen.
//
// Zeichnet nur. Welche Ansichten eine Quelle kann, entscheidet core
// (`fetch::views`) — hier steht nur, wie sie heißen.

import { useState } from "react";
import type { SourceOverview, Template } from "../bindings";

/** Was ausgewählt wurde: eine Vorlage oder eine Ansicht einer Quelle. */
export type DiagramChoice =
  | { kind: "template"; id: string }
  | { kind: "flow"; id: string }
  | { kind: "metro"; id: string };

type Kind = DiagramChoice["kind"];

type Props = {
  templates: Template[];
  sources: SourceOverview[];
  selected: DiagramChoice | null;
  onSelect: (choice: DiagramChoice) => void;
  onCreate: () => void;
};

/** Die Art steht als Überschrift über ihrer Gruppe. */
const KIND_LABEL: Record<Kind, string> = { template: "Mermaid", flow: "Fluss", metro: "Metro" };
const ORDER: Kind[] = ["template", "flow", "metro"];

type Entry = DiagramChoice & { title: string; hint: string };

export function DiagramList({ templates, sources, selected, onSelect, onCreate }: Props) {
  // Die Gruppe der gewählten Ansicht steht offen, sonst sähe man nicht, was
  // gerade gezeichnet wird. Danach entscheidet die Bedienung.
  const [open, setOpen] = useState<Record<Kind, boolean>>(() => ({
    template: selected === null || selected.kind === "template",
    flow: selected?.kind === "flow",
    metro: selected?.kind === "metro",
  }));

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
        <div className="diagram-groups">
          {ORDER.map((kind) => {
            const group = entries.filter((entry) => entry.kind === kind);
            // Eine leere Gruppe ist kein Bereich, den man aufklappen möchte —
            // außer bei Mermaid: Dort führt „Neu" hin.
            if (group.length === 0 && kind !== "template") return null;
            return (
              <details
                key={kind}
                className="diagram-group"
                open={open[kind]}
                onToggle={(e) => setOpen({ ...open, [kind]: e.currentTarget.open })}
              >
                <summary>
                  {KIND_LABEL[kind]}
                  <span className="muted"> {group.length}</span>
                </summary>
                <ul className="source-list">
                  {group.map((entry) => (
                    <li key={`${entry.kind}-${entry.id}`}>
                      <button
                        type="button"
                        className="source-item"
                        // Ohne eigenen Namen läse ein Bildschirmleser nur den
                        // Titel vor — „Projekte" gibt es als Fluss und Metro.
                        aria-label={`${entry.title} — ${KIND_LABEL[entry.kind]}`}
                        aria-current={
                          selected?.kind === entry.kind && selected.id === entry.id
                            ? "true"
                            : undefined
                        }
                        onClick={() => onSelect({ kind: entry.kind, id: entry.id })}
                      >
                        <span className="source-title">{entry.title}</span>
                        <span className="source-excerpt">{entry.hint}</span>
                      </button>
                    </li>
                  ))}
                </ul>
              </details>
            );
          })}
        </div>
      )}
    </nav>
  );
}
