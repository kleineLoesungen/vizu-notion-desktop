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

## Fertige Pakete veröffentlichen

Wer die Anwendung benutzen will, soll sie nicht erst übersetzen müssen. Die
Pakete gehören aber **nicht ins Repository**: Sie sind groß, mit jedem Commit
veraltet, und git behält jede Fassung für immer. Der Ort dafür ist ein
**Release auf GitHub**.

`.github/workflows/release.yml` baut auf beiden Systemen — ein Bündel entsteht
nur dort, wo es laufen soll — und hängt die Ergebnisse an einen Entwurf.

Eine neue Fassung, Schritt für Schritt:

1. `version` unter `[workspace.package]` in der obersten `Cargo.toml` anheben,
   etwa auf `0.2.0`. Die Nummer landet in den Dateinamen und im Programm.
2. `just check` — danach ist auch `Cargo.lock` nachgezogen.
3. Committen und pushen.
4. Das Schild setzen und hochladen:

   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

5. Unter *Actions* warten, bis der Lauf „Release" grün ist (etwa zehn Minuten).
6. Unter *Releases* den Entwurf öffnen, Text prüfen, **Publish release**.

Das Release **nicht vorher von Hand anlegen.** Der Workflow legt es selbst an.
Gibt es schon eins mit dem Namen, lädt er die Pakete dort hinein — ein von Hand
veröffentlichtes Release ist dann aber schon öffentlich, bevor die Dateien da
sind.

Dass der Workflow nur einen Entwurf anlegt, ist Absicht: Was unsigniert ist,
soll ein Mensch bewusst freigeben.

Die Webseite (`docs/site/index.html`) muss dafür nicht angepasst werden: Ihre
Download-Knöpfe fragen beim Laden das neueste Release ab.

Gebaut wird auf **Ubuntu 22.04**, nicht auf der neuesten Fassung: Ein Binary
läuft nur mit einer glibc, die mindestens so neu ist wie die, gegen die es
gebaut wurde. Auf 24.04 gebaut, startete es auf älteren Systemen nicht mehr.

Es entstehen je Lauf (GitHub ersetzt Leerzeichen im Namen durch Punkte):

| Datei | Für |
|---|---|
| `Vizu.Notion_<version>_aarch64.dmg` | macOS auf Apple Silicon |
| `Vizu.Notion_<version>_amd64.deb` | Debian, Ubuntu |
| `Vizu.Notion_<version>_amd64.AppImage` | andere Linux-Systeme |
| `vizu-notion-cli-<version>-<os>-<arch>.tar.gz` | nur die Kommandozeile |

**Was fehlt, und was das für Benutzer heißt:**

* **Kein Intel-Mac und kein ARM-Linux.** Nachrüstbar über weitere Einträge in
  der Matrix (`macos-13` für Intel) oder `bundle-macos-universal`.
* **Nicht signiert.** macOS meldet beim ersten Start „beschädigt"; es hilft
  `xattr -dr com.apple.quarantine "/Applications/Vizu Notion.app"`. Wer das
  vermeiden will, braucht ein Entwicklerkonto und die Schritte oben —
  die Schlüssel gehören dann als Secrets in die Actions.
* **Kein Auto-Update.** Eine neue Fassung holt man sich selbst.

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
