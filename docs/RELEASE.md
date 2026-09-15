# Ausliefern

## Kurzfassung

```bash
just bundle-macos      # dist/Starter.app, universal (arm64 + x86_64)
just package-linux     # dist/starter-<version>-<arch>-linux.tar.gz
```

Oder über die CI: ein Tag `v*` baut beides und hängt es an das GitHub-Release.

```bash
git tag v0.1.0 && git push origin v0.1.0
```

Die Fassung steht an genau einer Stelle: `version` unter `[workspace.package]`
in der obersten `Cargo.toml`. Die Skripte lesen sie von dort.

---

## macOS

### Was `bundle-macos.sh` tut

```
dist/Starter.app/
└── Contents/
    ├── Info.plist              Name, Kennung, Fassung, Mindestsystem
    ├── MacOS/
    │   ├── starter-gui         die Oberfläche  (CFBundleExecutable)
    │   └── starter             die Kommandozeile, kommt mit
    └── Resources/
        └── Starter.icns
```

`lipo` verschmilzt die Binaries für `aarch64-apple-darwin` und
`x86_64-apple-darwin` zu je einer Datei, die auf beiden Architekturen läuft.

**Warum kein `cargo-bundle`:** Das Bündel ist ein Verzeichnis mit fünf Dateien
darin. Wer es selbst zusammensetzt, sieht jeden Schritt und kann signieren,
notarisieren und eigene `Info.plist`-Einträge schreiben, ohne gegen die Annahmen
eines halb verwaisten Werkzeugs anzukämpfen.

### Die Falle mit Groß- und Kleinschreibung

macOS-Dateisysteme unterscheiden in der Voreinstellung **nicht** zwischen groß
und klein. Hieße die Oberfläche im Bündel `Starter`, wäre das dieselbe Datei wie
die Kommandozeile `starter` — die zweite Kopie überschriebe die erste, und ein
Doppelklick startete die CLI, die sofort wieder beendet.

Deshalb behalten beide ihren gebauten Namen. Der Anzeigename unter dem Symbol
kommt aus `CFBundleName`, nicht aus dem Dateinamen.

### Die CLI erreichbar machen

```bash
ln -s /Applications/Starter.app/Contents/MacOS/starter /usr/local/bin/starter
```

### Signieren und notarisieren

Das Skript signiert ad hoc (`codesign --sign -`). Das genügt auf dem eigenen
Rechner. Auf einem fremden meldet Gatekeeper „beschädigt" — nicht weil etwas
kaputt ist, sondern weil die Signatur fehlt.

Für die Weitergabe braucht es eine Apple-Developer-ID (kostenpflichtig):

```bash
# 1. Mit der eigenen Kennung signieren, mit Laufzeitabsicherung
codesign --force --deep --options runtime --timestamp \
    --sign "Developer ID Application: Dein Name (TEAMID)" \
    dist/Starter.app

# 2. Zum Einreichen packen — ditto, nicht zip:
#    nur ditto erhält Symlinks und erweiterte Attribute des Bündels.
ditto -c -k --keepParent dist/Starter.app dist/Starter.zip

# 3. Einreichen und auf Apples Antwort warten
xcrun notarytool submit dist/Starter.zip \
    --apple-id "du@example.com" \
    --team-id TEAMID \
    --password "app-spezifisches-passwort" \
    --wait

# 4. Das Ergebnis ans Bündel heften, damit es auch offline gilt
xcrun stapler staple dist/Starter.app

# 5. Erneut packen — jetzt mit angehefteter Bestätigung
rm dist/Starter.zip
ditto -c -k --keepParent dist/Starter.app dist/Starter.zip
```

Schritt 4 nicht auslassen: Ohne `stapler` muss der fremde Rechner beim ersten
Start Apple fragen. Ohne Netz schlägt das fehl.

In der CI kommen die Geheimnisse aus den Repository-Secrets; das
Signierzertifikat muss vorher in einen temporären Schlüsselbund importiert
werden. Die Schritte stehen in `.github/workflows/release.yml` bewusst **nicht**
drin — ein Kit ohne Zertifikat soll ohne Fehlschlag durchlaufen.

---

## Linux

`package-linux.sh` baut einen Tarball:

```
starter-0.1.0-x86_64-linux/
├── bin/{starter,starter-gui}
├── share/applications/starter.desktop
├── share/icons/hicolor/{32,64,128,256,512}x…/apps/starter.png
├── share/man/man1/*.1
├── share/completions/starter.{bash,zsh,fish}
└── install.sh
```

`install.sh` legt alles unter `~/.local` ab — ohne Sonderrechte, wie es die
XDG-Regeln vorsehen — und frischt Startmenü und Symbolzwischenspeicher auf.
Mit `PREFIX=/usr/local ./install.sh` geht es auch systemweit.

Handbuchseiten und Vervollständigungen werden beim Packen von der Anwendung
selbst erzeugt und können deshalb nicht veralten.

**Bewusst kein AppImage, kein Flatpak, kein `.deb`.** Ein Tarball funktioniert
überall und ist nachvollziehbar. Wer ein Distributionspaket braucht, baut es
daraus.

### Systembibliotheken beim Bauen

Die Oberfläche bindet gegen OpenGL und X11/Wayland. Auf einem nackten
Build-Rechner:

```bash
sudo apt-get install -y --no-install-recommends \
    libgl1-mesa-dev libx11-dev libxcursor-dev libxi-dev \
    libxrandr-dev libxkbcommon-dev libwayland-dev
```

Fehlen sie, bricht der Bau mit Meldungen über fehlende `.so`-Dateien ab, nicht
mit einem Rust-Fehler. Die Liste steht auch in `.github/workflows/check.yml`.

Auf dem **Zielrechner** braucht es nichts davon zusätzlich — die Bibliotheken
sind Teil jeder Desktop-Installation. SQLite ist ins Binary einkompiliert.

---

## Was dieses Kit nicht mitbringt

* **Kein Auto-Update.** Das braucht Signaturschlüssel und eine
  Serverinfrastruktur und gehört nicht in einen Startpunkt. Wer es will, sieht
  sich `self_update` oder Sparkle an.
* **Kein Windows.** Nachrüstbar: Ziel in `rust-toolchain.toml`, ein Runner in der
  CI, und in `crates/gui/src/main.rs` der Eintrag
  `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` — sonst
  öffnet sich neben dem Fenster eine Konsole.
