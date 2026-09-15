//! Der Notion-Token: Herkunft, Prüfung, Speicher.
//!
//! Der Schlüsselbund des Systems wird hier nie berührt — die Tests benutzen
//! den Speicher im Arbeitsspeicher und den Dateispeicher.

use vizu_notion_core::secret::{self, FileStore, MemoryStore, SecretStore, TokenOrigin};
use vizu_notion_core::{Error, ErrorCode, Result};

const TOKEN: &str = "ntn_1234567890abcdefghijklmnopqrstuv";

#[test]
fn die_umgebung_hat_vorrang_vor_dem_speicher() {
    let store = MemoryStore::default();
    store.save("ntn_aus_dem_speicher_0000000000").unwrap();

    let token = secret::resolve_with(&store, Some(TOKEN.to_string())).unwrap();
    assert_eq!(token, TOKEN);

    let status = secret::status_with(&store, Some(TOKEN.to_string()));
    assert_eq!(status.origin, Some(TokenOrigin::Environment));
}

#[test]
fn ohne_token_gibt_es_einen_eigenen_fehler() {
    let err = secret::resolve_with(&MemoryStore::default(), None).unwrap_err();
    assert_eq!(err.code(), ErrorCode::TokenMissing);
    assert!(err.to_string().contains("token set"), "{err}");
}

#[test]
fn speichert_bereinigt_und_zeigt_nur_einen_hinweis() {
    let store = MemoryStore::default();
    let status = secret::set(&store, &format!("  {TOKEN}\n")).unwrap();

    assert_eq!(store.load().unwrap().as_deref(), Some(TOKEN));
    assert_eq!(status.origin, Some(TokenOrigin::Store));
    assert_eq!(status.hint.as_deref(), Some("ntn_…stuv"));

    // Der Token steht nirgends in dem, was eine Schale ausgeben könnte.
    let shown = format!("{status:?}");
    assert!(!shown.contains("1234567890"), "{shown}");
}

#[test]
fn lehnt_offensichtlich_falsches_ab() {
    let store = MemoryStore::default();
    for raw in ["", "   ", "ntn_abc def ghi jkl mno pqr", "zu-kurz"] {
        let err = secret::set(&store, raw).unwrap_err();
        assert_eq!(err.fields().unwrap()[0].field, "token", "{raw:?}");
    }
    assert_eq!(store.load().unwrap(), None, "nichts gespeichert");
}

#[test]
fn loeschen_meldet_ob_etwas_da_war() {
    let store = MemoryStore::default();
    assert!(!secret::clear(&store).unwrap());
    secret::set(&store, TOKEN).unwrap();
    assert!(secret::clear(&store).unwrap());
    assert_eq!(secret::status_with(&store, None).origin, None);
}

#[test]
fn der_dateispeicher_ist_nur_fuer_den_besitzer_lesbar() {
    let dir = tempfile::tempdir().unwrap();
    let store = FileStore::new(dir.path().join("sub/token"));

    secret::set(&store, TOKEN).unwrap();
    assert_eq!(store.load().unwrap().as_deref(), Some(TOKEN));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(dir.path().join("sub/token"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    assert!(secret::clear(&store).unwrap());
    assert_eq!(store.load().unwrap(), None);
}

struct Broken;

impl SecretStore for Broken {
    fn load(&self) -> Result<Option<String>> {
        Err(Error::SecretStore("Secret Service nicht erreichbar".into()))
    }
    fn save(&self, _: &str) -> Result<()> {
        Err(Error::SecretStore("Secret Service nicht erreichbar".into()))
    }
    fn delete(&self) -> Result<bool> {
        Ok(false)
    }
    fn describe(&self) -> String {
        "kaputt".into()
    }
}

#[test]
fn ein_unerreichbarer_speicher_wird_benannt_statt_verschwiegen() {
    let status = secret::status_with(&Broken, None);
    assert_eq!(status.origin, None);
    assert!(status.store_error.unwrap().contains("Secret Service"));

    let err = secret::resolve_with(&Broken, None).unwrap_err();
    assert_eq!(err.code(), ErrorCode::TokenMissing);
    assert!(err.to_string().contains("Secret Service"), "{err}");

    assert_eq!(
        secret::set(&Broken, TOKEN).unwrap_err().code(),
        ErrorCode::SecretStoreUnavailable
    );
}

#[test]
fn der_hinweis_verraet_nicht_zu_viel() {
    assert_eq!(
        secret::hint("secret_abcdefghijklmnopqrstuvwxyz1234"),
        "secret_…1234"
    );
    assert_eq!(secret::hint("abcdefghijklmnopqrstuvwxyz"), "…wxyz");
    assert_eq!(secret::hint("kurz"), "…");
}
