import { describe, expect, it } from "vitest";
import { ApiError } from "../api";
import { formatDateTime } from "./format";
import { fillFromSuggestion } from "./mapping";
import { externalHref, markdownToHtml } from "./markdown";
import { svgFile } from "./mermaid";
import { isLightColor } from "./theme";

describe("markdown", () => {
  it("rendert gängiges Markdown", () => {
    const html = markdownToHtml("# Titel\n\n**fett** und `code`");
    expect(html).toContain("<h1>Titel</h1>");
    expect(html).toContain("<strong>fett</strong>");
    expect(html).toContain("<code>code</code>");
  });

  it("entfernt ausführbares HTML", () => {
    // Im Webview hätte ein Skript Zugriff auf jeden IPC-Befehl.
    const html = markdownToHtml('<img src="x" onerror="alert(1)"><script>alert(2)</script>');
    expect(html).not.toContain("onerror");
    expect(html).not.toContain("<script");
  });

  it("lässt Mermaid-Blöcke als Code stehen, damit lib/mermaid.ts sie findet", () => {
    const html = markdownToHtml("```mermaid\nflowchart LR\n  A --> B\n```");
    expect(html).toContain('<code class="language-mermaid">');
    expect(html).toContain("A --&gt; B");
  });

  it("lässt nur Verweise nach draußen durch", () => {
    expect(externalHref("https://mermaid.js.org")).toBe("https://mermaid.js.org");
    expect(externalHref("mailto:a@example.org")).toBe("mailto:a@example.org");
    expect(externalHref("javascript:alert(1)")).toBeUndefined();
    expect(externalHref("/relativ")).toBeUndefined();
    expect(externalHref(null)).toBeUndefined();
  });
});

describe("Diagramm speichern", () => {
  it("macht aus dem gezeichneten SVG eine Datei mit Namensraum", () => {
    const host = document.createElement("div");
    // So ähnlich liefert es mermaid.js: ohne xmlns, weil es im HTML steht.
    host.append(document.createElementNS("http://www.w3.org/2000/svg", "svg"));

    const file = svgFile(host) ?? "";

    expect(file.startsWith("<?xml")).toBe(true);
    expect(file).toContain('xmlns="http://www.w3.org/2000/svg"');
  });

  it("gibt nichts zurück, wenn nichts gezeichnet ist", () => {
    expect(svgFile(document.createElement("div"))).toBeNull();
  });
});

describe("Anzeige", () => {
  it("zeigt einen unlesbaren Zeitstempel unverändert statt 'Invalid Date'", () => {
    expect(formatDateTime("kaputt")).toBe("kaputt");
    expect(formatDateTime("2026-09-15T06:39:51Z")).toMatch(/15\.09\.2026/);
  });

  it("wählt dunkle Schrift nur auf heller Akzentfarbe", () => {
    expect(isLightColor("#ffdd00")).toBe(true);
    expect(isLightColor("#3b6ea5")).toBe(false);
    expect(isLightColor("blau")).toBe(false);
  });
});

describe("ApiError", () => {
  it("übernimmt die Fehlerhülle aus Rust", () => {
    const err = ApiError.from({
      code: "validation_failed",
      message: "title: darf nicht leer sein",
      fields: [{ field: "title", message: "darf nicht leer sein" }],
    });
    expect(err.isValidation).toBe(true);
    expect(err.fieldMessage("title")).toBe("darf nicht leer sein");
    expect(err.fieldMessage("body")).toBeUndefined();
  });

  it("macht aus einem Text von Tauri selbst einen Programmfehler", () => {
    // So antwortet Tauri auf einen unbekannten Befehl.
    const err = ApiError.from("command note_craete not found");
    expect(err.code).toBe("internal_error");
    expect(err.isValidation).toBe(false);
  });
});

describe("Vorschlag für die Zuordnung", () => {
  const vorschlag = [
    { role: "date", property: "Start" },
    { role: "next", property: "Nächstes" },
    { role: "title", property: "Name" },
  ];

  it("füllt eine leere Spalte und hängt fehlende Rollen an", () => {
    const neu = fillFromSuggestion([{ role: "title", property: "" }], vorschlag);
    expect(neu).toEqual([
      { role: "title", property: "Name" },
      { role: "date", property: "Start" },
      { role: "next", property: "Nächstes" },
    ]);
  });

  it("überschreibt nichts, was schon eingetragen ist", () => {
    const neu = fillFromSuggestion(
      [
        { role: "title", property: "Titel" },
        { role: "date", property: "Fällig" },
      ],
      vorschlag,
    );
    expect(neu).toEqual([
      { role: "title", property: "Titel" },
      { role: "date", property: "Fällig" },
      { role: "next", property: "Nächstes" },
    ]);
  });

  it("zweimal eingearbeitet ist wie einmal", () => {
    const einmal = fillFromSuggestion([{ role: "title", property: "" }], vorschlag);
    expect(fillFromSuggestion(einmal, vorschlag)).toEqual(einmal);
  });

  it("nimmt keine Spalte doppelt, die schon unter anderer Rolle steht", () => {
    const neu = fillFromSuggestion(
      [{ role: "beginn", property: "Start" }],
      [...vorschlag, { role: "start", property: "Start" }],
    );
    expect(neu.filter((m) => m.property === "Start")).toEqual([
      { role: "beginn", property: "Start" },
    ]);
  });

  it("behält eigene Rollen, die der Vorschlag nicht kennt", () => {
    const neu = fillFromSuggestion([{ role: "owner", property: "Verantwortlich" }], vorschlag);
    expect(neu[0]).toEqual({ role: "owner", property: "Verantwortlich" });
    expect(neu).toHaveLength(4);
  });
});
