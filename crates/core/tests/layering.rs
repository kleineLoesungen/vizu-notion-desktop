//! Der Wächter über die Schichtregel.
//!
//! Dieser Test ist das Gegenstück zum Routenschutz-Test der Web-Kits: Er hält
//! eine Architekturentscheidung fest, die sonst niemand bemerkt, wenn sie
//! verletzt wird. Der Code kompiliert ja trotzdem.
//!
//! ```text
//! core  →  darf nichts von clap, egui, eframe oder stdout wissen
//! cli   →  darf nichts von egui oder eframe wissen
//! gui   →  darf nichts von clap wissen
//! ```
//!
//! **Diesen Test nicht lockern.** Wenn er anschlägt, steht der Code an der
//! falschen Stelle — die Regel ist nicht das Problem. Der einzige zulässige
//! Grund, ihn anzufassen, ist eine neue Kiste im Arbeitsbereich.

use std::path::{Path, PathBuf};

/// Was wo nichts zu suchen hat.
///
/// Jeder Eintrag: (Verzeichnis unter `crates/`, verbotene Zeichenfolge,
/// Begründung für den Menschen, der den Fehlschlag liest).
const RULES: &[(&str, &str, &str)] = &[
    // --- core kennt keine Schale ------------------------------------------
    ("core/src", "clap", "Argumentparsen gehört nach crates/cli"),
    ("core/src", "egui", "Zeichnen gehört nach crates/gui"),
    (
        "core/src",
        "eframe",
        "Fensterverwaltung gehört nach crates/gui",
    ),
    (
        "core/src",
        "println!",
        "core schreibt nicht auf stdout — Werte zurückgeben, die Schale gibt aus",
    ),
    (
        "core/src",
        "eprintln!",
        "core schreibt nicht auf stderr — Error zurückgeben, die Schale meldet",
    ),
    ("core/src", "print!", "siehe println!"),
    ("core/src", "eprint!", "siehe eprintln!"),
    (
        "core/src",
        "std::process::exit",
        "core beendet das Programm nicht — Error zurückgeben",
    ),
    (
        "core/src",
        "dbg!",
        "dbg! ist zum Wegwerfen, nicht zum Einchecken",
    ),
    // --- die Schalen kennen einander nicht --------------------------------
    ("cli/src", "egui", "die Kommandozeile zeichnet nicht"),
    ("cli/src", "eframe", "die Kommandozeile öffnet kein Fenster"),
    ("gui/src", "clap", "die Oberfläche parst keine Argumente"),
    // --- Ansichten zeichnen nur -------------------------------------------
    //
    // Das ist die wichtigste Regel der Oberfläche: `ui()` läuft viele Male pro
    // Sekunde. Ein Schreibzugriff in einer Zeichenfunktion liefe genauso oft.
    // Der Weg nach draußen ist die Action, angewendet in app.rs.
    (
        "gui/src/views",
        "starter_core::App",
        "eine Ansicht kennt die Anwendung nicht — Action zurückgeben, app.rs wendet sie an",
    ),
    (
        "gui/src/views",
        "note::create",
        "Schreiben beim Zeichnen — stattdessen Action::SaveDraft",
    ),
    (
        "gui/src/views",
        "note::update",
        "Schreiben beim Zeichnen — stattdessen Action::SaveDraft",
    ),
    (
        "gui/src/views",
        "note::delete",
        "Schreiben beim Zeichnen — stattdessen Action::ConfirmDelete",
    ),
    (
        "gui/src/views",
        "set_config",
        "Schreiben beim Zeichnen — stattdessen Action::SaveConfig",
    ),
    (
        "gui/src/views",
        "conn()",
        "eine Ansicht sieht keine Datenbankverbindung",
    ),
];

#[test]
fn schichtregel_wird_eingehalten() {
    let mut verstoesse = Vec::new();

    for (dir, forbidden, why) in RULES {
        for file in rust_files(&crates_dir().join(dir)) {
            let text = std::fs::read_to_string(&file).expect("Quelldatei lesbar");
            for (nr, line) in text.lines().enumerate() {
                if code_only(line).contains(forbidden) {
                    verstoesse.push(format!(
                        "  {}:{}\n    enthält {forbidden:?} — {why}\n    > {}",
                        relative(&file),
                        nr + 1,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        verstoesse.is_empty(),
        "Die Schichtregel aus CLAUDE.md ist verletzt:\n\n{}\n\n\
         Den Code verschieben, nicht diesen Test ändern.",
        verstoesse.join("\n")
    );
}

/// Die Abhängigkeiten in `Cargo.toml` sind der zweite Riegel.
///
/// Der Textscan oben lässt sich mit einem Alias umgehen (`use egui as ui;`).
/// Ohne Eintrag in `Cargo.toml` kompiliert aber gar nichts davon.
#[test]
fn abhaengigkeiten_bleiben_getrennt() {
    let mut verstoesse = Vec::new();

    for (crate_name, forbidden, why) in RULES {
        // Nur Kistennamen prüfen — keine Makros, Pfade oder Methodenaufrufe.
        if forbidden.contains('!') || forbidden.contains(':') || forbidden.contains('(') {
            continue;
        }
        // Erster Pfadteil ist der Kistenname ("gui/src/views" → "gui").
        let crate_name = crate_name.split('/').next().expect("nicht leer");
        let manifest = crates_dir().join(crate_name).join("Cargo.toml");
        let text = std::fs::read_to_string(&manifest).expect("Cargo.toml lesbar");
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            // Ein Abhängigkeitseintrag beginnt mit dem Kistennamen.
            if line.starts_with(forbidden) {
                verstoesse.push(format!(
                    "  crates/{crate_name}/Cargo.toml hängt von {forbidden:?} ab — {why}"
                ));
            }
        }
    }

    assert!(
        verstoesse.is_empty(),
        "Verbotene Abhängigkeit:\n\n{}",
        verstoesse.join("\n")
    );
}

// --- Hilfsmittel -----------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR ist crates/core.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/core liegt zwei Ebenen unter der Wurzel")
        .to_path_buf()
}

fn crates_dir() -> PathBuf {
    workspace_root().join("crates")
}

fn relative(p: &Path) -> String {
    p.strip_prefix(workspace_root())
        .unwrap_or(p)
        .display()
        .to_string()
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out; // Kiste hat (noch) kein src/ — nichts zu prüfen.
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// Kommentare abschneiden, damit die Doku über die Regel die Regel nicht bricht.
///
/// Absichtlich einfach gehalten: ein `//` innerhalb einer Zeichenkette würde
/// den Rest der Zeile verschlucken. Das Ergebnis wäre ein übersehener Verstoß,
/// kein falscher Alarm — und in einer Kiste ohne URLs im Code kommt der Fall
/// nicht vor.
fn code_only(line: &str) -> &str {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*") {
        return "";
    }
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}
