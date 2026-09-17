// Die Oberfläche ohne Fenster durchklicken.
//
// `mockIPC` ersetzt die Rust-Seite: Jeder `invoke` aus api.ts landet in
// `backend` unten. So wird geprüft, dass die Oberfläche die richtigen Befehle
// mit den richtigen Argumentnamen schickt und Antworten richtig anzeigt.
//
// Die Attrappe hat absichtlich KEINE eigenen Regeln — sie lehnt nur ab, was
// der Test ihr sagt. Die Regeln stehen in crates/core und werden dort
// geprüft. Ob die echte Rust-Seite dieselben Namen erwartet, prüft
// crates/desktop/tests/ipc_contract.rs.

import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import type {
  ApiError,
  Config,
  Diagram,
  FetchStatus,
  SourceInput,
  SourceOverview,
  Template,
  TemplateInput,
  TokenStatus,
} from "./bindings";

// mermaid.js zeichnet in jsdom nicht — und hier geht es auch nicht ums
// Zeichnen, sondern darum, dass die Oberfläche den richtigen Text anfordert.
// Was daraus ein SVG macht, ist Sache von mermaid.
const drawn: string[] = [];
const saved: { path: string | null; svg?: string } = { path: null };

// Der Speichern-Dialog des Systems gibt es in jsdom nicht.
vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: () => Promise.resolve(saved.path),
}));
vi.mock("./lib/mermaid", () => ({
  renderDiagram: (host: HTMLElement, text: string) => {
    drawn.push(text);
    host.textContent = text;
    return Promise.resolve({ ok: true });
  },
  renderDiagrams: () => Promise.resolve(),
  // Die Attrappe zeichnet Text statt SVG; für den Export genügt er.
  svgFile: (host: HTMLElement) => `<svg>${host.textContent}</svg>`,
  MERMAID_SELECTOR: "",
}));

type Call = { cmd: string; args: Record<string, unknown> };

class Rejection {
  constructor(readonly error: ApiError) {}
}

const STAMP = "2026-09-16T06:39:51Z";

function overview(id: string, name: string, pages?: number): SourceOverview {
  return {
    views: [],
    source: {
      id,
      name,
      database_id: "396f6627-0f5d-8034-b55c-ebc685aa5e50",
      mappings: [{ role: "title", property: "Name" }],
      created_at: STAMP,
      updated_at: STAMP,
    },
    fetch:
      pages === undefined
        ? null
        : {
            source_id: id,
            database_title: `vizu ${name}`,
            database_url: "https://app.notion.com/p/396f66270f5d8034b55cebc685aa5e50",
            page_count: pages,
            request_count: 4,
            fetched_at: STAMP,
          },
  };
}

function template(id: string, title: string): Template {
  return {
    id,
    slug: title.toLowerCase(),
    title,
    sources: ["Projekte"],
    body: "---\ntitle: x\nsources:\n  - Projekte\n---\nflowchart TD\n",
    created_at: STAMP,
    updated_at: STAMP,
  };
}

/** Zwei Projekte, das erste zeigt auf das zweite. */
function diagramFor(hidden: string[]): Diagram {
  const nodes = [
    { id: "p1", title: "Website", source: "Projekte", relations: ["p2"] },
    { id: "p2", title: "Launch", source: "Projekte", relations: [] },
    { id: "z1", title: "Wachstum", source: "Ziele", relations: [] },
  ].filter((n) => !hidden.includes(n.id));
  return {
    title: "Fahrplan",
    mermaid: `flowchart TD\n${nodes.map((n) => `  ${n.id}["${n.title}"]`).join("\n")}`,
    // Ausgeblendete Knoten bleiben in der Liste — das Filterfeld braucht sie.
    nodes: [
      { id: "p1", title: "Website", source: "Projekte", relations: ["p2"] },
      { id: "p2", title: "Launch", source: "Projekte", relations: [] },
      { id: "z1", title: "Wachstum", source: "Ziele", relations: [] },
    ],
  };
}

let sources: SourceOverview[];
let templates: Template[];
let hiddenDiagrams: { kind: string; target: string }[];
let calls: Call[];
let config: Config;
let token: TokenStatus;
let nextError: ApiError | null;

function backend(cmd: string, args: Record<string, unknown> = {}): unknown {
  calls.push({ cmd, args });
  if (nextError) {
    const error = nextError;
    nextError = null;
    throw new Rejection(error);
  }
  switch (cmd) {
    case "source_create":
    case "source_update": {
      const input = args.input as SourceInput;
      const created = {
        id: (args.id as string) ?? "neu",
        name: input.name,
        database_id: input.database_id,
        mappings: input.mappings,
        created_at: STAMP,
        updated_at: STAMP,
      };
      sources = [{ source: created, fetch: null, views: [] }];
      return created;
    }
    case "metro_render":
      return {
        lines: [
          {
            label: "Website",
            color: "#4e79a7",
            stations: [
              {
                id: "p1",
                title: "Website",
                date: "2026-01-12",
                x: 120,
                y: 96,
                kind: "start",
                label_above: true,
              },
              {
                id: "p2",
                title: "Launch",
                date: "2026-04-01",
                x: 600,
                y: 96,
                kind: "terminus",
                label_above: false,
              },
            ],
          },
        ],
        zones: [{ label: "Web", y: 48, height: 96 }],
        ticks: [{ label: "01/2026", x: 120 }],
        width: 1440,
        height: 144,
        all_nodes: [
          { id: "p1", title: "Website", source: "Projekte", relations: ["p2"] },
          { id: "p2", title: "Launch", source: "Projekte", relations: [] },
        ],
        undated: ["Ohne Ziel"],
      };
    case "flow_render":
      return {
        nodes: [
          { id: "p1", title: "Website", subtitle: "Aktiv", x: 0, y: 0 },
          { id: "p2", title: "Launch", subtitle: "", x: 0, y: 128 },
        ].filter((n) => !(args.hidden as string[]).includes(n.id)),
        edges: [{ from: "p1", to: "p2" }],
        width: 180,
        height: 184,
        all_nodes: [
          { id: "p1", title: "Website", source: "Projekte", relations: ["p2"] },
          { id: "p2", title: "Launch", source: "Projekte", relations: [] },
        ],
        subtitle_roles: ["status"],
      };
    case "source_properties":
      return [{ name: "Name", id: "title", kind: "title", relation_to: null }];
    case "database_inspect":
      return {
        title: "vizu Projekte",
        properties: [
          { name: "Name", id: "title", kind: "title", relation_to: null },
          { name: "Nächstes", id: "WGBf", kind: "relation", relation_to: "ds" },
          { name: "Start", id: "n%5EzI", kind: "date", relation_to: null },
        ],
        suggestion: [
          { role: "date", property: "Start" },
          { role: "next", property: "Nächstes" },
          { role: "title", property: "Name" },
        ],
      };
    case "template_help":
      return { examples: [], hints: [] };
    case "template_get":
      return templates[0];
    case "template_delete":
    case "source_delete":
      return null;
    case "template_save":
      return { ...template("t1", "Fahrplan"), slug: (args.input as TemplateInput).slug };
    case "diagram_preview":
      return diagramFor([]);
    case "export_svg":
      saved.svg = args.svg as string;
      return args.path as string;
    case "token_set":
    case "token_clear":
      return token;
    case "config_get":
      return config;
    case "config_set":
      config = args.config as Config;
      return config;
    case "app_info":
      return {
        version: "0.1.0",
        data_dir: "/tmp/d",
        config_file: "/tmp/c",
        db_file: "/tmp/db",
        log_file: "/tmp/l",
      };
    case "token_status":
      return token;
    case "hidden_list":
      return hiddenDiagrams;
    case "hidden_set": {
      const entry = args.entry as { kind: string; target: string };
      hiddenDiagrams = (args.hidden as boolean)
        ? [...hiddenDiagrams, entry]
        : hiddenDiagrams.filter((h) => !(h.kind === entry.kind && h.target === entry.target));
      return hiddenDiagrams;
    }
    case "template_list":
      return templates;
    case "diagram_render":
      return diagramFor(args.hidden as string[]);
    case "source_list":
      return sources;
    case "source_fetch": {
      const id = args.id as string;
      sources = sources.map((s) => (s.source.id === id ? overview(id, s.source.name, 12) : s));
      return sources.find((s) => s.source.id === id)?.fetch as FetchStatus;
    }
    default:
      // Fenster-API und Plugins: annehmen, nichts tun.
      return null;
  }
}

beforeEach(() => {
  sources = [];
  templates = [];
  hiddenDiagrams = [];
  calls = [];
  drawn.length = 0;
  saved.path = null;
  saved.svg = undefined;
  nextError = null;
  config = { theme: "light", accent: "#3b6ea5" };
  token = { origin: "store", hint: "ntn_…stuv", store: "Schlüsselbund", store_error: null };
  mockWindows("main");
  mockIPC((cmd, args) => {
    try {
      return backend(cmd, args as Record<string, unknown>);
    } catch (e) {
      // Ein abgelehntes Promise mit der Fehlerhülle — wie aus Rust.
      if (e instanceof Rejection) return Promise.reject(e.error);
      throw e;
    }
  });
});

function commands(name: string): Call[] {
  return calls.filter((c) => c.cmd === name);
}

describe("Oberfläche", () => {
  it("lädt beim Start Einstellungen, Token und Quellen", async () => {
    sources = [overview("a", "Projekte", 12)];
    render(<App />);

    expect(await screen.findByRole("button", { name: /Projekte/ })).toBeTruthy();
    expect(commands("source_list")).toHaveLength(1);
    // Das Farbschema setzt ein Effekt, nachdem die Einstellungen da sind —
    // das kann einen Wimpernschlag nach der Liste geschehen.
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("light"));
  });

  it("führt durch die vier Schritte bis zum ersten Diagramm", async () => {
    const user = userEvent.setup();
    render(<App />);

    expect(await screen.findByText("Erste Schritte")).toBeTruthy();
    // Der Token ist da, die Quelle fehlt — also führt der Weg dorthin.
    const tokenSchritt = screen.getByRole("heading", { name: /Notion-Token hinterlegen/ });
    expect(tokenSchritt.closest("li")?.className).toContain("done");
    const schritte = screen.getByText("Erste Schritte").closest("section") as HTMLElement;
    await user.click(within(schritte).getByRole("button", { name: "Neue Quelle" }));
    expect(screen.getByRole("textbox", { name: "Name" })).toBeTruthy();
  });

  it("weist auf den fehlenden Token hin", async () => {
    const user = userEvent.setup();
    token = { origin: null, hint: null, store: "Schlüsselbund", store_error: null };
    render(<App />);

    expect(await screen.findByText(/Kein Notion-Token hinterlegt/)).toBeTruthy();
    // Der erste Schritt ist offen und führt in die Einstellungen.
    await user.click(screen.getByRole("button", { name: "Einstellungen öffnen" }));
    expect(screen.getByLabelText("Neuer Token")).toBeTruthy();
  });

  it("ruft eine Quelle ab und zeigt den neuen Stand", async () => {
    const user = userEvent.setup();
    sources = [overview("a", "Projekte")];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /Projekte/ }));
    expect(screen.getByText("noch nie")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Von Notion abrufen" }));

    await waitFor(() => expect(commands("source_fetch")).toHaveLength(1));
    expect(commands("source_fetch")[0]?.args).toEqual({ id: "a" });
    expect(await screen.findByText(/12 Seiten in 4 Anfragen/)).toBeTruthy();
  });

  it("ruft mit „Alle abrufen“ jede Quelle einzeln ab", async () => {
    const user = userEvent.setup();
    sources = [overview("a", "Projekte"), overview("b", "Aufgaben")];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Alle abrufen" }));

    await waitFor(() => expect(commands("source_fetch")).toHaveLength(2));
    expect(commands("source_fetch").map((c) => c.args.id)).toEqual(["a", "b"]);
  });

  it("zeigt einen Fehler von Notion oben", async () => {
    const user = userEvent.setup();
    sources = [overview("a", "Projekte")];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /Projekte/ }));
    nextError = {
      code: "notion_not_shared",
      message: "Notion: nicht gefunden — ist sie mit der Integration geteilt?",
    };
    await user.click(screen.getByRole("button", { name: "Von Notion abrufen" }));

    expect(await screen.findByText(/mit der Integration geteilt/)).toBeTruthy();
  });

  it("öffnet den Verweis nach Notion über das Opener-Plugin statt im Fenster", async () => {
    const user = userEvent.setup();
    sources = [overview("a", "Projekte", 12)];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /Projekte/ }));
    await user.click(screen.getByRole("link", { name: /in Notion öffnen/ }));

    await waitFor(() => expect(commands("plugin:opener|open_url")).toHaveLength(1));
    expect(commands("plugin:opener|open_url")[0]?.args).toMatchObject({
      url: "https://app.notion.com/p/396f66270f5d8034b55cebc685aa5e50",
    });
  });

  it("speichert Einstellungen und wendet die Akzentfarbe an", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Einstellungen" }));
    const accent = screen.getByRole("textbox", { name: "Akzentfarbe" });
    await user.clear(accent);
    await user.type(accent, "#ffdd00");
    await user.click(screen.getByRole("button", { name: "Übernehmen" }));

    await waitFor(() => expect(commands("config_set")).toHaveLength(1));
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("#ffdd00");
    expect(document.documentElement.dataset.accentLight).toBe("true");
  });

  it("zeigt einen Eingabefehler am Feld, nicht oben", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Einstellungen" }));
    nextError = {
      code: "validation_failed",
      message: "accent: erwartet #rrggbb",
      fields: [{ field: "accent", message: "erwartet #rrggbb" }],
    };
    await user.click(screen.getByRole("button", { name: "Übernehmen" }));

    // Die Meldung steht unter dem Feld, auf das sie sich bezieht …
    const message = await screen.findByText("erwartet #rrggbb");
    expect(message.id).toBe("accent-error");
    expect(
      screen.getByRole("textbox", { name: "Akzentfarbe" }).getAttribute("aria-describedby"),
    ).toBe("accent-error");
    // … und nicht oben im Fenster.
    expect(document.querySelector(".banner")).toBeNull();
  });

  it("zeichnet eine Vorlage, sobald sie gewählt ist", async () => {
    const user = userEvent.setup();
    templates = [template("t1", "Fahrplan")];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Fahrplan — Mermaid" }));

    await waitFor(() => expect(commands("diagram_render")).toHaveLength(1));
    expect(commands("diagram_render")[0]?.args).toEqual({ id: "t1", hidden: [] });
    expect(drawn[0]).toContain("Website");
  });

  it("holt vom Diagramm aus die Quellen und zeichnet danach neu", async () => {
    const user = userEvent.setup();
    templates = [template("t1", "Fahrplan")];
    // Die Vorlage benutzt „Projekte"; „Aufgaben" gehört nicht dazu.
    sources = [overview("a", "Projekte", 12), overview("b", "Aufgaben", 130)];
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Fahrplan — Mermaid" }));
    await waitFor(() => expect(commands("diagram_render")).toHaveLength(1));

    await user.click(screen.getByRole("button", { name: "Quelle abrufen" }));

    await waitFor(() => expect(commands("source_fetch")).toHaveLength(1));
    expect(commands("source_fetch")[0]?.args).toEqual({ id: "a" });
    // Mit frischen Daten zeigt das Diagramm sonst den alten Stand.
    await waitFor(() => expect(commands("diagram_render")).toHaveLength(2));
  });

  it("blendet einen Knoten aus und lässt in Rust neu zeichnen", async () => {
    const user = userEvent.setup();
    templates = [template("t1", "Fahrplan")];
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Fahrplan — Mermaid" }));
    await screen.findByRole("checkbox", { name: /Website/ });

    await user.click(screen.getByRole("checkbox", { name: /Website/ }));

    await waitFor(() => expect(commands("diagram_render")).toHaveLength(2));
    expect(commands("diagram_render")[1]?.args).toEqual({ id: "t1", hidden: ["p1"] });
    // Der ausgeblendete Knoten steht weiter in der Liste, sonst käme er nie zurück.
    expect(screen.getByRole("checkbox", { name: /Website/ })).toBeTruthy();
  });

  it("zeigt mit „Verwandte“ nur den Knoten und seine Nachbarn", async () => {
    const user = userEvent.setup();
    templates = [template("t1", "Fahrplan")];
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Fahrplan — Mermaid" }));
    await screen.findByRole("checkbox", { name: /Website/ });

    await user.click(screen.getByRole("button", { name: "Verwandte von Website zeigen" }));

    // „Wachstum" hängt an nichts und fällt heraus, „Launch" bleibt als Ziel.
    await waitFor(() =>
      expect(commands("diagram_render")[1]?.args).toEqual({
        id: "t1",
        hidden: ["z1"],
      }),
    );
  });

  it("schaltet eine ganze Quelle auf einmal aus", async () => {
    const user = userEvent.setup();
    templates = [template("t1", "Fahrplan")];
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Fahrplan — Mermaid" }));
    const panel = await screen.findByRole("complementary", { name: "Knoten filtern" });
    const gruppe = within(panel).getByText("Projekte").closest("details") as HTMLElement;

    await user.click(within(gruppe).getByRole("button", { name: "Keine" }));

    await waitFor(() =>
      expect(commands("diagram_render")[1]?.args).toEqual({ id: "t1", hidden: ["p1", "p2"] }),
    );
  });

  it("legt eine Quelle im Formular an und schickt Zuordnung als `input`", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(
      (await screen.findAllByRole("button", { name: "Neue Quelle" }))[0] as HTMLElement,
    );
    await user.type(screen.getByRole("textbox", { name: "Name" }), "Projekte");
    await user.type(
      screen.getByRole("textbox", { name: "Notion-Datenbank" }),
      "396f66270f5d8034b55cebc685aa5e50",
    );
    // Ein Feld mit Vorschlagsliste ist in ARIA eine combobox, kein textbox.
    await user.type(screen.getByLabelText("Spalte 1"), "Name");
    await user.click(screen.getByRole("button", { name: "Speichern" }));

    await waitFor(() => expect(commands("source_create")).toHaveLength(1));
    expect(commands("source_create")[0]?.args).toEqual({
      input: {
        name: "Projekte",
        database_id: "396f66270f5d8034b55cebc685aa5e50",
        mappings: [{ role: "title", property: "Name" }],
      },
    });
  });

  it("holt die Spalten von Notion und füllt die Zuordnung vor", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(
      (await screen.findAllByRole("button", { name: "Neue Quelle" }))[0] as HTMLElement,
    );
    await user.type(
      screen.getByRole("textbox", { name: "Notion-Datenbank" }),
      "396f66270f5d8034b55cebc685aa5e50",
    );
    await user.click(screen.getByRole("button", { name: "Spalten holen" }));

    await waitFor(() => expect(commands("database_inspect")).toHaveLength(1));
    expect(commands("database_inspect")[0]?.args).toEqual({
      database: "396f66270f5d8034b55cebc685aa5e50",
    });

    // Der Name der Datenbank steht im leeren Namensfeld, die Zuordnung kommt
    // aus core — drei Zeilen statt einer leeren.
    const name = (await screen.findByRole("textbox", { name: "Name" })) as HTMLInputElement;
    await waitFor(() => expect(name.value).toBe("vizu Projekte"));
    const spalte = (index: number) =>
      screen.getByRole("combobox", { name: `Spalte ${index}` }) as HTMLSelectElement;
    expect(spalte(1).value).toBe("Start");
    expect(spalte(2).value).toBe("Nächstes");
    expect(spalte(3).value).toBe("Name");
    // Die Spalten stehen zur Auswahl, statt abgetippt zu werden.
    expect(within(spalte(1)).getByRole("option", { name: "Start (date)" })).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Speichern" }));

    await waitFor(() => expect(commands("source_create")).toHaveLength(1));
    expect(commands("source_create")[0]?.args).toEqual({
      input: {
        name: "vizu Projekte",
        database_id: "396f66270f5d8034b55cebc685aa5e50",
        mappings: [
          { role: "date", property: "Start" },
          { role: "next", property: "Nächstes" },
          { role: "title", property: "Name" },
        ],
      },
    });
  });

  it("zeigt einen Fehler an der Rolle, zu der er gehört", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(
      (await screen.findAllByRole("button", { name: "Neue Quelle" }))[0] as HTMLElement,
    );
    nextError = {
      code: "validation_failed",
      message: "mappings.title: Spaltenname fehlt",
      fields: [{ field: "mappings.title", message: "Spaltenname fehlt" }],
    };
    await user.click(screen.getByRole("button", { name: "Speichern" }));

    expect(await screen.findByText("Spaltenname fehlt")).toBeTruthy();
    expect(document.querySelector(".banner")).toBeNull();
  });

  it("speichert einen Token, ohne ihn je anzuzeigen", async () => {
    const user = userEvent.setup();
    token = { origin: null, hint: null, store: "Schlüsselbund", store_error: null };
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Einstellungen" }));
    const feld = screen.getByLabelText("Neuer Token");
    await user.type(feld, "ntn_1234567890abcdefghijklmnopqrstuv");
    await user.click(
      within(feld.closest("fieldset") as HTMLElement).getByRole("button", { name: "Speichern" }),
    );

    await waitFor(() => expect(commands("token_set")).toHaveLength(1));
    expect(commands("token_set")[0]?.args).toEqual({
      token: "ntn_1234567890abcdefghijklmnopqrstuv",
    });
    expect(feld.getAttribute("type")).toBe("password");
  });

  it("zeichnet im Editor eine Vorschau aus dem ungespeicherten Text", async () => {
    const user = userEvent.setup();
    templates = [template("t1", "Fahrplan")];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Neu" }));
    const textfeld = await screen.findByRole("textbox", { name: "Vorlage" });
    await user.clear(textfeld);
    await user.type(textfeld, "x");

    // Die Vorschau läuft verzögert und über denselben Befehl wie das Diagramm.
    await waitFor(() => expect(commands("diagram_preview").length).toBeGreaterThan(0), {
      timeout: 2000,
    });
    expect(commands("template_save")).toHaveLength(0);
  });

  it("speichert eine Vorlage mit Kurznamen", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Neu" }));
    await user.type(await screen.findByRole("textbox", { name: "Kurzname" }), "fahrplan");
    await user.click(screen.getByRole("button", { name: "Speichern" }));

    await waitFor(() => expect(commands("template_save")).toHaveLength(1));
    const args = commands("template_save")[0]?.args as { id: string | null; input: TemplateInput };
    expect(args.id).toBeNull();
    expect(args.input.slug).toBe("fahrplan");
  });

  it("schreibt das SVG in die Datei aus dem Speichern-Dialog", async () => {
    const user = userEvent.setup();
    templates = [template("t1", "Fahrplan")];
    saved.path = "/tmp/fahrplan.svg";
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Fahrplan — Mermaid" }));
    await user.click(await screen.findByRole("button", { name: "SVG speichern" }));

    await waitFor(() => expect(commands("export_svg")).toHaveLength(1));
    expect(commands("export_svg")[0]?.args).toMatchObject({ path: "/tmp/fahrplan.svg" });
    expect(await screen.findByText("Gespeichert: /tmp/fahrplan.svg")).toBeTruthy();
  });

  it("schreibt nichts, wenn der Dialog abgebrochen wird", async () => {
    const user = userEvent.setup();
    templates = [template("t1", "Fahrplan")];
    saved.path = null;
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Fahrplan — Mermaid" }));
    await user.click(await screen.findByRole("button", { name: "SVG speichern" }));

    await waitFor(() => expect(commands("diagram_render")).toHaveLength(1));
    expect(commands("export_svg")).toHaveLength(0);
  });

  it("löscht ein Diagramm erst nach der Rückfrage", async () => {
    const user = userEvent.setup();
    templates = [template("t1", "Fahrplan")];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Fahrplan — Mermaid" }));
    await user.click(await screen.findByRole("button", { name: "Bearbeiten" }));
    await user.click(await screen.findByRole("button", { name: "Löschen" }));
    expect(commands("template_delete")).toHaveLength(0);

    await user.click(screen.getByRole("button", { name: "Endgültig löschen" }));
    await waitFor(() => expect(commands("template_delete")[0]?.args).toEqual({ id: "t1" }));
  });

  it("löscht eine Quelle erst nach der Rückfrage", async () => {
    const user = userEvent.setup();
    sources = [overview("a", "Projekte", 12)];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /Projekte/ }));
    await user.click(await screen.findByRole("button", { name: "Bearbeiten" }));
    await user.click(await screen.findByRole("button", { name: "Löschen" }));
    expect(commands("source_delete")).toHaveLength(0);

    await user.click(screen.getByRole("button", { name: "Endgültig löschen" }));
    await waitFor(() => expect(commands("source_delete")[0]?.args).toEqual({ id: "a" }));
  });

  it("zeichnet den Fluss einer Quelle ohne Vorlage", async () => {
    const user = userEvent.setup();
    sources = [{ ...overview("a", "Projekte", 12), views: ["flow"] }];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Projekte — Fluss" }));

    await waitFor(() => expect(commands("flow_render")).toHaveLength(1));
    expect(commands("flow_render")[0]?.args).toEqual({
      id: "a",
      hidden: [],
      subtitle: null,
    });
    // Einmal im Diagramm gezeichnet, einmal als Eintrag im Filterfeld.
    expect(await screen.findByRole("img", { name: /Flussdiagramm/ })).toBeTruthy();
    expect(screen.getAllByText("Website").length).toBeGreaterThan(1);
    expect(screen.getByRole("checkbox", { name: /Launch/ })).toBeTruthy();
  });

  it("zeichnet den Fluss mit anderer zweiter Zeile neu", async () => {
    const user = userEvent.setup();
    sources = [{ ...overview("a", "Projekte", 12), views: ["flow"] }];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Projekte — Fluss" }));
    await screen.findByRole("img", { name: /Flussdiagramm/ });
    await user.selectOptions(screen.getByLabelText("Zweite Zeile"), "status");

    await waitFor(() => expect(commands("flow_render")).toHaveLength(2));
    expect(commands("flow_render")[1]?.args).toMatchObject({ subtitle: "status" });
  });

  it("zeichnet die Metro-Karte und nennt Seiten ohne Datum", async () => {
    const user = userEvent.setup();
    sources = [{ ...overview("a", "Projekte", 12), views: ["flow", "metro"] }];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Projekte — Metro" }));

    await waitFor(() => expect(commands("metro_render")).toHaveLength(1));
    expect(commands("metro_render")[0]?.args).toEqual({ id: "a", hidden: [] });
    expect(await screen.findByRole("img", { name: /Metro-Karte/ })).toBeTruthy();
    // Was kein Datum hat, steht in keiner Linie — die Karte sagt das.
    expect(screen.getByText(/Ohne Datum.*Ohne Ziel/)).toBeTruthy();
  });

  it("versteckt ein Diagramm und holt es zurück", async () => {
    const user = userEvent.setup();
    templates = [template("t1", "Fahrplan")];
    sources = [{ ...overview("a", "Projekte", 12), views: ["flow", "metro"] }];
    render(<App />);

    const liste = await screen.findByRole("navigation", { name: "Diagramme" });
    expect(within(liste).getByRole("button", { name: "Projekte — Fluss" })).toBeTruthy();

    await user.click(within(liste).getByRole("button", { name: "Projekte — Fluss verstecken" }));

    await waitFor(() => expect(commands("hidden_set")).toHaveLength(1));
    expect(commands("hidden_set")[0]?.args).toEqual({
      entry: { kind: "flow", target: "a" },
      hidden: true,
    });
    // Aus der Liste heraus, aber nicht weg: Der Bereich „Versteckt" hat ihn.
    const versteckt = within(liste).getByText("Versteckt").closest("details") as HTMLElement;
    expect(within(versteckt).getByRole("button", { name: "Projekte — Fluss" })).toBeTruthy();
    // Metro und die Vorlage stehen weiter oben in der Liste.
    expect(within(liste).getByRole("button", { name: "Projekte — Metro" })).toBeTruthy();

    await user.click(
      within(versteckt).getByRole("button", { name: "Projekte — Fluss einblenden" }),
    );

    await waitFor(() => expect(commands("hidden_set")).toHaveLength(2));
    expect(commands("hidden_set")[1]?.args).toEqual({
      entry: { kind: "flow", target: "a" },
      hidden: false,
    });
    expect(within(liste).queryByText("Versteckt")).toBeNull();
  });

  it("schließt das Diagramm, das gerade versteckt wird", async () => {
    const user = userEvent.setup();
    sources = [{ ...overview("a", "Projekte", 12), views: ["metro"] }];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Projekte — Metro" }));
    expect(await screen.findByRole("img", { name: /Metro-Karte/ })).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Projekte — Metro verstecken" }));

    await waitFor(() => expect(screen.queryByRole("img", { name: /Metro-Karte/ })).toBeNull());
  });
});
