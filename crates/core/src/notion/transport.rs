//! Eine Anfrage hin, eine Antwort zurück — sonst nichts.
//!
//! Tempo, Wiederholungen und die Deutung von Fehlern stehen im
//! [`super::Client`]. Ein Transport sagt nur, was Notion geantwortet hat, oder
//! dass keine Antwort kam.

use std::net::SocketAddr;
use std::time::Duration;

use serde_json::Value;
use ureq::Agent;
use ureq::config::Config as UreqConfig;
use ureq::http::Uri;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
use ureq::unversioned::resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::{DefaultConnector, NextTimeout};

use super::API_VERSION;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
}

/// `path` beginnt hinter `/v1`, etwa `/databases/…`.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub body: Option<Value>,
}

impl Request {
    pub fn get(path: impl Into<String>) -> Self {
        Self {
            method: Method::Get,
            path: path.into(),
            body: None,
        }
    }

    pub fn post(path: impl Into<String>, body: Value) -> Self {
        Self {
            method: Method::Post,
            path: path.into(),
            body: Some(body),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub status: u16,
    /// Aus dem Kopf `Retry-After`, falls vorhanden.
    pub retry_after: Option<Duration>,
    /// Der Rumpf als JSON. Kein JSON (etwa eine HTML-Seite eines Proxys) wird
    /// zu einem JSON-Text, damit die Meldung nicht verloren geht.
    pub body: Value,
}

pub trait Transport: Send + Sync {
    /// `Err` nur, wenn **keine** Antwort kam. Ein 4xx oder 5xx ist eine
    /// Antwort.
    fn send(&self, request: &Request) -> Result<Response>;
}

/// Verbindungsaufbau. Danach probiert ureq die nächste Adresse.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Obergrenze für eine ganze Anfrage einschließlich Antwort.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Die echte Verbindung zu `api.notion.com`.
pub struct HttpTransport {
    agent: Agent,
    token: String,
    base: String,
}

impl HttpTransport {
    pub fn new(token: &str) -> Self {
        Self::with_base(token, "https://api.notion.com/v1")
    }

    /// Für einen Testserver oder einen Proxy.
    pub fn with_base(token: &str, base: &str) -> Self {
        // Einmal je Prozess. Ein zweiter Aufruf meldet nur, dass schon einer
        // gesetzt ist — das ist kein Fehler.
        let _ = rustls::crypto::ring::default_provider().install_default();

        let config = Agent::config_builder()
            .tls_config(
                TlsConfig::builder()
                    .provider(TlsProvider::Rustls)
                    // Zertifikate aus dem System, nicht aus einer eingebauten
                    // Liste — sonst scheitert jeder Firmen-Proxy mit eigener
                    // Zertifizierungsstelle.
                    .root_certs(RootCerts::PlatformVerifier)
                    .build(),
            )
            // Ein 404 ist eine Antwort, kein Transportfehler. Der Client deutet sie.
            .http_status_as_error(false)
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_global(Some(REQUEST_TIMEOUT))
            .user_agent(format!("vizu-notion/{}", env!("CARGO_PKG_VERSION")))
            .build();

        Self {
            agent: Agent::with_parts(config, DefaultConnector::new(), Ipv4First::default()),
            token: token.to_string(),
            base: base.trim_end_matches('/').to_string(),
        }
    }
}

// Von Hand, damit der Token nie in einem Protokoll landet.
impl std::fmt::Debug for HttpTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpTransport")
            .field("base", &self.base)
            .field("token", &"…")
            .finish()
    }
}

impl Transport for HttpTransport {
    fn send(&self, request: &Request) -> Result<Response> {
        let url = format!("{}{}", self.base, request.path);
        let auth = format!("Bearer {}", self.token);
        tracing::debug!(method = ?request.method, path = %request.path, "Notion-Anfrage");

        let result = match request.method {
            Method::Get => self
                .agent
                .get(&url)
                .header("Authorization", &auth)
                .header("Notion-Version", API_VERSION)
                .call(),
            Method::Post => self
                .agent
                .post(&url)
                .header("Authorization", &auth)
                .header("Notion-Version", API_VERSION)
                .send_json(request.body.clone().unwrap_or(Value::Null)),
        };
        let response = result.map_err(|e| Error::Unreachable(e.to_string()))?;

        let status = response.status().as_u16();
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<u64>().ok())
            .map(Duration::from_secs);
        let text = response
            .into_body()
            .read_to_string()
            .map_err(|e| Error::Unreachable(format!("Antwort nicht lesbar: {e}")))?;
        let body = serde_json::from_str(&text).unwrap_or(Value::String(text));

        Ok(Response {
            status,
            retry_after,
            body,
        })
    }
}

/// Probiert IPv4-Adressen vor IPv6.
///
/// ureq versucht die Adressen nacheinander. Kommt IPv6 im lokalen Netz nicht
/// durch, hängt der erste Versuch bis zum Verbindungs-Timeout — gemessen in
/// Spike S3: zehn Sekunden vor jedem ersten Abruf. `api.notion.com` ist über
/// IPv4 immer erreichbar; ein reines IPv6-Netz bekommt seine Adressen trotzdem,
/// nur als zweite Wahl.
#[derive(Debug, Default)]
struct Ipv4First(DefaultResolver);

impl Resolver for Ipv4First {
    fn resolve(
        &self,
        uri: &Uri,
        config: &UreqConfig,
        timeout: NextTimeout,
    ) -> std::result::Result<ResolvedSocketAddrs, ureq::Error> {
        let mut addrs = self.0.resolve(uri, config, timeout)?;
        // Stabil sortiert: innerhalb einer Familie bleibt die Reihenfolge des
        // Namensdienstes erhalten.
        addrs.sort_by_key(SocketAddr::is_ipv6);
        Ok(addrs)
    }
}
