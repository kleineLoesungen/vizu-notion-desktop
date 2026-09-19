// Der Assistent für eine Mermaid-Vorlage: Typ, Quellen, Aufbau, Titel.
//
// Zeichnet nur. Die Vorlage baut core (`template::compose`) und schreibt die
// Auswahl in ihren Kopf — so öffnet „Im Assistenten ändern" sie wieder, mit
// allem, was man gewählt hatte. Danach öffnet der gewohnte Editor, in dem man
// die Vorlage weiter von Hand anpassen kann.

import { useState } from "react";
import type { SourcePart, Spec } from "../bindings";
import { FieldMessage } from "./FieldMessage";

/** Eine Quelle, wie der Assistent sie braucht. */
export type WizardSource = {
  name: string;
  roles: string[];
  /** Taugt der Name für `{{#each Name}}`? Aus core. */
  ready: boolean;
};

type Kind = "flowchart" | "pie" | "gantt" | "mindmap";

const KINDS: { kind: Kind; name: string; what: string }[] = [
  { kind: "flowchart", name: "Flowchart", what: "Kästen und Pfeile — Abläufe, Abhängigkeiten" },
  { kind: "pie", name: "Pie Chart", what: "Anteile — wie viele je Status, Summe je Bereich" },
  { kind: "gantt", name: "Gantt", what: "Balken auf einer Zeitachse — Termine, Zeiträume" },
  { kind: "mindmap", name: "Mindmap", what: "Ein Baum — Seiten nach einem Feld sortiert" },
];

/** Die Farbe im Flowchart: keine, je Quelle oder je Wert einer Rolle. */
const BY_SOURCE = "@quelle";

type Props = {
  sources: WizardSource[];
  /** Eine frühere Auswahl — „Im Assistenten ändern". */
  initial: Spec | null;
  /** Die Vorlage wurde seit dem Assistenten von Hand geändert. */
  edited: boolean;
  fieldMessage: (field: string) => string | undefined;
  onCompose: (spec: Spec) => void;
  /** Ohne Assistent: eine schlichte Vorlage zum Selberschreiben. */
  onBlank?: () => void;
  onCancel: () => void;
};

/** Eine Rolle vorwählen, wenn es sie gibt. */
function prefer(roles: string[], ...wanted: string[]): string {
  return wanted.find((w) => roles.includes(w)) ?? "";
}

export function TemplateWizard({
  sources,
  initial,
  edited,
  fieldMessage,
  onCompose,
  onBlank,
  onCancel,
}: Props) {
  const firstReady = sources.find((s) => s.ready);
  const [kind, setKind] = useState<Kind>((initial?.kind as Kind) ?? "flowchart");
  const [chosen, setChosen] = useState<SourcePart[]>(
    initial?.sources ??
      (firstReady
        ? [
            {
              name: firstReady.name,
              link: prefer(firstReady.roles, "next") || null,
              link_to: null,
            },
          ]
        : []),
  );
  const [group, setGroup] = useState<string | null>(initial ? (initial.group ?? "") : null);
  const [byFrame, setByFrame] = useState(initial?.by_source ?? false);
  const [color, setColor] = useState(initial?.color_by_source ? BY_SOURCE : (initial?.color ?? ""));
  const [date, setDate] = useState<string | null>(initial?.date ?? null);
  const [sum, setSum] = useState(initial?.sum ?? "");
  const [title, setTitle] = useState<string | null>(initial?.title ?? null);

  const names = chosen.map((c) => c.name);
  const several = chosen.length > 1;
  const rolesOf = (name: string) =>
    (sources.find((s) => s.name === name)?.roles ?? []).filter((r) => r !== "title");
  // Alle Rollen der gewählten Quellen — eine Rolle, die nur eine Quelle hat,
  // wirkt auch nur dort.
  const roles = [...new Set(names.flatMap(rolesOf))];

  const groupValue =
    group ?? (kind === "flowchart" ? "" : prefer(roles, "status", "tag", "parent"));
  const dateValue = date ?? prefer(roles, "date");
  const kindName = KINDS.find((k) => k.kind === kind)?.name ?? kind;
  const joined = names.join(" und ");
  const titleValue =
    title ?? (groupValue ? `${joined} nach ${groupValue}` : `${joined} — ${kindName}`);

  function toggleSource(name: string, on: boolean) {
    setChosen(
      on
        ? [...chosen, { name, link: prefer(rolesOf(name), "next") || null, link_to: null }]
        : chosen.filter((c) => c.name !== name),
    );
  }

  function changePart(name: string, patch: Partial<SourcePart>) {
    setChosen(chosen.map((c) => (c.name === name ? { ...c, ...patch } : c)));
  }

  function submit() {
    const pick = (value: string) => (value === "" ? null : value);
    onCompose({
      kind,
      title: titleValue,
      // Pfeile gibt es nur im Flowchart; eine Zielquelle, die nicht mehr
      // gewählt ist, fällt weg, statt einen Fehler auszulösen.
      sources: chosen.map((c) => ({
        name: c.name,
        link: kind === "flowchart" ? (c.link ?? null) : null,
        link_to: kind === "flowchart" && c.link_to && names.includes(c.link_to) ? c.link_to : null,
      })),
      group: pick(groupValue),
      by_source: kind === "flowchart" && several && byFrame,
      color: kind === "flowchart" && color !== BY_SOURCE ? pick(color) : null,
      color_by_source: kind === "flowchart" && color === BY_SOURCE,
      date: kind === "gantt" ? pick(dateValue) : null,
      sum: kind === "pie" ? pick(sum) : null,
    });
  }

  /**
   * Ein Auswahlfeld für eine Rolle. `empty` ist, was die leere Wahl bedeutet
   * („—" für „nicht benutzen"); `null` heißt: Die Art braucht dieses Feld.
   */
  const roleSelect = (
    id: string,
    label: string,
    value: string,
    onChange: (v: string) => void,
    empty: string | null = "—",
  ) => (
    <label className="wizard-field">
      <span>{label}</span>
      <select
        id={id}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        aria-describedby={`${id}-error`}
      >
        {empty !== null && <option value="">{empty}</option>}
        {empty === null && value === "" && <option value="">auswählen …</option>}
        {roles.map((role) => (
          <option key={role} value={role}>
            {role}
          </option>
        ))}
      </select>
      <FieldMessage id={`${id}-error`} message={fieldMessage(id)} />
    </label>
  );

  return (
    <section
      className="wizard"
      aria-label={initial ? "Vorlage im Assistenten ändern" : "Neue Vorlage"}
    >
      <header className="detail-head">
        <h1>{initial ? "Im Assistenten ändern" : "Neues Diagramm"}</h1>
        <div className="head-tools">
          {onBlank && (
            <button type="button" className="ghost" onClick={onBlank}>
              Ohne Assistent beginnen
            </button>
          )}
          <button type="button" className="ghost" onClick={onCancel}>
            Abbrechen
          </button>
        </div>
      </header>

      {edited && (
        // Der Assistent baut die Vorlage neu — was man von Hand geändert hat,
        // wäre weg. Das Ergebnis landet ungespeichert im Editor; man kann es
        // dort noch verwerfen.
        <p className="wizard-warning" role="status">
          Diese Vorlage wurde seit dem Assistenten von Hand geändert. Baut der Assistent sie neu,
          fallen diese Änderungen weg — das Ergebnis erscheint ungespeichert im Editor und lässt
          sich dort noch verwerfen.
        </p>
      )}

      <form
        className="wizard-steps"
        onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        {/* Die Schritte bauen aufeinander auf — was man wählen kann, hängt
            vom Typ und von den Quellen davor ab. */}
        <fieldset className="wizard-step">
          <legend>
            <span className="step-number">1</span> Diagrammtyp
          </legend>
          <div className="kind-cards">
            {KINDS.map((k) => (
              <label key={k.kind} className="kind-card" data-checked={kind === k.kind}>
                <input
                  type="radio"
                  name="kind"
                  value={k.kind}
                  checked={kind === k.kind}
                  onChange={() => setKind(k.kind)}
                />
                <span className="kind-name">{k.name}</span>
                <span className="muted">{k.what}</span>
              </label>
            ))}
          </div>
          <FieldMessage id="kind-error" message={fieldMessage("kind")} />
        </fieldset>

        <fieldset className="wizard-step">
          <legend>
            <span className="step-number">2</span> Quellen
          </legend>
          <fieldset className="source-checks" aria-label="Quellen">
            {sources.map((s) => (
              <label key={s.name} className="source-check" data-disabled={!s.ready}>
                <input
                  type="checkbox"
                  checked={names.includes(s.name)}
                  // Ein Name mit Leerzeichen taugt nicht für `{{#each …}}` —
                  // lieber gleich sagen als eine kaputte Vorlage bauen.
                  disabled={!s.ready}
                  onChange={(e) => toggleSource(s.name, e.target.checked)}
                />
                <span>{s.name}</span>
                {!s.ready && <span className="muted">Name taugt nicht für Vorlagen</span>}
              </label>
            ))}
          </fieldset>
          <FieldMessage id="sources-error" message={fieldMessage("sources")} />
          {sources.some((s) => !s.ready) && (
            <p className="muted wizard-hint">
              Eine Quelle, deren Name Leerzeichen oder Bindestriche enthält, lässt sich in Vorlagen
              nicht ansprechen. Unter „Quellen“ umbenennen, etwa in „Roadmap“.
            </p>
          )}
        </fieldset>

        <fieldset className="wizard-step">
          <legend>
            <span className="step-number">3</span> Aufbau
          </legend>

          {kind === "flowchart" && (
            <>
              {/* Pfeile gehören zu einer Quelle — und können zu einer
                  anderen gewählten führen: Aufgaben → ihr Projekt. */}
              <div className="link-rows">
                {chosen.map((part) => (
                  <div key={part.name} className="link-row">
                    <span className="link-from">{part.name}</span>
                    <label className="inline-label">
                      Pfeile entlang
                      <select
                        aria-label={`Pfeile von ${part.name} entlang`}
                        value={part.link ?? ""}
                        onChange={(e) => changePart(part.name, { link: e.target.value || null })}
                      >
                        <option value="">—</option>
                        {rolesOf(part.name).map((role) => (
                          <option key={role} value={role}>
                            {role}
                          </option>
                        ))}
                      </select>
                    </label>
                    {part.link && several && (
                      <label className="inline-label">
                        zu
                        <select
                          aria-label={`Pfeile von ${part.name} zu`}
                          value={part.link_to ?? ""}
                          onChange={(e) =>
                            changePart(part.name, { link_to: e.target.value || null })
                          }
                        >
                          <option value="">{part.name} (dieselbe)</option>
                          {names
                            .filter((n) => n !== part.name)
                            .map((n) => (
                              <option key={n} value={n}>
                                {n}
                              </option>
                            ))}
                        </select>
                      </label>
                    )}
                  </div>
                ))}
              </div>
              <FieldMessage id="link-error" message={fieldMessage("link")} />
              <div className="wizard-grid">
                {roleSelect("group", "Rahmen je Wert von", groupValue, setGroup)}
                <label className="wizard-field">
                  <span>Farbe</span>
                  <select
                    id="color"
                    value={color}
                    onChange={(e) => setColor(e.target.value)}
                    aria-describedby="color-error"
                  >
                    <option value="">keine</option>
                    {several && <option value={BY_SOURCE}>je Quelle</option>}
                    {roles.map((role) => (
                      <option key={role} value={role}>
                        je Wert von {role}
                      </option>
                    ))}
                  </select>
                  <FieldMessage id="color-error" message={fieldMessage("color")} />
                </label>
              </div>
              {several && (
                <label className="wizard-check">
                  <input
                    type="checkbox"
                    checked={byFrame}
                    onChange={(e) => setByFrame(e.target.checked)}
                  />
                  Ein Rahmen je Quelle
                </label>
              )}
            </>
          )}

          {kind === "pie" && (
            <div className="wizard-grid">
              {roleSelect(
                "group",
                several ? "Stücke je Quelle, dazu je Wert von" : "Ein Stück je Wert von",
                groupValue,
                setGroup,
                several ? "—" : null,
              )}
              {roleSelect("sum", "Größe des Stücks", sum, setSum, "Anzahl der Seiten")}
            </div>
          )}

          {kind === "gantt" && (
            <div className="wizard-grid">
              {roleSelect("date", "Datum", dateValue, setDate, null)}
              {roleSelect("group", "Abschnitt je Wert von", groupValue, setGroup)}
            </div>
          )}

          {kind === "mindmap" && (
            <div className="wizard-grid">
              {roleSelect("group", "Zweig je Wert von", groupValue, setGroup)}
            </div>
          )}

          <p className="muted wizard-hint">{explain(kind, sum, several)}</p>
        </fieldset>

        <fieldset className="wizard-step">
          <legend>
            <span className="step-number">4</span> Titel
          </legend>
          <label className="wizard-field wide">
            <span>Überschrift</span>
            <input
              id="title"
              value={titleValue}
              onChange={(e) => setTitle(e.target.value)}
              aria-describedby="title-error"
            />
            <FieldMessage id="title-error" message={fieldMessage("title")} />
          </label>
        </fieldset>

        <div className="wizard-actions">
          <p className="muted">Die Vorlage lässt sich danach im Editor von Hand anpassen.</p>
          <button type="submit" className="primary" disabled={chosen.length === 0}>
            {initial ? "Vorlage neu bauen" : "Vorlage erstellen"}
          </button>
        </div>
      </form>
    </section>
  );
}

/** Ein Satz dazu, was herauskommt — und wie die Farben entstehen. */
function explain(kind: Kind, sum: string, several: boolean): string {
  switch (kind) {
    case "flowchart":
      return several
        ? "Jede Seite ein Kasten. Pfeile können zu einer anderen Quelle führen — etwa von einer Aufgabe zu ihrem Projekt. Gleiche Werte haben in jeder Quelle dieselbe Farbe."
        : "Jede Seite ein Kasten. Rahmen und Farbe kommen aus den Werten des gewählten Felds — die Farben aus einer Palette von zehn.";
    case "pie": {
      const size = sum
        ? `so groß wie die Summe von „${sum}“ seiner Seiten`
        : "so groß wie die Anzahl seiner Seiten — jede Seite zählt einmal";
      return several
        ? `Jede Quelle ein Stück, mit Feld je Quelle und Wert — jedes ${size}. Die Farben wählt Mermaid.`
        : `Jedes Stück ist ${size}. Die Farben wählt Mermaid.`;
    }
    case "gantt":
      return several
        ? "Jede Quelle ein Abschnitt, jede Seite mit Datum ein Balken. Ein Zeitraum in Notion ergibt einen langen Balken."
        : "Jede Seite mit Datum wird ein Balken. Ein Zeitraum in Notion ergibt einen langen Balken, ein einzelner Tag einen kurzen.";
    default:
      return several
        ? "Der Titel ist die Mitte, jede Quelle ein Zweig, darunter die Werte des Felds und die Seiten."
        : "Die Quelle ist die Mitte, die Werte des Felds die Zweige, die Seiten die Blätter.";
  }
}
