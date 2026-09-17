//! Die Metro-Karte: Ketten, Zeitachse, Bänder.

mod common;

use std::collections::HashSet;

use common::{FixtureNotion, Ids, client};
use vizu_notion_core::metro::{self, StationKind};
use vizu_notion_core::source::{self, ColumnMapping, Source, SourceInput};
use vizu_notion_core::{App, fetch};

fn app_mit_projekten() -> (App, Source) {
    app_mit("Projekte", "projekte")
}

/// Die Roadmap: gedacht für die Karte — mit Abzweigungen, Einmündungen und
/// anderthalb Jahren Zeitraum. Siehe fixtures/notion/README.md.
fn app_mit_roadmap() -> (App, Source) {
    app_mit("Roadmap", "roadmap")
}

fn app_mit(name: &str, fixture: &str) -> (App, Source) {
    let app = App::in_memory().unwrap();
    let source = source::create(
        app.conn(),
        SourceInput::new(
            name,
            Ids::load().database(fixture),
            vec![
                ColumnMapping::new("title", "Name"),
                ColumnMapping::new("next", "Nächstes"),
                ColumnMapping::new("date", "Start"),
                ColumnMapping::new("tag", "Tags"),
            ],
        ),
    )
    .unwrap();
    let (client, _) = client(FixtureNotion::new());
    let known = fetch::known_titles(app.conn()).unwrap();
    let download = fetch::download(&client, &source, &known).unwrap();
    fetch::store(app.conn(), &download).unwrap();
    (app, source)
}

#[test]
fn braucht_datum_und_nachfolger() {
    let app = App::in_memory().unwrap();
    let nur_datum = source::create(
        app.conn(),
        SourceInput::new(
            "Termine",
            "396f66270f5d8034b55cebc685aa5e50",
            vec![ColumnMapping::new("date", "Start")],
        ),
    )
    .unwrap();
    assert!(!metro::eligible(&nur_datum));

    let (_, beides) = app_mit_projekten();
    assert!(metro::eligible(&beides));
}

#[test]
fn legt_ketten_entlang_der_nachfolger_an() {
    let (app, source) = app_mit_projekten();

    let map = metro::build(app.conn(), source.id, &HashSet::new()).unwrap();

    assert!(!map.lines.is_empty());
    // Jede Seite mit Datum steht in genau einer Linie.
    let mut stations: Vec<&str> = map
        .lines
        .iter()
        .flat_map(|l| l.stations.iter().map(|s| s.id.as_str()))
        .collect();
    let count = stations.len();
    stations.sort_unstable();
    stations.dedup();
    assert_eq!(stations.len(), count, "eine Seite steht doppelt");

    // Zwei Projekte haben kein Datum — die Karte sagt das, statt sie zu
    // verschlucken.
    assert_eq!(map.undated.len(), 2);
    assert_eq!(count + map.undated.len(), 12);

    for line in &map.lines {
        let stations = &line.stations;
        if stations.len() == 1 {
            // Eine Seite ohne Nachbarn steht für sich.
            assert_eq!(stations[0].kind, StationKind::Single);
            continue;
        }
        assert_eq!(stations.first().unwrap().kind, StationKind::Start);
        assert_eq!(stations.last().unwrap().kind, StationKind::Terminus);
    }
}

#[test]
fn abzweigung_und_einmuendung_haengen_an_einer_station() {
    let (app, source) = app_mit_roadmap();
    let map = metro::build(app.conn(), source.id, &HashSet::new()).unwrap();

    // „Architektur" zeigt auf zwei Nachfolger: Der zweite beginnt eine eigene
    // Linie — die aber an der Station hängt, von der sie abzweigt, statt in
    // der Luft. Ebenso münden „Rollout EU" und „Rollout US" in dieselbe
    // Station „Version 1.0".
    let stellen: Vec<(f32, f32)> = map
        .lines
        .iter()
        .flat_map(|l| l.stations.iter().map(|s| (s.x, s.y)))
        .collect();

    let uebergaenge: Vec<_> = map
        .lines
        .iter()
        .flat_map(|l| [l.entry, l.exit])
        .flatten()
        .collect();
    assert!(
        uebergaenge.len() >= 3,
        "zu wenige Übergänge: {uebergaenge:?}"
    );
    for punkt in uebergaenge {
        assert!(
            stellen.contains(&(punkt.x, punkt.y)),
            "der Übergang {punkt:?} liegt auf keiner Station"
        );
    }

    // Die Abzweigung beginnt später als der Punkt, an dem sie abzweigt.
    for line in &map.lines {
        if let (Some(entry), Some(first)) = (line.entry, line.stations.first()) {
            assert!(entry.x <= first.x, "{entry:?} liegt rechts von {first:?}");
        }
    }
}

#[test]
fn die_roadmap_spannt_ueber_zwei_jahre() {
    let (app, source) = app_mit_roadmap();
    let map = metro::build(app.conn(), source.id, &HashSet::new()).unwrap();

    assert_eq!(map.all_nodes.len(), 17);
    // „Ideensammlung" hat kein Datum.
    assert_eq!(map.undated.len(), 1);
    assert!(map.ticks.iter().any(|t| t.label.ends_with("2026")));
    assert!(map.ticks.iter().any(|t| t.label.ends_with("2027")));
    // Vier Tags, vier Bänder — dazu eines für die Linien ohne Tag.
    assert!(map.zones.len() >= 4, "{:?}", map.zones);
}

#[test]
fn stationen_stehen_nach_ihrem_datum_nebeneinander() {
    let (app, source) = app_mit_projekten();
    let map = metro::build(app.conn(), source.id, &HashSet::new()).unwrap();

    let mut alle: Vec<(&str, f32)> = map
        .lines
        .iter()
        .flat_map(|l| l.stations.iter().map(|s| (s.date.as_str(), s.x)))
        .collect();
    alle.sort_by(|a, b| a.0.cmp(b.0));
    for pair in alle.windows(2) {
        let (earlier, later) = (pair[0], pair[1]);
        if earlier.0 == later.0 {
            assert_eq!(earlier.1, later.1, "gleiches Datum, andere Stelle");
        } else {
            assert!(
                earlier.1 < later.1,
                "{earlier:?} steht nicht links von {later:?}"
            );
        }
    }

    // Eine Spur je Linie, keine zwei Linien übereinander.
    let mut spuren: Vec<f32> = map.lines.iter().map(|l| l.stations[0].y).collect();
    spuren.sort_by(|a, b| a.partial_cmp(b).unwrap());
    spuren.dedup();
    assert_eq!(spuren.len(), map.lines.len());
}

#[test]
fn die_zeitachse_bekommt_marken() {
    let (app, source) = app_mit_projekten();
    let map = metro::build(app.conn(), source.id, &HashSet::new()).unwrap();

    assert!(map.ticks.len() >= 2, "{:?}", map.ticks);
    assert!(map.ticks.windows(2).all(|w| w[0].x < w[1].x));
    // Die Daten liegen 2026 — gut ein halbes Jahr, also Monate.
    assert!(
        map.ticks.iter().all(|t| t.label.ends_with("2026")),
        "{:?}",
        map.ticks
    );
    assert!(map.width > 0.0 && map.height > 0.0);
}

#[test]
fn baender_fassen_linien_mit_demselben_tag_zusammen() {
    let (app, source) = app_mit_projekten();
    let map = metro::build(app.conn(), source.id, &HashSet::new()).unwrap();

    assert!(!map.zones.is_empty(), "kein Band aus den Tags");
    for zone in &map.zones {
        assert!(!zone.label.is_empty());
        assert!(zone.height >= metro::LANE_HEIGHT);
    }
}

#[test]
fn ausgeblendete_stationen_fehlen_in_den_linien() {
    let (app, source) = app_mit_projekten();
    let alle = metro::build(app.conn(), source.id, &HashSet::new()).unwrap();
    let erste = alle.lines[0].stations[0].id.clone();
    let hidden: HashSet<String> = [erste.clone()].into_iter().collect();

    let gefiltert = metro::build(app.conn(), source.id, &hidden).unwrap();

    assert!(
        gefiltert
            .lines
            .iter()
            .all(|l| l.stations.iter().all(|s| s.id != erste))
    );
    // Die Liste fürs Filterfeld behält alle Seiten.
    assert_eq!(gefiltert.all_nodes.len(), 12);
}

#[test]
fn liest_die_datumsformate_von_notion() {
    use time::Month;
    let date = |y, m, d| time::Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap();

    assert_eq!(metro::parse_date("2026-04-01"), Some(date(2026, 4, 1)));
    assert_eq!(metro::parse_date("2026-04"), Some(date(2026, 4, 1)));
    assert_eq!(metro::parse_date("2026"), Some(date(2026, 1, 1)));
    // Notion hängt bei genauen Zeiten noch die Uhrzeit an.
    assert_eq!(
        metro::parse_date("2026-04-01T09:30:00.000+02:00"),
        Some(date(2026, 4, 1))
    );
    assert_eq!(metro::parse_date(""), None);
    assert_eq!(metro::parse_date("bald"), None);
    assert_eq!(metro::parse_date("2026-13-01"), None);
}
