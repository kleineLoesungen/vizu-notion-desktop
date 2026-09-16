// Was rechts steht, solange nichts ausgewählt ist.

type Props = {
  hasSources: boolean;
  hasTemplates: boolean;
  onCreateSource: () => void;
};

export function EmptyState({ hasSources, hasTemplates, onCreateSource }: Props) {
  if (hasSources && hasTemplates) {
    return (
      <section className="empty">
        <h1>Nichts ausgewählt</h1>
        <p className="muted">Links ein Diagramm wählen — oder eine Quelle, um sie abzurufen.</p>
      </section>
    );
  }

  return (
    <section className="empty">
      <h1>{hasSources ? "Noch kein Diagramm" : "Willkommen"}</h1>
      <p className="muted">
        {hasSources
          ? "Eine Vorlage ist eine .mmd-Datei: ein Kopf mit Titel und Quellen, darunter ein Mermaid-Diagramm mit Feldern aus Notion."
          : "Eine Quelle ist eine Notion-Datenbank unter einem Namen. Daraus entstehen die Diagramme."}
      </p>
      {!hasSources && (
        <div className="actions">
          <button type="button" className="primary" onClick={onCreateSource}>
            Neue Quelle
          </button>
        </div>
      )}
      <p className="muted">Dasselbe auf der Kommandozeile — sie arbeitet auf denselben Daten:</p>
      <pre className="hint">
        <code>
          {hasSources ? (
            "vizu-notion template import diagramm.mmd"
          ) : (
            <>
              vizu-notion source add Projekte --database &lt;ID&gt; --map title=Name{"\n"}
              vizu-notion fetch{"\n"}
              vizu-notion source import sources.json
            </>
          )}
        </code>
      </pre>
    </section>
  );
}
