// Die einzige Stelle, an der die Oberfläche mit Rust spricht.
//
// Jeder Befehl steht hier genau einmal, mit Typen aus bindings.ts. Eine
// Komponente importiert diese Datei nicht — sie gibt einen Rückruf nach oben,
// und App.tsx ruft. crates/core/tests/layering.rs bewacht beides.
//
// NEUER BEFEHL? Drei Stellen:
//   1. crates/desktop/src/commands.rs     #[tauri::command] async fn …
//   2. crates/desktop/src/lib.rs          generate_handler![…]
//   3. hier                               call<…>("name", { … })
// crates/desktop/tests/ipc_contract.rs prüft, dass keine fehlt.
//
// Argumentnamen: Tauri macht aus einem Rust-Argument `source_id` in JavaScript
// `sourceId`. Die Befehle hier haben deshalb nur einwortige Argumente.

import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import type {
  ApiError as ApiErrorData,
  AppInfo,
  Config,
  DatabaseSchema,
  Diagram,
  ErrorCode,
  FetchStatus,
  FieldError,
  FlowGraph,
  HiddenDiagram,
  MetroMap,
  Property,
  Source,
  SourceInput,
  SourceOverview,
  Template,
  TemplateHelp,
  TemplateInput,
  TokenStatus,
  View,
  ViewInput,
} from "./bindings";

/** Ein abgelehnter Befehl. Dieselbe Form wie `error` im JSON der CLI. */
export class ApiError extends Error {
  readonly code: ErrorCode;
  readonly fields: FieldError[];

  constructor(data: ApiErrorData) {
    super(data.message);
    this.name = "ApiError";
    this.code = data.code;
    this.fields = data.fields ?? [];
  }

  /** Eingabefehler gehören ans Feld, alles andere oben ins Fenster. */
  get isValidation(): boolean {
    return this.fields.length > 0;
  }

  /** Die Meldung zu einem Feld, falls es eine gibt. */
  fieldMessage(field: string): string | undefined {
    return this.fields.find((f) => f.field === field)?.message;
  }

  /**
   * Macht aus allem, was ein Promise ablehnen kann, einen ApiError.
   *
   * Rust schickt ein Objekt mit `code`. Tauri selbst schickt bei einem
   * unbekannten Befehl oder fehlenden Argument nur einen Text.
   */
  static from(raw: unknown): ApiError {
    if (raw instanceof ApiError) return raw;
    if (raw && typeof raw === "object" && "code" in raw && "message" in raw) {
      return new ApiError(raw as ApiErrorData);
    }
    return new ApiError({ code: "internal_error", message: String(raw) });
  }
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (raw) {
    throw ApiError.from(raw);
  }
}

export const api = {
  sources: {
    list: () => call<SourceOverview[]>("source_list"),
    get: (id: string) => call<Source>("source_get", { id }),
    /** Dauert Sekunden: Rust holt die Daten von Notion und speichert sie. */
    fetch: (id: string) => call<FetchStatus>("source_fetch", { id }),
    create: (input: SourceInput) => call<Source>("source_create", { input }),
    update: (id: string, input: SourceInput) => call<Source>("source_update", { id, input }),
    remove: (id: string) => call<null>("source_delete", { id }),
    /** Die Spalten aus dem letzten Abruf — für die Auswahl beim Zuordnen. */
    properties: (id: string) => call<Property[]>("source_properties", { id }),
    /** Der Fluss einer Quelle: Kanten entlang `next`, fertig angeordnet. */
    flow: (id: string, hidden: string[], subtitle: string | null) =>
      call<FlowGraph>("flow_render", { id, hidden, subtitle }),
    /** Die Metro-Karte: Linien auf einer Zeitachse aus `date`. */
    metro: (id: string, hidden: string[]) => call<MetroMap>("metro_render", { id, hidden }),
    /**
     * Die Spalten einer Notion-Datenbank, live — für den Dialog beim Anlegen,
     * bevor es einen Abruf gibt. Braucht Netz und Token.
     */
    inspect: (database: string) => call<DatabaseSchema>("database_inspect", { database }),
  },

  /** Diagramme, die in der Liste nicht auftauchen sollen. */
  hidden: {
    list: () => call<HiddenDiagram[]>("hidden_list"),
    /** Gibt die neue Liste zurück, damit die Oberfläche nicht nachfragt. */
    set: (entry: HiddenDiagram, hidden: boolean) =>
      call<HiddenDiagram[]>("hidden_set", { entry, hidden }),
  },

  /** Gespeicherte Ansichten: ein Diagramm samt Einstellungen. */
  views: {
    list: () => call<View[]>("view_list"),
    /** `id` leer lassen heißt: neu anlegen. */
    save: (id: string | null, input: ViewInput) => call<View>("view_save", { id, input }),
    remove: (id: string) => call<null>("view_delete", { id }),
  },

  templates: {
    list: () => call<Template[]>("template_list"),
    get: (id: string) => call<Template>("template_get", { id }),
    /** `id` leer lassen heißt: neu anlegen. */
    save: (id: string | null, input: TemplateInput) =>
      call<Template>("template_save", { id, input }),
    remove: (id: string) => call<null>("template_delete", { id }),
    /** Zeichnet aus dem Zwischenspeicher — ohne Netz, deshalb schnell. */
    render: (id: string, hidden: string[]) => call<Diagram>("diagram_render", { id, hidden }),
    /** Beispiele und Spickzettel — sie stehen in core, nicht im Webview. */
    help: () => call<TemplateHelp>("template_help"),
    /** Dasselbe für einen Text, der noch nicht gespeichert ist. */
    preview: (body: string, hidden: string[]) => call<Diagram>("diagram_preview", { body, hidden }),
  },

  token: {
    /** Der Token selbst kommt nie aus Rust zurück, nur sein Hinweis. */
    status: () => call<TokenStatus>("token_status"),
    set: (token: string) => call<TokenStatus>("token_set", { token }),
    clear: () => call<TokenStatus>("token_clear"),
  },

  config: {
    get: () => call<Config>("config_get"),
    set: (config: Config) => call<Config>("config_set", { config }),
  },

  appInfo: () => call<AppInfo>("app_info"),

  /**
   * Fragt nach einem Dateinamen und schreibt das SVG.
   *
   * `null`, wenn der Dialog abgebrochen wurde. Ein Download aus dem Webview
   * heraus ginge nicht — die Datei schreibt Rust.
   */
  exportSvg: async (svg: string, suggested: string): Promise<string | null> => {
    const path = await save({
      defaultPath: `${suggested}.svg`,
      filters: [{ name: "SVG", extensions: ["svg"] }],
    });
    if (!path) return null;
    return call<string>("export_svg", { path, svg });
  },

  /**
   * Öffnet eine Adresse im Standardbrowser — nie im Fenster der Anwendung.
   * Welche Schemata erlaubt sind, steht in crates/desktop/capabilities/.
   */
  openExternal: async (url: string) => {
    try {
      await openUrl(url);
    } catch (raw) {
      throw ApiError.from(raw);
    }
  },
};

export type Api = typeof api;
