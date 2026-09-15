//! Hilfen für die Tests, die mit Notion sprechen — ohne Netz.
//!
//! [`FixtureNotion`] beantwortet Anfragen mit den festgehaltenen Antworten aus
//! `tests/fixtures/notion/2025-09-03/` (siehe README dort). Einzelne Pfade
//! lassen sich mit [`FixtureNotion::route`] überschreiben, etwa um ein 429 oder
//! eine gekürzte Relation nachzustellen.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use vizu_notion_core::Result;
use vizu_notion_core::notion::{Client, Method, Request, Response, Transport};

pub fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/notion")
}

pub fn read_json(relative: &str) -> Value {
    let path = fixture_dir().join(relative);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap()
}

/// Die Ersatz-IDs der drei Testdatenbanken.
pub struct Ids {
    ids: Value,
}

impl Ids {
    pub fn load() -> Self {
        Self {
            ids: read_json("ids.json"),
        }
    }

    /// `ziele`, `projekte` oder `aufgaben`.
    pub fn database(&self, label: &str) -> String {
        self.ids["databases"][label].as_str().unwrap().to_string()
    }

    pub fn data_source(&self, label: &str) -> String {
        self.ids["data_sources"][label]
            .as_str()
            .unwrap()
            .to_string()
    }
}

type Handler = Box<dyn Fn(&Request) -> Option<Response> + Send + Sync>;

pub struct FixtureNotion {
    ids: Ids,
    routes: Vec<Handler>,
    log: Arc<Mutex<Vec<Request>>>,
}

impl FixtureNotion {
    pub fn new() -> Self {
        Self {
            ids: Ids::load(),
            routes: Vec::new(),
            log: Arc::default(),
        }
    }

    /// Eigene Antwort für Anfragen, auf die `handler` mit `Some` antwortet.
    /// Wird vor den Dateien befragt, spätere Routen vor früheren.
    pub fn route(
        mut self,
        handler: impl Fn(&Request) -> Option<Response> + Send + Sync + 'static,
    ) -> Self {
        self.routes.insert(0, Box::new(handler));
        self
    }

    /// Alle bisher gestellten Anfragen.
    pub fn log(&self) -> Arc<Mutex<Vec<Request>>> {
        self.log.clone()
    }

    fn respond_from_files(&self, request: &Request) -> Response {
        for label in ["ziele", "projekte", "aufgaben"] {
            let db = self.ids.database(label);
            let ds = self.ids.data_source(label);
            if request.method == Method::Get && request.path == format!("/databases/{db}") {
                return ok(read_json(&format!("2025-09-03/{label}.database.json")));
            }
            if request.method == Method::Get && request.path == format!("/data_sources/{ds}") {
                return ok(read_json(&format!("2025-09-03/{label}.data_source.json")));
            }
            if request.method == Method::Post && request.path == format!("/data_sources/{ds}/query")
            {
                return ok(query_batch(label, request.body.as_ref()));
            }
        }
        not_found(&request.path)
    }
}

impl Transport for FixtureNotion {
    fn send(&self, request: &Request) -> Result<Response> {
        self.log.lock().unwrap().push(request.clone());
        for route in &self.routes {
            if let Some(response) = route(request) {
                return Ok(response);
            }
        }
        Ok(self.respond_from_files(request))
    }
}

/// Der Stapel, dessen Vorgänger `start_cursor` als `next_cursor` hatte.
fn query_batch(label: &str, body: Option<&Value>) -> Value {
    let cursor = body.and_then(|b| b["start_cursor"].as_str());
    let mut n = 1;
    let mut previous: Option<Value> = None;
    loop {
        let path = fixture_dir().join(format!("2025-09-03/{label}.query.{n}.json"));
        if !path.exists() {
            panic!("kein Stapel zum Cursor {cursor:?} in {label}");
        }
        let batch = read_json(&format!("2025-09-03/{label}.query.{n}.json"));
        let matches = match (cursor, &previous) {
            (None, None) => true,
            (Some(c), Some(p)) => p["next_cursor"] == json!(c),
            _ => false,
        };
        if matches {
            return batch;
        }
        previous = Some(batch);
        n += 1;
    }
}

pub fn ok(body: Value) -> Response {
    Response {
        status: 200,
        retry_after: None,
        body,
    }
}

pub fn status(status: u16, code: &str, message: &str) -> Response {
    Response {
        status,
        retry_after: None,
        body: json!({ "object": "error", "status": status, "code": code, "message": message }),
    }
}

pub fn not_found(path: &str) -> Response {
    status(
        404,
        "object_not_found",
        &format!(
            "Could not find object at {path}. Make sure the relevant pages and databases are shared with your integration."
        ),
    )
}

/// Ein Client ohne echte Pausen. Die gewünschten Pausen landen in der Liste.
pub fn client<T: Transport>(transport: T) -> (Client<T>, Arc<Mutex<Vec<Duration>>>) {
    let sleeps: Arc<Mutex<Vec<Duration>>> = Arc::default();
    let record = sleeps.clone();
    let client = Client::with_pacing(
        transport,
        Duration::ZERO,
        Box::new(move |d| record.lock().unwrap().push(d)),
    );
    (client, sleeps)
}

/// Antworten der Reihe nach, danach immer die letzte.
pub struct Scripted {
    responses: Mutex<Vec<Response>>,
    pub calls: Arc<Mutex<u32>>,
}

impl Scripted {
    pub fn new(responses: Vec<Response>) -> Self {
        Self {
            responses: Mutex::new(responses),
            calls: Arc::default(),
        }
    }
}

impl Transport for Scripted {
    fn send(&self, _request: &Request) -> Result<Response> {
        *self.calls.lock().unwrap() += 1;
        let mut responses = self.responses.lock().unwrap();
        if responses.len() > 1 {
            Ok(responses.remove(0))
        } else {
            Ok(responses[0].clone())
        }
    }
}

/// Zählt, wie oft welcher Pfad angefragt wurde.
pub fn count_paths(log: &Mutex<Vec<Request>>) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for r in log.lock().unwrap().iter() {
        let path = r.path.split('?').next().unwrap().to_string();
        *out.entry(path).or_default() += 1;
    }
    out
}
