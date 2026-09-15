//! Die Notion-API — nur lesend.
//!
//! ```text
//! Client ──Request──▶ Transport ──HTTPS──▶ api.notion.com
//!   │ Tempo (3/s), Wiederholen bei 429/5xx, Fehler deuten
//!   ▼
//! Database · DataSource (Schema) · Page
//! ```
//!
//! Der [`Transport`] ist ein Trait, damit die Tests mit festgehaltenen
//! Antworten aus `tests/fixtures/notion/` laufen — ohne Netz und ohne Token.
//! Die echte Fassung ist [`HttpTransport`].
//!
//! Die API-Version steht an genau einer Stelle: [`API_VERSION`]. Seit
//! `2025-09-03` hat eine Datenbank **Datenquellen**; Schema und Seiten kommen
//! von dort (`/data_sources/{id}`), nicht mehr von der Datenbank selbst.

mod client;
mod id;
mod model;
mod transport;

pub use client::{Client, MIN_INTERVAL};
pub use id::parse_id;
pub use model::{DataSourceRef, Database, Page, Property, property_kinds};
pub use transport::{HttpTransport, Method, Request, Response, Transport};

/// Die Fassung der Notion-API, gegen die diese Anwendung geschrieben ist.
pub const API_VERSION: &str = "2025-09-03";
