# Ausliefern

## Kurzfassung

```bash
just bundle                   # für diesen Rechner
just bundle-macos-universal   # macOS: Apple Silicon + Intel
just package-cli              # die Kommandozeile als Tarball
```

| System | Ergebnis unter `target/release/bundle/` |
|---|---|
| macOS | `macos/Vizu Notion.app`, `dmg/Vizu Notion_<version>_<arch>.dmg` |
| Linux | `deb/vizu_notion_<version>_<arch>.deb`, `appimage/Vizu Notion_<version>_<arch>.AppImage` |

Universal-Bündel landen unter `target/universal-apple-darwin/release/bundle/`.

Die Fassung steht an genau einer Stelle: `version` unter `[workspace.package]`
in der obersten `Cargo.toml`. Tauri liest sie aus `crates/desktop/Cargo.toml`,
das sie erbt. `package.json` hat eine eigene Nummer, die niemand liest.

---

## macOS

### Was im Bündel steckt

```
Vizu Notion.app/Contents/
├── Info.plist           aus tauri.conf.json: productName, identifier, Mindestsystem 11.0
├── MacOS/vizu-notion-desktop
└── Resources/icon.icns
```

`mainBinaryName` ist ausdrücklich `vizu-notion-desktop`. Hieße die Datei wie der
Anzeigename `Vizu Notion`, wäre sie auf dem Dateisystem, das Groß- und
Kleinschreibung nicht unterscheidet, dieselbe wie eine daneben gelegte CLI
`vizu-notion`.

Die Oberfläche steckt **im Binary** — `ui/dist` wird beim Bau eingebettet.
Ein Bündel ist deshalb nur wenige Megabyte groß; das Webview liefert macOS.

### Die Kommandozeile

Tauri legt keine zweite ausführbare Datei ins Bündel, ohne sie als
„Sidecar" mit Zielplattform im Namen zu verlangen. Die CLI kommt deshalb als
eigener Tarball (`just package-cli`) mit Handbuch und Vervollständigungen.

### `.dmg` auf dem eigenen Rechner

`bundle_dmg.sh` richtet das Fenster des Abbilds über AppleScript im Finder
ein. Hat das Terminal keine Berechtigung, den Finder zu steuern, hängt der
Schritt und bricht ab. Entweder in **Systemeinstellungen → Datenschutz &
Sicherheit → Automation** erlauben, oder ohne Gestaltung bauen:

```bash
CI=true npm run tauri build
```

### Signieren und notarisieren

Ohne Einstellungen signiert Tauri ad hoc. Das genügt auf dem eigenen Rechner.
Auf einem fremden meldet Gatekeeper „beschädigt" oder „nicht verifizierter
Entwickler" — nicht weil etwas kaputt ist, sondern weil die Signatur fehlt.
Übergangsweise: Rechtsklick → Öffnen, oder
`xattr -dr com.apple.quarantine /Applications/Vizu Notion.app`.

Für die Weitergabe braucht es eine Apple-Developer-ID (kostenpflichtig).
Tauri übernimmt Signieren **und** Notarisieren, wenn diese Umgebungsvariablen
gesetzt sind:

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Dein Name (TEAMID)"
export APPLE_ID="du@example.com"
export APPLE_PASSWORD="app-spezifisches-passwort"
export APPLE_TEAM_ID="TEAMID"
just bundle-macos-universal
```

Das Zertifikat muss dafür im Schlüsselbund liegen.

---

## Linux

### Beim Bauen

```bash
# Debian, Ubuntu
sudo apt install libwebkit2gtk-4.1-dev libxdo-dev libssl-dev \
    libayatana-appindicator3-dev librsvg2-dev build-essential file
```

Fehlen sie, bricht der Bau mit Meldungen über fehlende `.pc`-Dateien ab
(`webkit2gtk-4.1 was not found`), nicht mit einem Rust-Fehler.

**Gebaut wird auf der ältesten Distribution, die unterstützt werden soll.**
Ein Binary, das gegen eine neue glibc gebaut ist, startet auf einer älteren
nicht. Für Ubuntu heißt das: auf 22.04 bauen, nicht auf der neuesten Fassung.

### Auf dem Zielrechner

| Paket | Braucht |
|---|---|
| `.deb` | `libwebkit2gtk-4.1-0` und `libgtk-3-0` — trägt Tauri als Abhängigkeit ein, `apt` zieht sie |
| `.AppImage` | nichts; bringt die Bibliotheken mit (dafür deutlich größer) |

Andere Formate (`.rpm`) in `tauri.conf.json` unter `bundle.targets` ergänzen.

### Wayland, NVIDIA und leere Fenster

WebKitGTK zeichnet auf manchen Kombinationen aus Wayland und
NVIDIA-Treibern ein leeres oder flackerndes Fenster. Abhilfe für Betroffene:

```bash
WEBKIT_DISABLE_DMABUF_RENDERER=1 vizu-notion-desktop
```

Wer das dauerhaft braucht, setzt es in `crates/desktop/src/lib.rs` vor dem
Start — nur unter `#[cfg(target_os = "linux")]`, und nur dort.

---

## Was dieses Kit nicht mitbringt

* **Kein Auto-Update.** `tauri-plugin-updater` braucht einen
  Signaturschlüssel und einen Ort, an dem die Update-Beschreibung liegt. Gehört
  nicht in einen Startpunkt.
* **Kein Windows.** Nachrüstbar: auf Windows bauen, `"msi"`/`"nsis"` in
  `bundle.targets`, `icon.ico` über `just icons` wieder erzeugen lassen, und in
  `crates/desktop/src/main.rs`
  `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` —
  sonst öffnet sich neben dem Fenster eine Konsole.
* **Kein Mobil.** Tauri kann iOS und Android; die Symbole dafür löscht
  `just icons` absichtlich.
