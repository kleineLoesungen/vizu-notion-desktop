// Die einzige Komponente, die api.ts benutzt.
//
// Hier liegt der Zustand der Oberfläche, und hier wird aus einem Wunsch einer
// Komponente ein Befehl an Rust:
//
//   Komponente  ──onFetch()──▶  App.tsx  ──api.sources.fetch()──▶  Rust
//
// Fachliche Regeln stehen hier nicht. Was eine Vorlage zeichnet und welche
// Knoten dabei wegfallen, entscheidet crates/core — hier wird angezeigt, was
// von dort zurückkommt.

import { useCallback, useEffect, useState } from "react";
import { ApiError, api } from "./api";
import type { AppInfo, Config, Diagram, SourceOverview, Template, TokenStatus } from "./bindings";
import { DiagramView } from "./components/DiagramView";
import { EmptyState } from "./components/EmptyState";
import { FilterPanel } from "./components/FilterPanel";
import { SettingsDialog } from "./components/SettingsDialog";
import { SourceDetail } from "./components/SourceDetail";
import { SourceList } from "./components/SourceList";
import { TemplateList } from "./components/TemplateList";
import { withNeighbours } from "./lib/graph";
import { applyTheme, useIsDark } from "./lib/theme";

type Selection =
  | { kind: "none" }
  | { kind: "source"; id: string }
  | { kind: "template"; id: string };

export function App() {
  const [sources, setSources] = useState<SourceOverview[]>([]);
  const [templates, setTemplates] = useState<Template[]>([]);
  const [selection, setSelection] = useState<Selection>({ kind: "none" });

  const [diagram, setDiagram] = useState<Diagram | null>(null);
  const [hidden, setHidden] = useState<Set<string>>(new Set());
  const [drawing, setDrawing] = useState(false);

  const [busyId, setBusyId] = useState<string | null>(null);
  const [fetchingAll, setFetchingAll] = useState(false);

  const [token, setToken] = useState<TokenStatus | null>(null);
  const [config, setConfig] = useState<Config | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);

  // Zwei Sorten Fehler, wie überall im Kit: mit Feldern an die Felder,
  // ohne Felder als Meldung oben.
  const [fieldError, setFieldError] = useState<ApiError | null>(null);
  const [banner, setBanner] = useState<string | null>(null);

  const dark = useIsDark(config?.theme ?? "system");

  /** Ein fehlgeschlagener Befehl: ans Feld oder nach oben. */
  const fail = useCallback((raw: unknown) => {
    const err = ApiError.from(raw);
    if (err.isValidation) {
      setFieldError(err);
    } else {
      setBanner(err.message);
    }
  }, []);

  const reload = useCallback(async () => {
    try {
      const [overview, list] = await Promise.all([api.sources.list(), api.templates.list()]);
      setSources(overview);
      setTemplates(list);
    } catch (raw) {
      fail(raw);
    }
  }, [fail]);

  /** Zeichnen lassen — jedes Mal in Rust, damit die Regeln dort bleiben. */
  const draw = useCallback(
    async (id: string, hide: Set<string>) => {
      setDrawing(true);
      try {
        setDiagram(await api.templates.render(id, [...hide]));
        setBanner(null);
      } catch (raw) {
        setDiagram(null);
        fail(raw);
      } finally {
        setDrawing(false);
      }
    },
    [fail],
  );

  // --- Start ---------------------------------------------------------------
  // biome-ignore lint/correctness/useExhaustiveDependencies: nur einmal beim Start
  useEffect(() => {
    void (async () => {
      try {
        const [loadedConfig, appInfo, tokenStatus] = await Promise.all([
          api.config.get(),
          api.appInfo(),
          api.tokenStatus(),
        ]);
        setConfig(loadedConfig);
        setInfo(appInfo);
        setToken(tokenStatus);
      } catch (raw) {
        fail(raw);
      }
      await reload();
    })();
  }, []);

  useEffect(() => {
    if (!config) return;
    applyTheme(config);
    document.title = config.app_name;
    api.setWindowTitle(config.app_name).catch(() => {
      // Ohne Fenster (Tests, Browser) gibt es keinen Titel zu setzen.
    });
  }, [config]);

  // --- Auswahl -------------------------------------------------------------

  function selectSource(id: string) {
    setSelection({ kind: "source", id });
    setDiagram(null);
  }

  function selectTemplate(id: string) {
    setSelection({ kind: "template", id });
    // Ausgeblendete Knoten gehören zur Vorlage, nicht zur Anwendung.
    setHidden(new Set());
    void draw(id, new Set());
  }

  /** Neu zeichnen mit geänderter Auswahl. */
  function changeHidden(next: Set<string>) {
    setHidden(next);
    if (selection.kind === "template") void draw(selection.id, next);
  }

  function toggleNode(id: string) {
    const next = new Set(hidden);
    if (!next.delete(id)) next.add(id);
    changeHidden(next);
  }

  function onlyRelated(id: string) {
    const keep = withNeighbours(diagram?.nodes ?? [], id);
    changeHidden(new Set((diagram?.nodes ?? []).map((n) => n.id).filter((n) => !keep.has(n))));
  }

  function setSourceVisible(source: string, visible: boolean) {
    const next = new Set(hidden);
    for (const node of diagram?.nodes ?? []) {
      if (node.source !== source) continue;
      if (visible) {
        next.delete(node.id);
      } else {
        next.add(node.id);
      }
    }
    changeHidden(next);
  }

  // --- Quellen -------------------------------------------------------------

  async function fetchOne(id: string) {
    setBusyId(id);
    setBanner(null);
    try {
      await api.sources.fetch(id);
      await reload();
      setToken(await api.tokenStatus());
      // Das Diagramm zeigt jetzt veraltete Daten.
      if (selection.kind === "template") await draw(selection.id, hidden);
    } catch (raw) {
      fail(raw);
    } finally {
      setBusyId(null);
    }
  }

  async function fetchAll() {
    setFetchingAll(true);
    setBanner(null);
    try {
      // Nacheinander: Notion begrenzt die Anfragen ohnehin auf drei je Sekunde.
      for (const { source } of sources) {
        setBusyId(source.id);
        await api.sources.fetch(source.id);
      }
      await reload();
      if (selection.kind === "template") await draw(selection.id, hidden);
    } catch (raw) {
      fail(raw);
      await reload();
    } finally {
      setBusyId(null);
      setFetchingAll(false);
    }
  }

  async function saveConfig(next: Config) {
    try {
      setConfig(await api.config.set(next));
      setFieldError(null);
      setSettingsOpen(false);
    } catch (raw) {
      fail(raw);
    }
  }

  function openLink(url: string) {
    api.openExternal(url).catch(fail);
  }

  // --- Zeichnen ------------------------------------------------------------

  const selectedSource =
    selection.kind === "source" ? sources.find((s) => s.source.id === selection.id) : undefined;
  const selectedTemplate =
    selection.kind === "template" ? templates.find((t) => t.id === selection.id) : undefined;
  const fieldMessage = (field: string) => fieldError?.fieldMessage(field);

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">{config?.app_name ?? " "}</div>
        <SourceList
          sources={sources}
          selectedId={selection.kind === "source" ? selection.id : null}
          busy={fetchingAll}
          onSelect={selectSource}
          onFetchAll={() => void fetchAll()}
        />
        <TemplateList
          templates={templates}
          selectedId={selection.kind === "template" ? selection.id : null}
          onSelect={selectTemplate}
        />
        <button
          type="button"
          className="ghost settings-button"
          disabled={!config}
          onClick={() => {
            setFieldError(null);
            setSettingsOpen(true);
          }}
        >
          Einstellungen
        </button>
      </aside>

      <main className="main">
        {banner && (
          <div className="banner" role="alert">
            <span>{banner}</span>
            <button type="button" className="ghost" onClick={() => setBanner(null)}>
              Schließen
            </button>
          </div>
        )}

        {token && !token.origin && (
          <div className="banner" role="status">
            <span>
              Kein Notion-Token hinterlegt ({token.store}). Speichern mit:{" "}
              <code>vizu-notion token set</code>
            </span>
          </div>
        )}

        {selectedSource && (
          <SourceDetail
            entry={selectedSource}
            busy={busyId === selectedSource.source.id}
            onFetch={() => void fetchOne(selectedSource.source.id)}
            onOpenLink={openLink}
          />
        )}

        {selectedTemplate && (
          <section className="diagram-page">
            <header className="detail-head">
              <h1>{selectedTemplate.title}</h1>
              <span className="muted">
                {drawing ? "Zeichne …" : `Quellen: ${selectedTemplate.sources.join(", ")}`}
              </span>
            </header>
            {diagram && <DiagramView mermaid={diagram.mermaid} dark={dark} />}
          </section>
        )}

        {selection.kind === "none" && (
          <EmptyState hasSources={sources.length > 0} hasTemplates={templates.length > 0} />
        )}
      </main>

      {selectedTemplate && diagram && (
        <FilterPanel
          nodes={diagram.nodes}
          hidden={hidden}
          onToggle={toggleNode}
          onOnlyRelated={onlyRelated}
          onSetSource={setSourceVisible}
          onReset={() => changeHidden(new Set())}
        />
      )}

      {settingsOpen && config && (
        <SettingsDialog
          config={config}
          info={info}
          fieldMessage={fieldMessage}
          onSave={(next) => void saveConfig(next)}
          onClose={() => {
            setFieldError(null);
            setSettingsOpen(false);
          }}
        />
      )}
    </div>
  );
}
