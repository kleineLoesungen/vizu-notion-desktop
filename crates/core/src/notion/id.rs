//! Notion-Kennungen aus dem, was ein Mensch einfügt.

/// Macht aus einer Kennung, einem Seitennamen mit Kennung oder einer ganzen
/// Adresse eine UUID in Kleinbuchstaben mit Bindestrichen.
///
/// Angenommen wird alles, was man aus Notion kopieren kann:
///
/// * `396f66270f5d8034b55cebc685aa5e50`
/// * `396f6627-0f5d-8034-b55c-ebc685aa5e50`
/// * `Vizu-App-3dcf66270f5d803886a1f2481b0968ac` (so steht es in der Adresse)
/// * `https://www.notion.so/team/Projekte-396f…?v=…` — die Ansicht hinter `?v=`
///   ist selbst eine Kennung und wird deshalb zuerst abgeschnitten.
///
/// Gibt `None` zurück, wenn keine Kennung darin steckt.
pub fn parse_id(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let raw = raw.split(['?', '#']).next().unwrap_or(raw);
    let last = raw.trim_end_matches('/').rsplit('/').next().unwrap_or(raw);

    let is_hex32 = |s: &str| s.len() == 32 && s.chars().all(|c| c.is_ascii_hexdigit());

    // Schon eine UUID mit Bindestrichen?
    let compact: String = last.chars().filter(|c| *c != '-').collect();
    let hex = if uuid_shape(last) && is_hex32(&compact) {
        compact
    } else {
        // Sonst das letzte Stück hinter einem Bindestrich: „Name-Name-<id>".
        let tail = last.rsplit('-').next().unwrap_or(last);
        if !is_hex32(tail) {
            return None;
        }
        tail.to_string()
    };

    let h = hex.to_ascii_lowercase();
    Some(format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    ))
}

/// 8-4-4-4-12 Zeichen, durch Bindestriche getrennt.
fn uuid_shape(s: &str) -> bool {
    let parts: Vec<usize> = s.split('-').map(str::len).collect();
    parts == [8, 4, 4, 4, 12]
}

#[cfg(test)]
mod tests {
    use super::parse_id;

    const ID: &str = "396f6627-0f5d-8034-b55c-ebc685aa5e50";

    #[test]
    fn nimmt_alle_ueblichen_schreibweisen() {
        for raw in [
            "396f66270f5d8034b55cebc685aa5e50",
            "396F66270F5D8034B55CEBC685AA5E50",
            "396f6627-0f5d-8034-b55c-ebc685aa5e50",
            "  396f66270f5d8034b55cebc685aa5e50\n",
            "Projekte-396f66270f5d8034b55cebc685aa5e50",
            "Vizu-App-396f66270f5d8034b55cebc685aa5e50",
            "https://www.notion.so/team/Projekte-396f66270f5d8034b55cebc685aa5e50",
            "https://www.notion.so/396f66270f5d8034b55cebc685aa5e50?v=0123456789abcdef0123456789abcdef",
            "https://app.notion.com/p/396f66270f5d8034b55cebc685aa5e50/",
            // So kommt es aus „Link kopieren": mit Ansicht und Herkunft. Hinter
            // `?v=` steht ebenfalls eine 32-stellige Kennung — die der Ansicht.
            // Sie darf nicht gewinnen.
            "https://app.notion.com/p/396f66270f5d8034b55cebc685aa5e50?v=189a030d54c64c7da4248eaf948307d1",
            "https://app.notion.com/p/396f66270f5d8034b55cebc685aa5e50?v=189a030d54c64c7da4248eaf948307d1&source=copy_link",
        ] {
            assert_eq!(parse_id(raw).as_deref(), Some(ID), "{raw}");
        }
    }

    #[test]
    fn ohne_kennung_kein_ergebnis() {
        for raw in [
            "",
            "Projekte",
            "396f66270f5d8034b55cebc685aa5e5",
            "396f66270f5d8034b55cebc685aa5e50ff",
            "zzzf66270f5d8034b55cebc685aa5e50",
        ] {
            assert_eq!(parse_id(raw), None, "{raw}");
        }
    }
}
