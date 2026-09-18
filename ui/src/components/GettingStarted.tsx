// Die Kette bis zum ersten Diagramm — solange ein Glied fehlt.
//
// Zeichnet nur: Was erledigt ist, entscheidet der Zustand in App.tsx. Sichtbar
// bleibt der Kasten, bis alle vier Schritte stehen; ein fehlender Token führt
// ihn also auch später wieder herbei.

type Step = {
  title: string;
  done: boolean;
  hint: string;
  action?: { label: string; onClick: () => void };
  command: string;
};

type Props = {
  hasToken: boolean;
  hasSource: boolean;
  hasFetch: boolean;
  hasTemplate: boolean;
  onOpenSettings: () => void;
  onCreateSource: () => void;
  onFetchAll: () => void;
  onCreateTemplate: () => void;
};

export function GettingStarted({
  hasToken,
  hasSource,
  hasFetch,
  hasTemplate,
  onOpenSettings,
  onCreateSource,
  onFetchAll,
  onCreateTemplate,
}: Props) {
  const steps: Step[] = [
    {
      title: "Notion-Token hinterlegen",
      done: hasToken,
      hint: "Aus einer Integration unter notion.so/my-integrations. Er bleibt im Schlüsselbund.",
      action: { label: "Einstellungen öffnen", onClick: onOpenSettings },
      command: "vizu-notion token set",
    },
    {
      title: "Quelle anlegen",
      done: hasSource,
      hint: "Eine Notion-Datenbank unter einem Namen, mit der Zuordnung ihrer Spalten. Die Datenbank muss in Notion mit der Integration geteilt sein.",
      action: { label: "Neue Quelle", onClick: onCreateSource },
      command: "vizu-notion source add Projekte --database <LINK> --auto",
    },
    {
      title: "Daten abrufen",
      done: hasFetch,
      hint: "Holt Schema und alle Seiten. Danach arbeitet alles Weitere ohne Netz.",
      ...(hasSource ? { action: { label: "Alle abrufen", onClick: onFetchAll } } : {}),
      command: "vizu-notion fetch",
    },
    {
      title: "Diagramm anlegen",
      done: hasTemplate,
      hint: "Eine Vorlage in Mermaid mit Feldern aus Notion. Beispiele stehen im Editor bereit.",
      ...(hasSource ? { action: { label: "Neues Diagramm", onClick: onCreateTemplate } } : {}),
      command: "vizu-notion template import diagramm.mmd",
    },
  ];

  return (
    <section className="getting-started">
      <h1>Erste Schritte</h1>
      <p className="muted">
        Vier Schritte bis zum ersten Diagramm. Jeder geht auch auf der Kommandozeile — beide
        arbeiten auf denselben Daten.
      </p>

      <ol className="steps">
        {steps.map((step) => (
          <li key={step.title} className={step.done ? "step done" : "step"}>
            <span className="step-mark" aria-hidden="true">
              {step.done ? "✓" : "○"}
            </span>
            <div className="step-body">
              <h2>
                {step.title}
                {step.done && <span className="muted"> — erledigt</span>}
              </h2>
              <p className="muted">{step.hint}</p>
              <code className="step-command">{step.command}</code>
            </div>
            {step.action && !step.done && (
              <button type="button" className="primary" onClick={step.action.onClick}>
                {step.action.label}
              </button>
            )}
          </li>
        ))}
      </ol>
    </section>
  );
}
