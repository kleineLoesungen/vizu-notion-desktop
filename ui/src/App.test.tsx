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
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { App } from "./App";
import type { ApiError, Config, FetchStatus, SourceOverview, TokenStatus } from "./bindings";

type Call = { cmd: string; args: Record<string, unknown> };

class Rejection {
  constructor(readonly error: ApiError) {}
}

const STAMP = "2026-09-16T06:39:51Z";

function overview(id: string, name: string, pages?: number): SourceOverview {
  return {
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

let sources: SourceOverview[];
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
  calls = [];
  nextError = null;
  config = { app_name: "Vizu Notion", theme: "light", accent: "#3b6ea5" };
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
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("sagt ohne Quellen, wie man eine anlegt", async () => {
    render(<App />);
    expect(await screen.findByText("Willkommen")).toBeTruthy();
    expect(screen.getByText(/source add Projekte/)).toBeTruthy();
  });

  it("weist auf den fehlenden Token hin", async () => {
    token = { origin: null, hint: null, store: "Schlüsselbund", store_error: null };
    render(<App />);
    expect(await screen.findByText(/Kein Notion-Token hinterlegt/)).toBeTruthy();
    expect(screen.getByText("vizu-notion token set")).toBeTruthy();
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
});
