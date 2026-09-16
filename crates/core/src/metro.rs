//! Die Metro-Karte: Zeit auf der x-Achse, Linien untereinander.
//!
//! ```text
//! Seiten mit `date` und `next`
//!   │  Ketten entlang der Nachfolger      →  Linien
//!   │  Datum auf einer Zeitachse          →  x
//!   │  Linie                              →  y (eine Spur je Linie)
//!   │  `tag` oder `parent` der Linie      →  Band im Hintergrund
//!   ▼
//! MetroMap { lines, zones, ticks, width, height }
//! ```
//!
//! Wie beim Flussdiagramm steht **die Anordnung hier**, nicht in der
//! Oberfläche: `vizu-notion metro --json` gibt dieselben Zahlen aus, mit denen
//! das Fenster zeichnet.
//!
//! Das ist bewusst **nicht** Metroviz aus der Webapp nachgebaut. Deren
//! Darstellung hängt an d3, an eigenen Stilen und an 145 Farbangaben in einer
//! CSS-Datei — Dinge, die hier weder gebraucht noch erlaubt sind. Gemeinsam
//! ist die Idee: eine Zeitachse, Linien mit Stationen, Bänder dahinter.

use std::collections::{BTreeMap, HashSet};

use rusqlite::Connection;
use serde::Serialize;
use time::{Date, Month};
use ts_rs::TS;
use uuid::Uuid;

use crate::error::Result;
use crate::notion::Page;
use crate::rows::{self, NodeInfo};
use crate::source::Source;
use crate::{fetch, palette, source};

/// Höhe einer Spur in Punkten.
pub const LANE_HEIGHT: f32 = 96.0;
/// Rand links und rechts, damit die äußersten Beschriftungen Platz haben.
const MARGIN_X: f32 = 120.0;
/// Höhe der Zeitachse über der ersten Spur.
const AXIS_HEIGHT: f32 = 48.0;
/// Breite der Zeichnung ohne Ränder.
const TIMELINE_WIDTH: f32 = 1200.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum StationKind {
    /// Anfang einer Linie.
    Start,
    /// Station dazwischen.
    Stop,
    /// Ende einer Linie.
    Terminus,
    /// Eine Seite, auf die nichts zeigt und die auf nichts zeigt.
    Single,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct MetroStation {
    /// Die Seiten-ID — dieselbe wie im Filterfeld.
    pub id: String,
    pub title: String,
    /// Wie in Notion, als Text: `2026-04-01`.
    pub date: String,
    pub x: f32,
    pub y: f32,
    pub kind: StationKind,
    /// Beschriftung über oder unter der Station — sonst überlagern sie sich.
    pub label_above: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct MetroLine {
    pub label: String,
    pub color: String,
    pub stations: Vec<MetroStation>,
}

/// Ein Band hinter mehreren Spuren: alle Linien mit demselben `tag`/`parent`.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct MetroZone {
    pub label: String,
    pub y: f32,
    pub height: f32,
}

/// Eine Marke auf der Zeitachse.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct MetroTick {
    pub label: String,
    pub x: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct MetroMap {
    pub lines: Vec<MetroLine>,
    pub zones: Vec<MetroZone>,
    pub ticks: Vec<MetroTick>,
    pub width: f32,
    pub height: f32,
    /// Jede Seite der Quelle, auch die ausgeblendeten: das Filterfeld.
    pub all_nodes: Vec<NodeInfo>,
    /// Seiten ohne lesbares Datum. Sie stehen in keiner Linie — die Oberfläche
    /// sagt es, statt sie stillschweigend zu verschlucken.
    pub undated: Vec<String>,
}

/// Eine Quelle taugt als Metro-Karte, sobald sie `date` **und** `next` hat.
pub fn eligible(source: &Source) -> bool {
    source.property("date").is_some() && source.property("next").is_some()
}

/// Baut die Karte aus dem Zwischenspeicher — ohne Netz.
pub fn build(conn: &Connection, source_id: Uuid, hidden: &HashSet<String>) -> Result<MetroMap> {
    let source = source::get(conn, source_id)?;
    let pages = fetch::pages(conn, source_id)?;

    let mut all_nodes = Vec::new();
    for page in &pages {
        all_nodes.push(NodeInfo {
            id: page.id.clone(),
            title: rows::page_title(page, &source),
            source: source.name.clone(),
            relations: rows::relation_targets(page),
        });
    }

    let visible: Vec<&Page> = pages.iter().filter(|p| !hidden.contains(&p.id)).collect();
    let mut dated = BTreeMap::new();
    let mut undated = Vec::new();
    for page in &visible {
        match date_of(page, &source) {
            Some(date) => {
                dated.insert(page.id.as_str(), date);
            }
            None => undated.push(rows::page_title(page, &source)),
        }
    }

    let chains = chains(&visible, &source, &dated, hidden);
    let (first, last) = span(&dated);
    let scale = Scale::new(first, last);

    let mut lines = Vec::new();
    for (index, chain) in chains.iter().enumerate() {
        let y = AXIS_HEIGHT + index as f32 * LANE_HEIGHT + LANE_HEIGHT / 2.0;
        let stations: Vec<MetroStation> = chain
            .pages
            .iter()
            .enumerate()
            .map(|(i, page)| {
                let date = dated[page.id.as_str()];
                MetroStation {
                    id: page.id.clone(),
                    title: rows::page_title(page, &source),
                    date: date_text(date),
                    x: scale.x(date),
                    y,
                    kind: match i {
                        _ if chain.pages.len() == 1 => StationKind::Single,
                        0 => StationKind::Start,
                        i if i + 1 == chain.pages.len() => StationKind::Terminus,
                        _ => StationKind::Stop,
                    },
                    // Abwechselnd oben und unten: Zwei Stationen dicht
                    // beieinander hätten sonst überlappende Beschriftungen.
                    label_above: i % 2 == 0,
                }
            })
            .collect();
        lines.push(MetroLine {
            label: chain.label.clone(),
            color: palette::nth(index).to_string(),
            stations,
        });
    }

    let zones = zones(&chains);
    let height = AXIS_HEIGHT + chains.len().max(1) as f32 * LANE_HEIGHT;
    Ok(MetroMap {
        lines,
        zones,
        ticks: scale.ticks(),
        width: TIMELINE_WIDTH + 2.0 * MARGIN_X,
        height,
        all_nodes,
        undated,
    })
}

/// Eine Kette von Seiten entlang `next`, mit dem Band, in das sie gehört.
struct Chain<'a> {
    label: String,
    zone: String,
    pages: Vec<&'a Page>,
}

/// Zerlegt die Seiten in Ketten.
///
/// Anfang ist, worauf niemand zeigt. Danach immer dem ersten Nachfolger
/// nach — verzweigt eine Seite, beginnt für den zweiten Nachfolger eine eigene
/// Linie. Was in einem Kreis steckt und nie als Anfang auftaucht, bildet eine
/// eigene Linie ab der ersten noch unbesuchten Seite; sonst fehlte sie ganz.
fn chains<'a>(
    pages: &[&'a Page],
    source: &Source,
    dated: &BTreeMap<&str, Date>,
    hidden: &HashSet<String>,
) -> Vec<Chain<'a>> {
    let next_property = source.property("next");
    let successors = |page: &Page| -> Vec<String> {
        next_property
            .and_then(|property| page.properties.get(property))
            .map(|value| {
                value["relation"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|t| t["id"].as_str().map(String::from))
                    .filter(|id| !hidden.contains(id) && dated.contains_key(id.as_str()))
                    .collect()
            })
            .unwrap_or_default()
    };

    // Seiten ohne Datum kommen nicht auf die Karte; sie sollen aber auch keine
    // Kette zerreißen — deshalb fallen sie hier vorher heraus.
    let placed: Vec<&Page> = pages
        .iter()
        .copied()
        .filter(|p| dated.contains_key(p.id.as_str()))
        .collect();
    let by_id: BTreeMap<&str, &Page> = placed.iter().map(|p| (p.id.as_str(), *p)).collect();

    let mut has_incoming: HashSet<String> = HashSet::new();
    for page in &placed {
        for target in successors(page) {
            has_incoming.insert(target);
        }
    }

    let mut used: HashSet<String> = HashSet::new();
    let mut chains = Vec::new();
    // Erst die echten Anfänge, dann der Rest — das hält die Reihenfolge
    // nachvollziehbar und fängt Kreise ein.
    let starts = placed
        .iter()
        .filter(|p| !has_incoming.contains(&p.id))
        .chain(placed.iter());

    for start in starts {
        if used.contains(&start.id) {
            continue;
        }
        let mut chain = Vec::new();
        let mut current = *start;
        loop {
            if !used.insert(current.id.clone()) {
                break;
            }
            chain.push(current);
            let Some(next) = successors(current)
                .into_iter()
                .find(|id| !used.contains(id))
                .and_then(|id| by_id.get(id.as_str()).copied())
            else {
                break;
            };
            current = next;
        }
        if chain.is_empty() {
            continue;
        }
        // Die Linie heißt wie ihre erste Station, das Band wie deren `tag`
        // oder — wenn es keinen gibt — wie ihr `parent`.
        let label = rows::page_title(chain[0], source);
        let zone = ["tag", "parent"]
            .iter()
            .find_map(|role| {
                source
                    .property(role)
                    .and_then(|property| chain[0].properties.get(property))
                    .map(rows::text_of)
                    .filter(|text| !text.is_empty())
            })
            .unwrap_or_default();
        chains.push(Chain {
            label,
            zone,
            pages: chain,
        });
    }
    chains
}

/// Bänder hinter zusammenhängenden Spuren mit demselben Schlüssel.
fn zones(chains: &[Chain<'_>]) -> Vec<MetroZone> {
    let mut zones: Vec<MetroZone> = Vec::new();
    for (index, chain) in chains.iter().enumerate() {
        if chain.zone.is_empty() {
            continue;
        }
        let y = AXIS_HEIGHT + index as f32 * LANE_HEIGHT;
        match zones.last_mut() {
            Some(last) if last.label == chain.zone && last.y + last.height == y => {
                last.height += LANE_HEIGHT;
            }
            _ => zones.push(MetroZone {
                label: chain.zone.clone(),
                y,
                height: LANE_HEIGHT,
            }),
        }
    }
    zones
}

/// Das Datum einer Seite, aus der Rolle `date`.
fn date_of(page: &Page, source: &Source) -> Option<Date> {
    let text = source
        .property("date")
        .and_then(|property| page.properties.get(property))
        .map(rows::text_of)?;
    parse_date(&text)
}

/// `2026-04-01`, `2026-04` oder `2026` — mehr liefert Notion nicht.
pub fn parse_date(text: &str) -> Option<Date> {
    let text = text.split('T').next()?.trim();
    let mut parts = text.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month = parts
        .next()
        .map_or(Some(1), |m| m.parse::<u8>().ok())
        .filter(|m| (1..=12).contains(m))?;
    let day = parts
        .next()
        .map_or(Some(1), |d| d.parse::<u8>().ok())
        .filter(|d| (1..=31).contains(d))?;
    Date::from_calendar_date(year, Month::try_from(month).ok()?, day).ok()
}

fn date_text(date: Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        date.month() as u8,
        date.day()
    )
}

fn span(dated: &BTreeMap<&str, Date>) -> (Date, Date) {
    let first = dated.values().min().copied();
    let last = dated.values().max().copied();
    let fallback = Date::from_calendar_date(2000, Month::January, 1).expect("gültiges Datum");
    (first.unwrap_or(fallback), last.unwrap_or(fallback))
}

/// Rechnet ein Datum auf die x-Achse und liefert die Marken dazu.
struct Scale {
    first: Date,
    days: f32,
}

impl Scale {
    fn new(first: Date, last: Date) -> Self {
        let days = (last - first).whole_days().max(1) as f32;
        Self { first, days }
    }

    fn x(&self, date: Date) -> f32 {
        let offset = (date - self.first).whole_days() as f32;
        MARGIN_X + offset / self.days * TIMELINE_WIDTH
    }

    /// Eine Marke je Monat, bei langen Zeiträumen je Quartal oder Jahr — sonst
    /// steht die Achse voller Text.
    fn ticks(&self) -> Vec<MetroTick> {
        let months = (self.days / 30.0).ceil() as i32;
        let step = match months {
            0..=18 => 1,
            19..=48 => 3,
            _ => 12,
        };
        let mut ticks = Vec::new();
        let mut year = self.first.year();
        let mut month = self.first.month() as i32;
        for _ in 0..=(months / step + 1) {
            let Some(date) = Month::try_from(month as u8)
                .ok()
                .and_then(|m| Date::from_calendar_date(year, m, 1).ok())
            else {
                break;
            };
            if date >= self.first {
                ticks.push(MetroTick {
                    label: if step == 12 {
                        year.to_string()
                    } else {
                        format!("{:02}/{year}", month)
                    },
                    x: self.x(date),
                });
            }
            month += step;
            while month > 12 {
                month -= 12;
                year += 1;
            }
        }
        ticks
    }
}
