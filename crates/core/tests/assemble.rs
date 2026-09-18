//! Bausteine einsetzen, ohne den Aufbau einer Vorlage zu zerbrechen.

use vizu_notion_core::template::{InsertInput, Inserted, insert};

const PFEILE: &str = "{{#each Projekte}}\n  {{#if next}}{{title}} --> {{next}}{{/if}}\n{{/each}}\n";

fn einsetzen(body: &str, cursor: usize, snippet: &str, sources: &[&str]) -> Inserted {
    insert(InsertInput {
        body: body.to_string(),
        cursor: body[..cursor].encode_utf16().count() as u32,
        snippet: snippet.to_string(),
        sources: sources.iter().map(|s| s.to_string()).collect(),
    })
}

/// Die Schreibmarke als Byte-Stelle, zum Vergleichen mit `body`.
fn marke(result: &Inserted) -> usize {
    let mut units = 0;
    for (byte, c) in result.body.char_indices() {
        if units >= result.cursor as usize {
            return byte;
        }
        units += c.len_utf16();
    }
    result.body.len()
}

#[test]
fn in_eine_leere_vorlage_kommen_kopf_und_diagrammart_dazu() {
    let r = einsetzen("", 0, PFEILE, &["Projekte"]);

    assert_eq!(
        r.body,
        format!(
            "---\ntitle: \"Neues Diagramm\"\nsources:\n  - Projekte\n---\nflowchart LR\n{PFEILE}"
        )
    );
    assert_eq!(marke(&r), r.body.len(), "Schreibmarke hinter dem Baustein");
}

#[test]
fn eine_fehlende_quelle_kommt_unter_sources_dazu() {
    let body = "---\ntitle: \"Plan\"\nsources:\n  - Ziele\nstyles:\n  title:\n    shape: rounded\n---\nflowchart TD\n";

    let r = einsetzen(body, body.len(), PFEILE, &["Projekte"]);

    assert!(
        r.body.starts_with(
            "---\ntitle: \"Plan\"\nsources:\n  - Ziele\n  - Projekte\nstyles:\n  title:\n    shape: rounded\n---\n"
        ),
        "{}",
        r.body
    );
    assert!(r.body.ends_with(&format!("flowchart TD\n{PFEILE}")));
}

#[test]
fn eine_schon_genannte_quelle_bleibt_einmal() {
    let body = "---\ntitle: \"Plan\"\nsources:\n  - projekte\n---\nflowchart TD\n";

    let r = einsetzen(body, body.len(), PFEILE, &["Projekte"]);

    // Quellnamen vergleicht die Vorlage ohne Rücksicht auf Groß und klein.
    assert_eq!(r.body.matches("rojekte\n").count(), 1, "{}", r.body);
}

#[test]
fn eine_schreibmarke_im_kopf_setzt_ans_ende() {
    let body = "---\ntitle: \"Plan\"\nsources:\n  - Projekte\n---\nflowchart TD\n  A\n";
    let im_kopf = body.find("Plan").unwrap();

    let r = einsetzen(body, im_kopf, PFEILE, &["Projekte"]);

    assert_eq!(r.body, format!("{body}{PFEILE}"));
}

#[test]
fn eine_schreibmarke_vor_der_diagrammart_setzt_ans_ende() {
    let body = "---\ntitle: \"Plan\"\nsources:\n  - Projekte\n---\nflowchart TD\n  A\n";
    let vor_flowchart = body.find("flowchart").unwrap();

    let r = einsetzen(body, vor_flowchart, PFEILE, &["Projekte"]);

    assert_eq!(r.body, format!("{body}{PFEILE}"));
}

#[test]
fn mitten_in_einer_zeile_beginnt_der_baustein_auf_einer_neuen() {
    let body = "---\ntitle: \"Plan\"\nsources:\n  - Projekte\n---\nflowchart TD\n  A --> B";

    let r = einsetzen(body, body.len(), PFEILE, &["Projekte"]);

    assert_eq!(r.body, format!("{body}\n{PFEILE}"));
    assert_eq!(marke(&r), r.body.len());
}

#[test]
fn an_der_schreibmarke_bleibt_der_rest_dahinter_stehen() {
    let body = "---\ntitle: \"Plan\"\nsources:\n  - Projekte\n---\nflowchart TD\n  A\n  Z\n";
    let vor_z = body.find("  Z").unwrap();

    let r = einsetzen(body, vor_z, PFEILE, &["Projekte"]);

    assert!(
        r.body.ends_with(&format!("  A\n{PFEILE}  Z\n")),
        "{}",
        r.body
    );
    assert_eq!(&r.body[marke(&r)..], "  Z\n");
}

#[test]
fn ohne_diagrammart_kommt_flowchart_lr_dazu() {
    let body = "---\ntitle: \"Plan\"\nsources:\n  - Projekte\n---\n";

    let r = einsetzen(body, body.len(), PFEILE, &["Projekte"]);

    assert_eq!(r.body, format!("{body}flowchart LR\n{PFEILE}"));
}

#[test]
fn das_geruest_wird_in_eine_leere_vorlage_ganz_eingesetzt() {
    let geruest = "---\ntitle: \"Neues Diagramm\"\nsources:\n  - Projekte\n---\nflowchart LR\n{{#each Projekte}}\n  {{title}}\n{{/each}}\n";

    let r = einsetzen("", 0, geruest, &["Projekte"]);

    assert_eq!(r.body, geruest);
}

#[test]
fn das_geruest_bringt_in_eine_fertige_vorlage_keinen_zweiten_kopf() {
    let body = "---\ntitle: \"Plan\"\nsources:\n  - Ziele\n---\nflowchart TD\n";
    let geruest = "---\ntitle: \"Neues Diagramm\"\nsources:\n  - Projekte\n---\nflowchart LR\n{{#each Projekte}}\n  {{title}}\n{{/each}}\n";

    let r = einsetzen(body, body.len(), geruest, &["Projekte"]);

    assert_eq!(r.body.matches("---").count(), 2, "{}", r.body);
    assert_eq!(r.body.matches("flowchart").count(), 1, "{}", r.body);
    assert!(r.body.contains("  - Projekte\n"));
    assert!(
        r.body
            .ends_with("{{#each Projekte}}\n  {{title}}\n{{/each}}\n")
    );
}

#[test]
fn quellen_in_klammern_werden_zur_liste() {
    let body = "---\ntitle: \"Plan\"\nsources: [Ziele]\n---\nflowchart TD\n";

    let r = einsetzen(body, body.len(), PFEILE, &["Projekte"]);

    assert!(
        r.body
            .starts_with("---\ntitle: \"Plan\"\nsources:\n  - Ziele\n  - Projekte\n---\n"),
        "{}",
        r.body
    );
}

#[test]
fn die_schreibmarke_zaehlt_wie_das_textfeld_in_utf16() {
    // Das Emoji ist ein Zeichen, aber zwei UTF-16-Einheiten und vier Bytes.
    let body = "---\ntitle: \"Plan 🚀\"\nsources:\n  - Projekte\n---\nflowchart TD\n  A\n  Z\n";
    let vor_z = body.find("  Z").unwrap();

    let r = einsetzen(body, vor_z, PFEILE, &["Projekte"]);

    assert!(
        r.body.ends_with(&format!("  A\n{PFEILE}  Z\n")),
        "{}",
        r.body
    );
    assert_eq!(&r.body[marke(&r)..], "  Z\n");
}

#[test]
fn das_ergebnis_hat_immer_einen_gueltigen_kopf() {
    for body in [
        "",
        "flowchart TD\n  A\n",
        "---\ntitle: \"Plan\"\nsources: [Ziele]\n---\n",
        "---\ntitle: \"Plan\"\nsources:\n  - Ziele\n---\nflowchart TD\n",
    ] {
        let r = einsetzen(body, body.len(), PFEILE, &["Projekte"]);
        let meta = vizu_notion_core::template::parse_meta(&r.body)
            .unwrap_or_else(|e| panic!("{body:?} → {:?}: {e}", r.body));
        assert!(
            meta.sources.iter().any(|s| s == "Projekte"),
            "{body:?} → {:?}",
            meta.sources
        );
    }
}
