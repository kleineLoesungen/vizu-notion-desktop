// Was rechts steht, solange keine Quelle ausgewählt ist.

type Props = {
  hasSources: boolean;
};

export function EmptyState({ hasSources }: Props) {
  return (
    <section className="empty">
      <h1>{hasSources ? "Keine Quelle ausgewählt" : "Willkommen"}</h1>
      {hasSources ? (
        <p className="muted">Links eine Quelle wählen, um Zuordnung und Abrufstand zu sehen.</p>
      ) : (
        <>
          <p className="muted">
            Eine Quelle ist eine Notion-Datenbank unter einem Namen. Angelegt werden sie vorerst auf
            der Kommandozeile:
          </p>
          <pre className="hint">
            <code>
              vizu-notion token set{"\n"}
              vizu-notion source add Projekte --database &lt;ID&gt; --map title=Name{"\n"}
              vizu-notion fetch
            </code>
          </pre>
          <p className="muted">
            Vorhandene Konfiguration der Webapp: <code>vizu-notion source import sources.json</code>
          </p>
        </>
      )}
    </section>
  );
}
