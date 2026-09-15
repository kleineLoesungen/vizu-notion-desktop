//! Der Notion-Client: Takt, Wiederholungen, Fehler.

mod common;

use std::time::Duration;

use common::{Scripted, client, ok, status};
use serde_json::json;
use vizu_notion_core::notion::{Client, Response};
use vizu_notion_core::{Error, ErrorCode, NotionErrorKind};

fn rate_limited(seconds: u64) -> Response {
    Response {
        retry_after: Some(Duration::from_secs(seconds)),
        ..status(
            429,
            "rate_limited",
            "This request exceeds the number of requests allowed.",
        )
    }
}

#[test]
fn wartet_bei_429_so_lange_wie_notion_sagt() {
    let transport = Scripted::new(vec![rate_limited(3), ok(json!({ "properties": {} }))]);
    let calls = transport.calls.clone();
    let (client, sleeps) = client(transport);

    client.data_source("ds").unwrap();

    assert_eq!(*calls.lock().unwrap(), 2);
    assert_eq!(*sleeps.lock().unwrap(), vec![Duration::from_secs(3)]);
    assert_eq!(client.requests(), 2);
}

#[test]
fn gibt_nach_fuenf_mal_429_auf() {
    let (client, sleeps) = client(Scripted::new(vec![rate_limited(1)]));

    let err = client.data_source("ds").unwrap_err();

    assert!(matches!(
        err,
        Error::Notion {
            kind: NotionErrorKind::RateLimited,
            ..
        }
    ));
    assert_eq!(err.code(), ErrorCode::NotionError);
    assert_eq!(sleeps.lock().unwrap().len(), 5);
}

#[test]
fn wiederholt_serverfehler_zweimal_mit_laengerer_pause() {
    let transport = Scripted::new(vec![
        status(502, "bad_gateway", "Bad Gateway"),
        status(503, "service_unavailable", "down"),
        ok(json!({})),
    ]);
    let (client, sleeps) = client(transport);

    client.data_source("ds").unwrap();

    assert_eq!(
        *sleeps.lock().unwrap(),
        vec![Duration::from_secs(1), Duration::from_secs(2)]
    );
}

#[test]
fn ein_abgelehnter_token_ist_kein_allgemeiner_fehler() {
    let (client, _) = client(Scripted::new(vec![status(
        401,
        "unauthorized",
        "API token is invalid.",
    )]));

    let err = client.database("db").unwrap_err();

    assert_eq!(err.code(), ErrorCode::NotionUnauthorized);
    assert!(err.to_string().contains("Token"), "{err}");
    assert!(err.to_string().contains("API token is invalid."), "{err}");
}

#[test]
fn andere_fehler_behalten_die_meldung_von_notion() {
    let (client, _) = client(Scripted::new(vec![status(
        400,
        "validation_error",
        "body failed validation",
    )]));
    let err = client.data_source("ds").unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotionError);
    assert_eq!(err.to_string(), "Notion: body failed validation");
}

#[test]
fn haelt_mindestabstand_zwischen_anfragen() {
    let sleeps = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Duration>::new()));
    let record = sleeps.clone();
    let client = Client::with_pacing(
        Scripted::new(vec![ok(json!({}))]),
        Duration::from_secs(60),
        Box::new(move |d| record.lock().unwrap().push(d)),
    );

    client.data_source("a").unwrap();
    client.data_source("b").unwrap();

    let sleeps = sleeps.lock().unwrap();
    assert_eq!(
        sleeps.len(),
        1,
        "vor der ersten Anfrage wird nicht gewartet"
    );
    assert!(sleeps[0] > Duration::from_secs(59));
}

#[test]
fn ueberliest_nicht_seiten_objekte_in_der_abfrage() {
    let (client, _) = client(Scripted::new(vec![ok(json!({
        "results": [
            { "object": "page", "id": "p1", "properties": {} },
            { "object": "data_source", "id": "ds2" }
        ],
        "has_more": false
    }))]));
    let pages = client.query_data_source("ds").unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].id, "p1");
}
