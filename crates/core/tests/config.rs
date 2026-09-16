//! Tests der Einstellungsdatei.

use vizu_notion_core::{Config, Paths, Theme};

#[test]
fn fehlende_datei_ergibt_die_voreinstellung() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    assert_eq!(
        Config::load(&paths.config_file()).unwrap(),
        Config::default()
    );
}

#[test]
fn schreiben_und_lesen_ergibt_dasselbe() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    paths.ensure().unwrap();

    let config = Config {
        theme: Theme::Dark,
        accent: "#aa3344".into(),
    };
    config.save(&paths.config_file()).unwrap();

    assert_eq!(Config::load(&paths.config_file()).unwrap(), config);
}

#[test]
fn eine_aeltere_datei_mit_app_name_wird_noch_gelesen() {
    // `app_name` war einmal eine Einstellung. Eine vorhandene Datei damit soll
    // die Anwendung nicht zum Absturz bringen — der Schlüssel wird überlesen
    // und beim nächsten Speichern nicht wieder geschrieben.
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    paths.ensure().unwrap();
    std::fs::write(
        paths.config_file(),
        "app_name = \"Vereinsportal\"\ntheme = \"dark\"\naccent = \"#aa3344\"\n",
    )
    .unwrap();

    let config = Config::load(&paths.config_file()).unwrap();
    assert_eq!(config.theme, Theme::Dark);

    config.save(&paths.config_file()).unwrap();
    let text = std::fs::read_to_string(paths.config_file()).unwrap();
    assert!(!text.contains("app_name"), "{text}");
}

#[test]
fn ein_unbekannter_schluessel_ist_ein_fehler() {
    // Sonst verschwindet ein Tippfehler im Schlüsselnamen unbemerkt und die
    // Einstellung wirkt einfach nicht.
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    paths.ensure().unwrap();
    std::fs::write(paths.config_file(), "app_nme = \"Tippfehler\"\n").unwrap();

    let err = Config::load(&paths.config_file()).unwrap_err();
    assert!(matches!(err, vizu_notion_core::Error::ConfigParse { .. }));
}

#[test]
fn lehnt_eine_kaputte_akzentfarbe_ab() {
    let config = Config {
        accent: "blau".into(),
        ..Default::default()
    };
    let err = config.validate().unwrap_err();
    assert_eq!(err.fields().unwrap()[0].field, "accent");
}

#[test]
fn zerlegt_die_akzentfarbe_in_bytes() {
    let config = Config {
        accent: "#3b6ea5".into(),
        ..Default::default()
    };
    assert_eq!(config.accent_rgb(), [0x3b, 0x6e, 0xa5]);
}

#[test]
fn pfade_liegen_unter_der_angegebenen_wurzel() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    paths.ensure().unwrap();

    assert!(paths.db_file().starts_with(dir.path()));
    assert!(paths.config_file().starts_with(dir.path()));
    assert!(paths.data_dir().is_dir());
    assert!(paths.config_dir().is_dir());
}
