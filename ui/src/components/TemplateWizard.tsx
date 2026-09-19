// Der Assistent für eine neue Mermaid-Vorlage: Typ, Quelle, Aufbau, Titel.
//
// Zeichnet nur. Die Vorlage baut core (`template::compose`) — mit Helfern,
// die an Rahmen und Sonderzeichen nicht scheitern, und jede Kombination ist
// dort mit echten Daten geprüft. Danach öffnet der gewohnte Editor, in dem
// man die Vorlage weiter von Hand anpassen kann.

import { useState } from "react";
import type { Spec } from "../bindings";
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

type Props = {
  sources: WizardSource[];
  fieldMessage: (field: string) => string | undefined;
  onCompose: (spec: Spec) => void;
  /** Ohne Assistent: eine schlichte Vorlage zum Selberschreiben. */
  onBlank: () => void;
  onCancel: () => void;
};

/** Eine Rolle vorwählen, wenn es sie gibt. */
function prefer(roles: string[], ...wanted: string[]): string {
  return wanted.find((w) => roles.includes(w)) ?? "";
}

export function TemplateWizard({ sources, fieldMessage, onCompose, onBlank, onCancel }: Props) {
  const firstReady = sources.find((s) => s.ready) ?? sources[0];
  const [kind, setKind] = useState<Kind>("flowchart");
  const [sourceName, setSourceName] = useState(firstReady?.name ?? "");
  const source = sources.find((s) => s.name === sourceName);
  const roles = (source?.roles ?? []).filter((r) => r !== "title");

  // Vorgewählt, was eine Quelle üblicherweise hergibt — änderbar.
  const [link, setLink] = useState<string | null>(null);
  const [group, setGroup] = useState<string | null>(null);
  const [color, setColor] = useState<string | null>(null);
  const [date, setDate] = useState<string | null>(null);
  const [sum, setSum] = useState("");
  const [title, setTitle] = useState<string | null>(null);

  const linkValue = link ?? prefer(roles, "next");
  const groupValue =
    group ?? (kind === "flowchart" ? "" : prefer(roles, "status", "tag", "parent"));
  const colorValue = color ?? "";
  const dateValue = date ?? prefer(roles, "date");
  const kindName = KINDS.find((k) => k.kind === kind)?.name ?? kind;
  const titleValue =
    title ?? (groupValue ? `${sourceName} nach ${groupValue}` : `${sourceName} — ${kindName}`);

  function submit() {
    const pick = (value: string) => (value === "" ? null : value);
    onCompose({
      kind,
      title: titleValue,
      source: sourceName,
      link: kind === "flowchart" ? pick(linkValue) : null,
      group: pick(groupValue),
      color: kind === "flowchart" ? pick(colorValue) : null,
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
    <section className="wizard" aria-label="Neue Vorlage">
      <header className="detail-head">
        <h1>Neues Diagramm</h1>
        <div className="head-tools">
          <button type="button" className="ghost" onClick={onBlank}>
            Ohne Assistent beginnen
          </button>
          <button type="button" className="ghost" onClick={onCancel}>
            Abbrechen
          </button>
        </div>
      </header>

      <form
        className="wizard-steps"
        onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        {/* Die Schritte bauen aufeinander auf — was man wählen kann, hängt
            vom Typ und von der Quelle davor ab. */}
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
            <span className="step-number">2</span> Quelle
          </legend>
          <label className="wizard-field">
            <span>Daten aus</span>
            <select
              id="source"
              value={sourceName}
              onChange={(e) => setSourceName(e.target.value)}
              aria-describedby="source-error"
            >
              {sources.map((s) => (
                // Ein Name mit Leerzeichen taugt nicht für `{{#each …}}` —
                // lieber gleich sagen als eine kaputte Vorlage bauen.
                <option key={s.name} value={s.name} disabled={!s.ready}>
                  {s.ready ? s.name : `${s.name} (Name taugt nicht für Vorlagen)`}
                </option>
              ))}
            </select>
            <FieldMessage id="source-error" message={fieldMessage("source")} />
          </label>
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
          <div className="wizard-grid">
            {kind === "flowchart" && (
              <>
                {roleSelect("link", "Pfeile entlang", linkValue, setLink)}
                {roleSelect("group", "Rahmen je Wert von", groupValue, setGroup)}
                {roleSelect("color", "Farbe je Wert von", colorValue, setColor)}
              </>
            )}
            {kind === "pie" && (
              <>
                {roleSelect("group", "Ein Stück je Wert von", groupValue, setGroup, null)}
                {roleSelect("sum", "Größe des Stücks", sum, setSum, "Anzahl der Seiten")}
              </>
            )}
            {kind === "gantt" && (
              <>
                {roleSelect("date", "Datum", dateValue, setDate, null)}
                {roleSelect("group", "Abschnitt je Wert von", groupValue, setGroup)}
              </>
            )}
            {kind === "mindmap" && roleSelect("group", "Zweig je Wert von", groupValue, setGroup)}
          </div>
          <p className="muted wizard-hint">{explain(kind, sum)}</p>
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
          <button type="submit" className="primary" disabled={!source}>
            Vorlage erstellen
          </button>
        </div>
      </form>
    </section>
  );
}

/** Ein Satz dazu, was herauskommt — und wie die Farben entstehen. */
function explain(kind: Kind, sum: string): string {
  switch (kind) {
    case "flowchart":
      return "Jede Seite ein Kasten. Rahmen und Farbe kommen aus den Werten des gewählten Felds — die Farben aus einer Palette von zehn.";
    case "pie":
      return sum
        ? `Jedes Stück ist so groß wie die Summe von „${sum}“ seiner Seiten. Die Farben wählt Mermaid.`
        : "Jedes Stück ist so groß wie die Anzahl seiner Seiten — jede Seite zählt einmal. Die Farben wählt Mermaid.";
    case "gantt":
      return "Jede Seite mit Datum wird ein Balken. Ein Zeitraum in Notion ergibt einen langen Balken, ein einzelner Tag einen kurzen.";
    default:
      return "Die Quelle ist die Mitte, die Werte des Felds die Zweige, die Seiten die Blätter. Ohne Feld hängen alle Seiten direkt an der Mitte.";
  }
}
