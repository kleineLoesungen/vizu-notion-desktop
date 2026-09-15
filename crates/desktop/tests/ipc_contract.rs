//! Die IPC-Grenze zwischen Webview und Rust.
//!
//! Drei Sorten Prüfung:
//!
//! 1. **Echte Aufrufe** über `tauri::test::MockRuntime` — ohne Fenster, aber
//!    mit derselben Deserialisierung der Argumente wie im Webview. Ein
//!    Argument, das in Rust `note_id` heißt und im JSON `id`, fällt hier auf
//!    und nicht erst beim Klicken.
//! 2. **Die Befehlsliste** in `lib.rs` gegen die in `ui/src/api.ts`. Ein
//!    Befehl, der nur auf einer Seite steht, ist entweder toter Code oder ein
//!    Aufruf, der zur Laufzeit mit „command not found" scheitert.
//! 3. **Die Sicherheitsrichtlinie** in `tauri.conf.json` — so eng wie möglich,
//!    so weit wie Mermaid es braucht.
//!
//! Die fachlichen Regeln stehen in den Tests von `vizu-notion-core`. Hier geht es
//! nur ums Durchreichen.

use serde_json::{Value, json};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{INVOKE_KEY, MockRuntime, mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::{WebviewWindow, WebviewWindowBuilder};
use vizu_notion_core::secret::{MemoryStore, SecretStore};
use vizu_notion_core::source::{ColumnMapping, SourceInput};
use vizu_notion_core::{App, Paths, source};
use vizu_notion_desktop::{AppState, configure};

struct Ctx {
    _dir: tempfile::TempDir,
    /// Derselbe Speicher, den die Anwendung benutzt — nie der Schlüsselbund
    /// des Rechners, auf dem die Tests laufen.
    secrets: std::sync::Arc<MemoryStore>,
    // Die App muss leben, solange das Fenster benutzt wird.
    _app: tauri::App<MockRuntime>,
    window: WebviewWindow<MockRuntime>,
}

impl Ctx {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let secrets = std::sync::Arc::new(MemoryStore::default());
        let core = App::open_with(Paths::under(dir.path()), Box::new(secrets.clone())).unwrap();
        Self {
            secrets,
            ..Self::with_state(dir, AppState::new(core))
        }
    }

    /// Eine Quelle, wie sie sonst die Kommandozeile anlegt.
    fn source(&self, name: &str) -> uuid::Uuid {
        let app = App::open_with(
            Paths::under(self.dir_path()),
            Box::new(MemoryStore::default()),
        )
        .unwrap();
        source::create(
            app.conn(),
            SourceInput::new(
                name,
                "396f66270f5d8034b55cebc685aa5e50",
                vec![ColumnMapping::new("title", "Name")],
            ),
        )
        .unwrap()
        .id
    }

    fn set_token(&self, token: &str) {
        self.secrets.save(token).unwrap();
    }

    fn dir_path(&self) -> std::path::PathBuf {
        self._dir.path().to_path_buf()
    }

    fn with_state(dir: tempfile::TempDir, state: AppState) -> Self {
        let app = configure(mock_builder(), state)
            .build(mock_context(noop_assets()))
            .expect("Tauri-Attrappe baut");
        let window = WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("Fenster der Attrappe");
        Self {
            _dir: dir,
            secrets: std::sync::Arc::new(MemoryStore::default()),
            _app: app,
            window,
        }
    }

    /// Ruft einen Befehl so auf, wie `invoke(cmd, args)` im Webview es tut.
    fn invoke(&self, cmd: &str, args: Value) -> Result<Value, Value> {
        tauri::test::get_ipc_response(
            &self.window,
            InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                // So heißt die eigene Oberfläche auf macOS und Linux. Eine
                // andere Adresse gilt als fremde Seite, und Tauri verweigert
                // ihr jeden Befehl.
                url: "tauri://localhost".parse().unwrap(),
                body: InvokeBody::Json(args),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )
        .map(|body| body.deserialize::<Value>().expect("Antwort ist JSON"))
    }
}

#[test]
fn listet_quellen_mit_ihrem_abrufstand() {
    let ctx = Ctx::new();
    let source = ctx.source("Projekte");

    let list = ctx.invoke("source_list", json!({})).unwrap();

    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["source"]["name"], "Projekte");
    assert_eq!(list[0]["source"]["mappings"][0]["role"], "title");
    assert!(list[0]["fetch"].is_null(), "noch nie abgerufen");

    let single = ctx
        .invoke("source_get", json!({ "id": source.to_string() }))
        .unwrap();
    assert_eq!(single["id"], source.to_string());
}

#[test]
fn ein_abruf_ohne_token_nennt_den_grund_statt_es_zu_versuchen() {
    // Ohne Token darf gar keine Anfrage an Notion gehen — der Fehler kommt
    // sofort und trägt einen eigenen Code, damit die Oberfläche den Hinweis
    // auf `token set` zeigen kann.
    let ctx = Ctx::new();
    let source = ctx.source("Projekte");

    let err = ctx
        .invoke("source_fetch", json!({ "id": source.to_string() }))
        .unwrap_err();

    assert_eq!(err["code"], "token_missing");
}

#[test]
fn der_token_selbst_kommt_nie_ins_webview() {
    let ctx = Ctx::new();
    ctx.set_token("ntn_1234567890abcdefghijklmnopqrstuv");

    let status = ctx.invoke("token_status", json!({})).unwrap();

    assert_eq!(status["origin"], "store");
    assert_eq!(status["hint"], "ntn_…stuv");
    assert!(
        !status.to_string().contains("1234567890"),
        "Token sichtbar: {status}"
    );
}

#[test]
fn unbekannte_kennung_ist_not_found() {
    let ctx = Ctx::new();
    let err = ctx
        .invoke("source_get", json!({ "id": uuid::Uuid::now_v7() }))
        .unwrap_err();
    assert_eq!(err["code"], "not_found");
    assert!(err.get("fields").is_none(), "kein Eingabefehler");
}

#[test]
fn ungueltige_einstellungen_bleiben_ungespeichert() {
    let ctx = Ctx::new();
    let before = ctx.invoke("config_get", json!({})).unwrap();

    let mut broken = before.clone();
    broken["accent"] = json!("blau");
    let err = ctx
        .invoke("config_set", json!({ "config": broken }))
        .unwrap_err();
    assert_eq!(err["fields"][0]["field"], "accent");

    assert_eq!(ctx.invoke("config_get", json!({})).unwrap(), before);
}

#[test]
fn ein_fehler_beim_start_erreicht_jeden_befehl() {
    // Die Anwendung ließ sich nicht öffnen — das Fenster soll trotzdem eine
    // Meldung zeigen können, statt kommentarlos zu verschwinden.
    let dir = tempfile::tempdir().unwrap();
    let state = AppState::failed(vizu_notion_desktop::ApiError::internal("kaputt"));
    let ctx = Ctx::with_state(dir, state);

    let err = ctx.invoke("source_list", json!({})).unwrap_err();
    assert_eq!(err["message"], "kaputt");
}

#[test]
fn falscher_argumentname_wird_abgelehnt() {
    // Der häufigste Tauri-Fehler: Rust erwartet `id`, das Webview schickt
    // etwas anderes. Der Aufruf muss scheitern, nicht still nichts tun.
    let ctx = Ctx::new();
    let err = ctx
        .invoke("source_get", json!({ "sourceId": uuid::Uuid::now_v7() }))
        .unwrap_err();
    assert!(
        err.to_string().contains("id"),
        "Meldung nennt das fehlende Argument: {err}"
    );
}

// --- Sicherheitsrichtlinie -------------------------------------------------

#[test]
fn csp_laesst_inline_stile_von_diagrammen_zu() {
    // Tauri hängt beim Bündeln eine Nonce an style-src. Dann ignoriert der
    // Browser 'unsafe-inline', und Mermaid zeichnet schwarze Flächen — nur im
    // fertigen Bündel, nie in `just dev`. Siehe CLAUDE.md, Punkt 7.
    let conf: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let security = &conf["app"]["security"];

    let disabled = security["dangerousDisableAssetCspModification"]
        .as_array()
        .expect("dangerousDisableAssetCspModification als Liste");
    assert!(disabled.iter().any(|d| d == "style-src"));
    assert!(
        !disabled.iter().any(|d| d == "script-src"),
        "die Nonce für Skripte bleibt"
    );

    let csp = security["csp"].as_object().expect("csp als Objekt");
    assert!(
        csp["style-src"]
            .as_str()
            .unwrap()
            .contains("'unsafe-inline'")
    );
    for (directive, value) in csp {
        let value = value.as_str().unwrap();
        assert!(
            !value.contains("'unsafe-eval'") && !value.split(' ').any(|v| v == "*"),
            "{directive} öffnet die CSP zu weit: {value}"
        );
    }
}

// --- Befehlsliste Rust ↔ TypeScript ---------------------------------------

#[test]
fn jeder_befehl_steht_auf_beiden_seiten() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();

    let rust =
        rust_commands(&std::fs::read_to_string(root.join("crates/desktop/src/lib.rs")).unwrap());
    let ts = ts_commands(&std::fs::read_to_string(root.join("ui/src/api.ts")).unwrap());

    let only_rust: Vec<_> = rust.iter().filter(|c| !ts.contains(c)).collect();
    let only_ts: Vec<_> = ts.iter().filter(|c| !rust.contains(c)).collect();

    assert!(
        only_rust.is_empty() && only_ts.is_empty(),
        "Befehlslisten passen nicht zusammen.\n\
         Nur in lib.rs (generate_handler!): {only_rust:?}\n\
         Nur in ui/src/api.ts:              {only_ts:?}"
    );
    assert!(
        !rust.is_empty(),
        "keine Befehle in generate_handler! gefunden"
    );
}

/// `commands::note_list,` innerhalb von `generate_handler![ … ]`.
fn rust_commands(lib_rs: &str) -> Vec<String> {
    let start = lib_rs
        .find("generate_handler![")
        .expect("generate_handler! in lib.rs");
    let end = start + lib_rs[start..].find(']').expect("schließende Klammer");
    let mut out: Vec<String> = lib_rs[start + "generate_handler![".len()..end]
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with("//"))
        .map(|s| s.rsplit("::").next().unwrap().to_string())
        .collect();
    out.sort();
    out
}

/// `call<Note[]>("note_list", …)` in api.ts — ein Aufruf je Zeile.
fn ts_commands(api_ts: &str) -> Vec<String> {
    let mut out: Vec<String> = api_ts
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .filter_map(|line| {
            let rest = &line[line.find("call<")?..];
            let open = rest.find(">(\"")? + 3;
            let close = open + rest[open..].find('"')?;
            Some(rest[open..close].to_string())
        })
        .collect();
    out.sort();
    out.dedup();
    out
}
