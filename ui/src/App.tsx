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
import type {
  AppInfo,
  Config,
  DatabaseSchema,
  Diagram,
  FlowGraph,
  HiddenDiagram,
  MetroMap,
  Property,
  Source,
  SourceInput,
  SourceOverview,
  Template,
  TemplateHelp,
  TokenStatus,
  View,
} from "./bindings";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { type DiagramChoice, DiagramList } from "./components/DiagramList";
import { DiagramView } from "./components/DiagramView";
import { EmptyState } from "./components/EmptyState";
import { FilterPanel } from "./components/FilterPanel";
import { FlowView } from "./components/FlowView";
import { GettingStarted } from "./components/GettingStarted";
import { MetroView } from "./components/MetroView";
import { SaveViewDialog } from "./components/SaveViewDialog";
import { SettingsDialog } from "./components/SettingsDialog";
import { SourceDetail } from "./components/SourceDetail";
import { SourceDialog } from "./components/SourceDialog";
import { SourceList } from "./components/SourceList";
import { SourceRefresh } from "./components/SourceRefresh";
import { TemplateEditor } from "./components/TemplateEditor";
import { ViewList } from "./components/ViewList";
import { withNeighbours } from "./lib/graph";
import { applyTheme, useIsDark } from "./lib/theme";

const DELETE_TITLE = {
  source: "Quelle löschen",
  template: "Diagramm löschen",
  view: "Ansicht löschen",
} as const;

/** Was verloren geht — und was bleibt. */
function deleteMessage(what: { kind: keyof typeof DELETE_TITLE; name: string }): string {
  const name = `„${what.name}“`;
  switch (what.kind) {
    case "source":
      return `${name} wird gelöscht, mitsamt den abgerufenen Seiten. Vorlagen, die diese Quelle benutzen, zeichnen danach nicht mehr.`;
    case "template":
      return `${name} wird gelöscht. Die Daten der Quellen bleiben.`;
    default:
      return `${name} wird gelöscht. Das Diagramm selbst bleibt — nur die gespeicherten Einstellungen sind weg.`;
  }
}

type Selection =
  | { kind: "none" }
  | { kind: "source"; id: string }
  | { kind: "template"; id: string }
  /** Eine Ansicht aus der Zuordnung einer Quelle. */
  | { kind: "flow"; id: string }
  | { kind: "metro"; id: string };

/** Die Vorlage im Editor. `id` leer heißt: noch nicht gespeichert. */
type Draft = { id: string | null; slug: string; body: string; saved: string };

/** Womit eine neue Vorlage anfängt — zum Überschreiben gedacht. */
const NEW_TEMPLATE = `---
title: "Neues Diagramm"
sources:
  - QUELLE
---
flowchart TD
{{#each QUELLE}}
  {{title}}
{{/each}}
`;

/**
 * Der Name der Anwendung — aus `ui/index.html`, wo auch der Fenstertitel
 * herkommt. Eine Einstellung dafür gab es einmal; sie ergab keinen Sinn.
 */
const APP_NAME = document.title;

/** So lange nach dem letzten Tastendruck wird gewartet, bevor neu gezeichnet wird. */
const PREVIEW_DELAY_MS = 400;

export function App() {
  const [sources, setSources] = useState<SourceOverview[]>([]);
  const [templates, setTemplates] = useState<Template[]>([]);
  /** Diagramme, die aus der Liste in den Bereich „Versteckt" gerutscht sind. */
  const [hiddenDiagrams, setHiddenDiagrams] = useState<HiddenDiagram[]>([]);
  /** Gespeicherte Ansichten — ein Diagramm samt Einstellungen. */
  const [views, setViews] = useState<View[]>([]);
  /** Welche Ansicht gerade offen ist; `null`, sobald man etwas anderes wählt. */
  const [activeView, setActiveView] = useState<string | null>(null);
  const [saveViewOpen, setSaveViewOpen] = useState(false);
  const [selection, setSelection] = useState<Selection>({ kind: "none" });

  const [diagram, setDiagram] = useState<Diagram | null>(null);
  const [flow, setFlow] = useState<FlowGraph | null>(null);
  const [metro, setMetro] = useState<MetroMap | null>(null);
  /** Welche Rolle im Fluss unter dem Titel steht. */
  const [subtitle, setSubtitle] = useState<string | null>(null);
  const [hidden, setHidden] = useState<Set<string>>(new Set());
  const [drawing, setDrawing] = useState(false);

  const [busyId, setBusyId] = useState<string | null>(null);
  const [fetchingAll, setFetchingAll] = useState(false);

  const [token, setToken] = useState<TokenStatus | null>(null);
  const [config, setConfig] = useState<Config | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [sourceDialog, setSourceDialog] = useState<{ source: Source | null } | null>(null);
  const [properties, setProperties] = useState<Property[]>([]);
  /** Spalten, die für den Dialog frisch bei Notion erfragt wurden. */
  const [schema, setSchema] = useState<DatabaseSchema | null>(null);
  const [inspecting, setInspecting] = useState(false);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [help, setHelp] = useState<TemplateHelp | null>(null);
  /** Eine Löschung, die noch bestätigt werden muss. */
  const [confirmDelete, setConfirmDelete] = useState<
    | { kind: "source"; name: string }
    | { kind: "template"; name: string }
    | { kind: "view"; name: string; id: string }
    | null
  >(null);

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
      const [overview, list, gone, saved] = await Promise.all([
        api.sources.list(),
        api.templates.list(),
        api.hidden.list(),
        api.views.list(),
      ]);
      setSources(overview);
      setTemplates(list);
      setHiddenDiagrams(gone);
      setViews(saved);
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
        const [loadedConfig, appInfo, tokenStatus, templateHelp] = await Promise.all([
          api.config.get(),
          api.appInfo(),
          api.token.status(),
          api.templates.help(),
        ]);
        setConfig(loadedConfig);
        setInfo(appInfo);
        setToken(tokenStatus);
        setHelp(templateHelp);
      } catch (raw) {
        fail(raw);
      }
      await reload();
    })();
  }, []);

  useEffect(() => {
    if (config) applyTheme(config);
  }, [config]);

  // Die Vorschau im Editor: verzögert, damit nicht bei jedem Tastendruck
  // gezeichnet wird, und veraltete Läufe werden verworfen.
  useEffect(() => {
    if (!draft) return;
    let current = true;
    const timer = window.setTimeout(() => {
      void (async () => {
        try {
          const preview = await api.templates.preview(draft.body, []);
          if (current) {
            setDiagram(preview);
            setFieldError(null);
          }
        } catch (raw) {
          if (!current) return;
          setDiagram(null);
          fail(raw);
        }
      })();
    }, PREVIEW_DELAY_MS);
    return () => {
      current = false;
      window.clearTimeout(timer);
    };
  }, [draft, fail]);

  // --- Auswahl -------------------------------------------------------------

  function selectSource(id: string) {
    setSelection({ kind: "source", id });
    setDraft(null);
    setDiagram(null);
    setFlow(null);
    setMetro(null);
  }

  /** Die Ansicht aus der Zuordnung. Gezeichnet wird wieder in Rust. */
  const drawFlow = useCallback(
    async (id: string, hide: Set<string>, role: string | null) => {
      try {
        setFlow(await api.sources.flow(id, [...hide], role));
        setBanner(null);
      } catch (raw) {
        setFlow(null);
        fail(raw);
      }
    },
    [fail],
  );

  const drawMetro = useCallback(
    async (id: string, hide: Set<string>) => {
      try {
        setMetro(await api.sources.metro(id, [...hide]));
        setBanner(null);
      } catch (raw) {
        setMetro(null);
        fail(raw);
      }
    },
    [fail],
  );

  /** Ein Eintrag aus der Diagrammliste — Vorlage, Fluss oder Metro. */
  function selectDiagram(choice: DiagramChoice) {
    setActiveView(null);
    setSelection(choice);
    setDraft(null);
    setDiagram(null);
    setFlow(null);
    setMetro(null);
    setHidden(new Set());
    if (choice.kind === "template") {
      void draw(choice.id, new Set());
    } else if (choice.kind === "flow") {
      setSubtitle(null);
      void drawFlow(choice.id, new Set(), null);
    } else {
      void drawMetro(choice.id, new Set());
    }
  }

  /** Ein Diagramm aus der Liste nehmen oder zurückholen. */
  async function hideDiagram(choice: DiagramChoice, hide: boolean) {
    try {
      setHiddenDiagrams(await api.hidden.set({ kind: choice.kind, target: choice.id }, hide));
      // Was versteckt wird, bleibt nicht offen stehen.
      if (hide && selection.kind === choice.kind && selection.id === choice.id) {
        setSelection({ kind: "none" });
        setDiagram(null);
        setFlow(null);
        setMetro(null);
      }
    } catch (raw) {
      fail(raw);
    }
  }

  /** Neu zeichnen mit geänderter Auswahl. */
  function changeHidden(next: Set<string>) {
    setHidden(next);
    if (selection.kind === "template") void draw(selection.id, next);
    if (selection.kind === "flow") void drawFlow(selection.id, next, subtitle);
    if (selection.kind === "metro") void drawMetro(selection.id, next);
  }

  function toggleNode(id: string) {
    const next = new Set(hidden);
    if (!next.delete(id)) next.add(id);
    changeHidden(next);
  }

  /** Die Knotenliste der gerade sichtbaren Ansicht. */
  const nodes = flow?.all_nodes ?? metro?.all_nodes ?? diagram?.nodes ?? [];

  function onlyRelated(id: string) {
    const keep = withNeighbours(nodes, id);
    changeHidden(new Set(nodes.map((n) => n.id).filter((n) => !keep.has(n))));
  }

  function setSourceVisible(source: string, visible: boolean) {
    const next = new Set(hidden);
    for (const node of nodes) {
      if (node.source !== source) continue;
      if (visible) {
        next.delete(node.id);
      } else {
        next.add(node.id);
      }
    }
    changeHidden(next);
  }

  // --- Gespeicherte Ansichten ------------------------------------------------

  /** Stellt wieder her, was die Ansicht festhält. */
  function openView(view: View) {
    const hide = new Set(view.hidden);
    setActiveView(view.id);
    setSelection({ kind: view.kind, id: view.target } as Selection);
    setDraft(null);
    setDiagram(null);
    setFlow(null);
    setMetro(null);
    setHidden(hide);
    setSubtitle(view.subtitle);
    if (view.kind === "template") void draw(view.target, hide);
    if (view.kind === "flow") void drawFlow(view.target, hide, view.subtitle);
    if (view.kind === "metro") void drawMetro(view.target, hide);
  }

  async function saveView(name: string) {
    if (selection.kind === "none" || selection.kind === "source") return;
    setFieldError(null);
    try {
      const saved = await api.views.save(activeView, {
        name,
        kind: selection.kind,
        target: selection.id,
        hidden: [...hidden],
        subtitle: selection.kind === "flow" ? subtitle : null,
      });
      setViews(await api.views.list());
      setActiveView(saved.id);
      setSaveViewOpen(false);
    } catch (raw) {
      fail(raw);
    }
  }

  async function deleteView(id: string) {
    try {
      await api.views.remove(id);
      setViews(await api.views.list());
      if (activeView === id) setActiveView(null);
      setConfirmDelete(null);
    } catch (raw) {
      fail(raw);
      setConfirmDelete(null);
    }
  }

  // --- Quellen -------------------------------------------------------------

  async function fetchOne(id: string) {
    setBusyId(id);
    setBanner(null);
    try {
      await api.sources.fetch(id);
      await reload();
      setToken(await api.token.status());
      // Das Diagramm zeigt jetzt veraltete Daten.
      await redraw();
    } catch (raw) {
      fail(raw);
    } finally {
      setBusyId(null);
    }
  }

  /** Die Quellen, aus denen das gewählte Diagramm gezeichnet ist. */
  async function fetchForDiagram(ids: string[]) {
    setFetchingAll(true);
    setBanner(null);
    try {
      // Nacheinander: Notion begrenzt die Anfragen ohnehin auf drei je Sekunde.
      for (const id of ids) {
        setBusyId(id);
        await api.sources.fetch(id);
      }
      await reload();
      await redraw();
    } catch (raw) {
      fail(raw);
      await reload();
    } finally {
      setBusyId(null);
      setFetchingAll(false);
    }
  }

  /** Dasselbe noch einmal zeichnen, mit den frisch geholten Daten. */
  async function redraw() {
    if (selection.kind === "template") await draw(selection.id, hidden);
    if (selection.kind === "flow") await drawFlow(selection.id, hidden, subtitle);
    if (selection.kind === "metro") await drawMetro(selection.id, hidden);
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
      await redraw();
    } catch (raw) {
      fail(raw);
      await reload();
    } finally {
      setBusyId(null);
      setFetchingAll(false);
    }
  }

  // --- Verwalten -----------------------------------------------------------

  async function openSourceDialog(source: Source | null) {
    setFieldError(null);
    setSchema(null);
    setProperties(source ? await api.sources.properties(source.id).catch(() => []) : []);
    setSourceDialog({ source });
  }

  /** Die Spalten einer Datenbank bei Notion erfragen — für die Einrichtung. */
  async function inspectDatabase(databaseId: string) {
    setInspecting(true);
    setFieldError(null);
    setBanner(null);
    try {
      setSchema(await api.sources.inspect(databaseId));
    } catch (raw) {
      setSchema(null);
      fail(raw);
    } finally {
      setInspecting(false);
    }
  }

  async function saveSource(input: SourceInput) {
    const existing = sourceDialog?.source;
    try {
      const saved = existing
        ? await api.sources.update(existing.id, input)
        : await api.sources.create(input);
      setSourceDialog(null);
      setFieldError(null);
      await reload();
      selectSource(saved.id);
    } catch (raw) {
      fail(raw);
    }
  }

  async function deleteSource() {
    const existing = sourceDialog?.source;
    setConfirmDelete(null);
    if (!existing) return;
    try {
      await api.sources.remove(existing.id);
      setSourceDialog(null);
      setSelection({ kind: "none" });
      await reload();
    } catch (raw) {
      fail(raw);
    }
  }

  async function editTemplate(id: string | null) {
    setFieldError(null);
    setSelection({ kind: "none" });
    if (id === null) {
      setDraft({ id: null, slug: "", body: NEW_TEMPLATE, saved: "" });
      return;
    }
    try {
      const found = await api.templates.get(id);
      setDraft({ id, slug: found.slug, body: found.body, saved: found.body + found.slug });
    } catch (raw) {
      fail(raw);
    }
  }

  async function saveTemplate() {
    if (!draft) return;
    try {
      const saved = await api.templates.save(draft.id, { slug: draft.slug, body: draft.body });
      setDraft({ ...draft, id: saved.id, saved: saved.body + saved.slug });
      setFieldError(null);
      await reload();
    } catch (raw) {
      fail(raw);
    }
  }

  async function deleteTemplate() {
    setConfirmDelete(null);
    if (!draft?.id) return;
    try {
      await api.templates.remove(draft.id);
      setDraft(null);
      setDiagram(null);
      await reload();
    } catch (raw) {
      fail(raw);
    }
  }

  async function saveToken(token: string) {
    try {
      setToken(await api.token.set(token));
      setFieldError(null);
    } catch (raw) {
      fail(raw);
    }
  }

  async function clearToken() {
    try {
      setToken(await api.token.clear());
    } catch (raw) {
      fail(raw);
    }
  }

  async function exportSvg(svg: string) {
    const name =
      draft?.slug ||
      selectedTemplate?.slug ||
      (selection.kind === "flow" || selection.kind === "metro"
        ? `${selection.kind}-${sources.find((s) => s.source.id === selection.id)?.source.name ?? ""}`
        : "diagramm");
    try {
      const path = await api.exportSvg(svg, name);
      if (path) setBanner(`Gespeichert: ${path}`);
    } catch (raw) {
      fail(raw);
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
  /** Ein Name, den man nicht erst tippen muss. */
  const diagramTitle = (choice: DiagramChoice) => {
    if (choice.kind === "template") {
      return templates.find((t) => t.id === choice.id)?.title ?? "Ansicht";
    }
    const name = sources.find((s) => s.source.id === choice.id)?.source.name ?? "Ansicht";
    return choice.kind === "flow" ? `Fluss: ${name}` : `Metro: ${name}`;
  };
  /** Alle vier Schritte erledigt? Dann führt nichts mehr durch den Einstieg. */
  const ready = !!token?.origin && sources.some((s) => s.fetch !== null) && templates.length > 0;
  /** Quellname → Zeitpunkt des letzten Abrufs, für Filterfeld und Kopfzeile. */
  const fetchedAt: Record<string, string | null> = Object.fromEntries(
    sources.map(({ source, fetch }) => [source.name, fetch?.fetched_at ?? null]),
  );
  /** Eine Vorlage nennt ihre Quellen beim Namen, abgerufen wird über die Kennung. */
  const idsOf = (names: string[]) =>
    sources.filter(({ source }) => names.includes(source.name)).map(({ source }) => source.id);
  const namesOf = (id: string) => {
    const found = sources.find((s) => s.source.id === id)?.source.name;
    return found ? [found] : [];
  };

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">{APP_NAME}</div>
        <DiagramList
          templates={templates}
          sources={sources}
          selected={selection.kind === "none" || selection.kind === "source" ? null : selection}
          hiddenDiagrams={hiddenDiagrams}
          onSelect={selectDiagram}
          onHide={(choice, hide) => void hideDiagram(choice, hide)}
          onCreate={() => void editTemplate(null)}
        />
        <ViewList
          views={views}
          activeId={activeView}
          onOpen={openView}
          onDelete={(view) => setConfirmDelete({ kind: "view", name: view.name, id: view.id })}
        />
        {/* Quellen richtet man selten ein — eingeklappt, bis sie gebraucht werden. */}
        <details className="sources-section" open={sources.length === 0}>
          <summary>
            Quellen<span className="muted"> {sources.length}</span>
          </summary>
          <SourceList
            sources={sources}
            selectedId={selection.kind === "source" ? selection.id : null}
            busy={fetchingAll}
            onSelect={selectSource}
            onFetchAll={() => void fetchAll()}
            onCreate={() => void openSourceDialog(null)}
          />
        </details>
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
            onEdit={() => void openSourceDialog(selectedSource.source)}
            onOpenLink={openLink}
          />
        )}

        {draft && (
          <TemplateEditor
            slug={draft.slug}
            body={draft.body}
            preview={diagram}
            dirty={draft.body + draft.slug !== draft.saved}
            isNew={draft.id === null}
            dark={dark}
            sources={sources.map((s) => ({
              name: s.source.name,
              roles: s.source.mappings.map((m) => m.role),
            }))}
            help={help}
            fieldMessage={fieldMessage}
            onSlugChange={(slug) => setDraft({ ...draft, slug })}
            onBodyChange={(body) => setDraft({ ...draft, body })}
            onSave={() => void saveTemplate()}
            {...(draft.id
              ? { onDelete: () => setConfirmDelete({ kind: "template", name: draft.slug }) }
              : {})}
            onClose={() => {
              setDraft(null);
              setDiagram(null);
            }}
          />
        )}

        {selectedTemplate && (
          <section className="diagram-page">
            <header className="detail-head">
              <h1>{selectedTemplate.title}</h1>
              <div className="head-tools">
                {drawing && <span className="muted">Zeichne …</span>}
                <SourceRefresh
                  names={selectedTemplate.sources}
                  fetched={fetchedAt}
                  busy={fetchingAll}
                  onFetch={() => void fetchForDiagram(idsOf(selectedTemplate.sources))}
                />
                <button
                  type="button"
                  className="ghost"
                  onClick={() => {
                    setFieldError(null);
                    setSaveViewOpen(true);
                  }}
                >
                  Ansicht speichern
                </button>
                <button
                  type="button"
                  className="ghost"
                  onClick={() => void editTemplate(selectedTemplate.id)}
                >
                  Bearbeiten
                </button>
              </div>
            </header>
            {diagram && (
              <DiagramView
                mermaid={diagram.mermaid}
                dark={dark}
                onExport={(svg) => void exportSvg(svg)}
              />
            )}
          </section>
        )}

        {selection.kind === "flow" && flow && (
          <section className="diagram-page">
            <header className="detail-head">
              <h1>Fluss: {sources.find((s) => s.source.id === selection.id)?.source.name}</h1>
              <div className="head-tools">
                <SourceRefresh
                  names={namesOf(selection.id)}
                  fetched={fetchedAt}
                  busy={fetchingAll}
                  onFetch={() => void fetchForDiagram([selection.id])}
                />
                <label className="inline-field">
                  Zweite Zeile
                  <select
                    value={subtitle ?? ""}
                    onChange={(e) => {
                      const role = e.target.value || null;
                      setSubtitle(role);
                      void drawFlow(selection.id, hidden, role);
                    }}
                  >
                    <option value="">keine</option>
                    {flow.subtitle_roles.map((role) => (
                      <option key={role} value={role}>
                        {role}
                      </option>
                    ))}
                  </select>
                </label>
                <button
                  type="button"
                  className="ghost"
                  onClick={() => {
                    setFieldError(null);
                    setSaveViewOpen(true);
                  }}
                >
                  Ansicht speichern
                </button>
              </div>
            </header>
            <FlowView graph={flow} onExport={(svg) => void exportSvg(svg)} />
          </section>
        )}

        {selection.kind === "metro" && metro && (
          <section className="diagram-page">
            <header className="detail-head">
              <h1>Metro: {sources.find((s) => s.source.id === selection.id)?.source.name}</h1>
              <div className="head-tools">
                <SourceRefresh
                  names={namesOf(selection.id)}
                  fetched={fetchedAt}
                  busy={fetchingAll}
                  onFetch={() => void fetchForDiagram([selection.id])}
                />
                <button
                  type="button"
                  className="ghost"
                  onClick={() => {
                    setFieldError(null);
                    setSaveViewOpen(true);
                  }}
                >
                  Ansicht speichern
                </button>
              </div>
            </header>
            {metro.undated.length > 0 && (
              <p className="muted diagram-note">
                Ohne Datum, deshalb nicht auf der Karte: {metro.undated.join(", ")}
              </p>
            )}
            <MetroView map={metro} onExport={(svg) => void exportSvg(svg)} />
          </section>
        )}

        {selection.kind === "none" &&
          !draft &&
          (ready ? (
            <EmptyState />
          ) : (
            <GettingStarted
              hasToken={!!token?.origin}
              hasSource={sources.length > 0}
              hasFetch={sources.some((s) => s.fetch !== null)}
              hasTemplate={templates.length > 0}
              onOpenSettings={() => {
                setFieldError(null);
                setSettingsOpen(true);
              }}
              onCreateSource={() => void openSourceDialog(null)}
              onFetchAll={() => void fetchAll()}
              onCreateTemplate={() => void editTemplate(null)}
            />
          ))}
      </main>

      {selection.kind !== "none" && selection.kind !== "source" && nodes.length > 0 && (
        <FilterPanel
          nodes={nodes}
          hidden={hidden}
          fetched={fetchedAt}
          onToggle={toggleNode}
          onOnlyRelated={onlyRelated}
          onSetSource={setSourceVisible}
          onReset={() => changeHidden(new Set())}
        />
      )}

      {sourceDialog && (
        <SourceDialog
          source={sourceDialog.source}
          properties={properties}
          schema={schema}
          inspecting={inspecting}
          onInspect={(databaseId) => void inspectDatabase(databaseId)}
          fieldMessage={fieldMessage}
          onSave={(input) => void saveSource(input)}
          {...(sourceDialog.source
            ? {
                onDelete: () =>
                  setConfirmDelete({ kind: "source", name: sourceDialog.source?.name ?? "" }),
              }
            : {})}
          onClose={() => {
            setFieldError(null);
            setSourceDialog(null);
          }}
        />
      )}

      {saveViewOpen && selection.kind !== "none" && selection.kind !== "source" && (
        <SaveViewDialog
          suggestion={
            activeView
              ? (views.find((v) => v.id === activeView)?.name ?? "")
              : diagramTitle(selection)
          }
          hiddenCount={hidden.size}
          fieldMessage={fieldMessage}
          onSave={(name) => void saveView(name)}
          onClose={() => setSaveViewOpen(false)}
        />
      )}

      {confirmDelete && (
        <ConfirmDialog
          title={DELETE_TITLE[confirmDelete.kind]}
          message={deleteMessage(confirmDelete)}
          confirmLabel="Endgültig löschen"
          onConfirm={() => {
            if (confirmDelete.kind === "source") void deleteSource();
            else if (confirmDelete.kind === "template") void deleteTemplate();
            else void deleteView(confirmDelete.id);
          }}
          onCancel={() => setConfirmDelete(null)}
        />
      )}

      {settingsOpen && config && (
        <SettingsDialog
          config={config}
          info={info}
          token={token}
          fieldMessage={fieldMessage}
          onTokenSave={(next) => void saveToken(next)}
          onTokenClear={() => void clearToken()}
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
