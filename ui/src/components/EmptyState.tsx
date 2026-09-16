// Was rechts steht, wenn alles eingerichtet ist und nur nichts ausgewählt.

export function EmptyState() {
  return (
    <section className="empty">
      <h1>Nichts ausgewählt</h1>
      <p className="muted">Links ein Diagramm wählen — oder eine Quelle, um sie abzurufen.</p>
    </section>
  );
}
