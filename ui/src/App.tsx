// Die einzige Komponente, die api.ts benutzt.
//
// Hier liegt der Zustand der Oberfläche, und hier wird aus einem Wunsch einer
// Komponente ein Befehl an Rust:
//
//   Komponente  ──onFetch()──▶  App.tsx  ──api.sources.fetch()──▶  Rust
//
// Fachliche Regeln stehen hier nicht. Ob eine Zuordnung gültig ist, entscheidet
// crates/core — hier wird nur angezeigt, was von dort zurückkommt.

import { useCallback, useEffect, useState } from "react";
import { ApiError, api } from "./api";
import type { AppInfo, Config, SourceOverview, TokenStatus } from "./bindings";
import { EmptyState } from "./components/EmptyState";
import { SettingsDialog } from "./components/SettingsDialog";
import { SourceDetail } from "./components/SourceDetail";
import { SourceList } from "./components/SourceList";
import { applyTheme } from "./lib/theme";

export function App() {
  const [sources, setSources] = useState<SourceOverview[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
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
      setSources(await api.sources.list());
    } catch (raw) {
      fail(raw);
    }
  }, [fail]);

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

  // --- Quellen -------------------------------------------------------------

  async function fetchOne(id: string) {
    setBusyId(id);
    setBanner(null);
    try {
      await api.sources.fetch(id);
      await reload();
      setToken(await api.tokenStatus());
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

  const selected = sources.find((s) => s.source.id === selectedId) ?? null;
  const fieldMessage = (field: string) => fieldError?.fieldMessage(field);

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">{config?.app_name ?? " "}</div>
        <SourceList
          sources={sources}
          selectedId={selectedId}
          busy={fetchingAll}
          onSelect={setSelectedId}
          onFetchAll={() => void fetchAll()}
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

        {selected ? (
          <SourceDetail
            entry={selected}
            busy={busyId === selected.source.id}
            onFetch={() => void fetchOne(selected.source.id)}
            onOpenLink={openLink}
          />
        ) : (
          <EmptyState hasSources={sources.length > 0} />
        )}
      </main>

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
