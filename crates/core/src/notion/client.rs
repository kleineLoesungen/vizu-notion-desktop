//! Tempo, Wiederholungen, Blättern und die Deutung von Fehlern.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::model::{Database, Page};
use super::transport::{Request, Response, Transport};
use crate::error::{Error, NotionErrorKind, Result};

/// Mindestabstand zwischen zwei Anfragen. Notion erlaubt im Mittel drei je
/// Sekunde; etwas Luft schützt vor 429 bei ungleichmäßiger Uhr.
pub const MIN_INTERVAL: Duration = Duration::from_millis(340);

/// So oft wird ein 429 abgewartet, bevor der Abruf aufgibt.
const RATE_LIMIT_RETRIES: u32 = 5;
/// So oft wird ein 5xx wiederholt — mit 1 s und 2 s Pause.
const SERVER_ERROR_RETRIES: u32 = 2;

/// Notion liefert höchstens so viele Einträge je Stapel.
const PAGE_SIZE: u32 = 100;

type Sleep = Box<dyn Fn(Duration) + Send + Sync>;

pub struct Client<T: Transport> {
    transport: T,
    min_interval: Duration,
    sleep: Sleep,
    last: Mutex<Option<Instant>>,
    requests: AtomicU32,
}

impl<T: Transport> Client<T> {
    pub fn new(transport: T) -> Self {
        Self::with_pacing(transport, MIN_INTERVAL, Box::new(std::thread::sleep))
    }

    /// Mit eigenem Takt und eigener Pause. Tests übergeben eine Pause, die
    /// nur mitschreibt, statt zu schlafen.
    pub fn with_pacing(transport: T, min_interval: Duration, sleep: Sleep) -> Self {
        Self {
            transport,
            min_interval,
            sleep,
            last: Mutex::new(None),
            requests: AtomicU32::new(0),
        }
    }

    /// Wie viele Anfragen dieser Client bisher gestellt hat, Wiederholungen
    /// eingeschlossen.
    pub fn requests(&self) -> u32 {
        self.requests.load(Ordering::Relaxed)
    }

    /// `GET /databases/{id}` — Titel, Adresse und die Datenquellen.
    pub fn database(&self, id: &str) -> Result<Database> {
        let body = self.call(&Request::get(format!("/databases/{id}")))?;
        serde_json::from_value(body)
            .map_err(|e| unexpected(format!("Antwort auf /databases unlesbar: {e}")))
    }

    /// `GET /data_sources/{id}` — das Schema, unverändert als JSON.
    pub fn data_source(&self, id: &str) -> Result<Value> {
        self.call(&Request::get(format!("/data_sources/{id}")))
    }

    /// `GET /pages/{id}` — eine einzelne Seite, etwa das Ziel einer Relation
    /// in einer Datenbank, die selbst keine Quelle ist.
    pub fn page(&self, id: &str) -> Result<Page> {
        let body = self.call(&Request::get(format!("/pages/{id}")))?;
        serde_json::from_value(body).map_err(|e| unexpected(format!("Seite unlesbar: {e}")))
    }

    /// Alle Seiten einer Datenquelle, Stapel für Stapel.
    pub fn query_data_source(&self, id: &str) -> Result<Vec<Page>> {
        let path = format!("/data_sources/{id}/query");
        let mut pages = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let mut body = json!({ "page_size": PAGE_SIZE });
            if let Some(c) = &cursor {
                body["start_cursor"] = json!(c);
            }
            let batch = self.call(&Request::post(&path, body))?;
            for item in batch["results"].as_array().into_iter().flatten() {
                // Eine Abfrage kann in Wikis auch andere Objekte liefern.
                if item["object"] != "page" {
                    continue;
                }
                let page: Page = serde_json::from_value(item.clone())
                    .map_err(|e| unexpected(format!("Seite unlesbar: {e}")))?;
                pages.push(page);
            }
            match next_cursor(&batch) {
                Some(next) => cursor = Some(next),
                None => return Ok(pages),
            }
        }
    }

    /// Alle Ziele einer Relation.
    ///
    /// Im Seitenobjekt stehen höchstens 25; darüber setzt Notion
    /// `has_more: true`, und der Rest kommt nur über diesen Endpunkt.
    pub fn relation_ids(&self, page_id: &str, property_id: &str) -> Result<Vec<String>> {
        let mut ids = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let mut path =
                format!("/pages/{page_id}/properties/{property_id}?page_size={PAGE_SIZE}");
            if let Some(c) = &cursor {
                path.push_str(&format!("&start_cursor={c}"));
            }
            let batch = self.call(&Request::get(path))?;
            for item in batch["results"].as_array().into_iter().flatten() {
                if let Some(id) = item["relation"]["id"].as_str() {
                    ids.push(id.to_string());
                }
            }
            match next_cursor(&batch) {
                Some(next) => cursor = Some(next),
                None => return Ok(ids),
            }
        }
    }

    fn call(&self, request: &Request) -> Result<Value> {
        let mut rate_limited = 0;
        let mut server_errors = 0;
        loop {
            self.pace();
            self.requests.fetch_add(1, Ordering::Relaxed);
            let response = self.transport.send(request)?;
            match response.status {
                200..=299 => return Ok(response.body),
                429 if rate_limited < RATE_LIMIT_RETRIES => {
                    rate_limited += 1;
                    let wait = response.retry_after.unwrap_or(Duration::from_secs(1));
                    tracing::info!(?wait, path = %request.path, "Notion bremst (429)");
                    (self.sleep)(wait);
                }
                500 | 502 | 503 | 504 if server_errors < SERVER_ERROR_RETRIES => {
                    server_errors += 1;
                    (self.sleep)(Duration::from_secs(1 << (server_errors - 1)));
                }
                _ => return Err(notion_error(request, &response)),
            }
        }
    }

    /// Wartet, bis seit der letzten Anfrage [`MIN_INTERVAL`] vergangen ist.
    fn pace(&self) {
        let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(previous) = *last {
            let wait = self.min_interval.saturating_sub(previous.elapsed());
            if !wait.is_zero() {
                (self.sleep)(wait);
            }
        }
        *last = Some(Instant::now());
    }
}

fn next_cursor(batch: &Value) -> Option<String> {
    if batch["has_more"] == json!(true) {
        batch["next_cursor"].as_str().map(String::from)
    } else {
        None
    }
}

fn unexpected(message: String) -> Error {
    Error::Notion {
        kind: NotionErrorKind::Other,
        status: 200,
        message,
    }
}

/// Aus einer Fehlerantwort eine Meldung, mit der ein Mensch etwas anfangen kann.
fn notion_error(request: &Request, response: &Response) -> Error {
    let from_notion = response.body["message"]
        .as_str()
        .map(String::from)
        .unwrap_or_else(|| format!("HTTP {}", response.status));
    let (kind, message) = match response.status {
        401 => (
            NotionErrorKind::Unauthorized,
            format!(
                "Token abgelehnt — falsch, widerrufen oder von einer anderen Integration ({from_notion})"
            ),
        ),
        403 | 404 => (
            NotionErrorKind::NotShared,
            format!(
                "nicht gefunden — ist sie mit der Integration geteilt? ({from_notion}; {})",
                request.path
            ),
        ),
        429 => (
            NotionErrorKind::RateLimited,
            format!("zu viele Anfragen, auch nach mehreren Pausen ({from_notion})"),
        ),
        _ => (NotionErrorKind::Other, from_notion),
    };
    Error::Notion {
        kind,
        status: response.status,
        message,
    }
}
