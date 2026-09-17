// Versteht mermaid.js, was die Beispiele erzeugen?
//
// Die Dateien in `crates/core/tests/fixtures/examples/` schreibt
// `crates/core/tests/examples.rs`: jedes Beispiel aus dem Editor, gezeichnet
// mit echten Daten. Dort wird geprüft, dass die Vorlage überhaupt durchläuft —
// hier, dass der Text danach ein gültiges Diagramm ist.
//
// Ohne diesen Test fiele ein Tippfehler in einem Beispiel erst auf, wenn
// jemand es im Fenster auswählt und „Parse error" liest.

import { beforeAll, describe, expect, it } from "vitest";

// Über Vite gelesen, nicht über `node:fs`: So braucht der Test keine
// Node-Typen und läuft mit derselben Konfiguration wie die Anwendung.
const files: Record<string, string> = import.meta.glob(
  "../../../crates/core/tests/fixtures/examples/*.mmd",
  { query: "?raw", import: "default", eager: true },
);

// biome-ignore lint/suspicious/noExplicitAny: mermaid bringt seine Typen erst beim Laden mit
let mermaid: any;

beforeAll(async () => {
  mermaid = (await import("mermaid")).default;
  mermaid.initialize({ startOnLoad: false });
});

describe("Beispiele aus dem Editor", () => {
  it("es gibt welche", () => {
    expect(Object.keys(files).length).toBeGreaterThanOrEqual(10);
  });

  for (const [path, text] of Object.entries(files)) {
    it(`mermaid versteht ${path.split("/").pop()}`, async () => {
      await expect(mermaid.parse(text)).resolves.toBeTruthy();
    });
  }
});
