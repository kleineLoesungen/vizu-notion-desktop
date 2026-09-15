#!/usr/bin/env bash
# Macht aus dem Kit ein eigenes Projekt.
#
#   ./scripts/new-project.sh notizbuch "Notizbuch"
#
# Ersetzt überall "starter" durch den neuen Namen: Kistennamen, Binaries,
# Umgebungsvariablen, Datenverzeichnis, Bündelkennung, Fenstertitel, Doku.
#
# DIREKT NACH DEM KLONEN AUSFÜHREN, vor jedem `just`-Befehl. Bis dahin heißt
# der Klon noch "starter" und benutzt DASSELBE Datenverzeichnis wie das Kit —
# zwei Projekte mit demselben Namen teilen sich unbemerkt eine Datenbank.
#
# Der ganze Ablauf steht in einer Funktion, die erst am Dateiende gerufen wird.
# Das ist kein Schönheitsentscheid: Bash liest ein Skript stückweise, während es
# läuft. Dieses Skript schreibt aber auch sich selbst um — ohne die Funktion
# läse Bash ab der ersten Ersetzung an der falschen Stelle weiter.
set -euo pipefail

main() {
    cd "$(dirname "$0")/.."

    local slug=${1:-}
    local display=${2:-}

    if [ -z "$slug" ]; then
        cat >&2 <<USAGE
Aufruf: $0 <kurzname> [Anzeigename]

  kurzname     Kleinbuchstaben, Ziffern, Bindestriche. Wird zum Namen des
               Binaries, der Kisten und des Datenverzeichnisses.
  Anzeigename  Steht im Fenstertitel und unter dem Symbol.
               Voreinstellung: kurzname mit großem Anfangsbuchstaben.

Beispiel: $0 notizbuch "Notizbuch"
USAGE
        exit 2
    fi

    if ! printf '%s' "$slug" | grep -Eq '^[a-z][a-z0-9-]*$'; then
        echo "Fehler: '$slug' muss mit einem Kleinbuchstaben beginnen und darf nur" >&2
        echo "        Kleinbuchstaben, Ziffern und Bindestriche enthalten." >&2
        exit 2
    fi

    if [ "$slug" = "starter" ]; then
        echo "Fehler: 'starter' ist der Name des Kits selbst." >&2
        exit 2
    fi

    # Abgeleitete Schreibweisen.
    local underscore=${slug//-/_}
    local upper
    upper=$(printf '%s' "$underscore" | tr '[:lower:]' '[:upper:]')
    if [ -z "$display" ]; then
        display="$(printf '%s' "${slug:0:1}" | tr '[:lower:]' '[:upper:]')${slug:1}"
    fi

    echo "Kurzname:     $slug"
    echo "Anzeigename:  $display"
    echo "Umgebung:     ${upper}_DATA_DIR / ${upper}_CONFIG_DIR"
    echo ""

    # Nur verwaltete Textdateien anfassen. Das schließt target/, dist/ und
    # .git/ von selbst aus.
    local files
    if [ -d .git ]; then
        files=$(git ls-files)
    else
        files=$(find . -type f \
            -not -path './target/*' -not -path './dist/*' \
            -not -path './.git/*' -not -path './.local/*')
    fi

    local file tmp count=0
    tmp=$(mktemp)
    for file in $files; do
        case "$file" in
            *.png|*.icns|*.ico) continue ;;    # Symbole sind keine Textdateien
        esac

        # Reihenfolge ist wichtig: spezifische Schreibweisen zuerst, damit das
        # allgemeine "starter" am Schluss nichts zerlegt, was schon ersetzt ist.
        #
        # Umweg über eine Zwischendatei statt `sed -i`: BSD-sed (macOS) verlangt
        # dort ein Argument, GNU-sed (Linux) verbietet es.
        LC_ALL=C sed \
            -e "s/STARTER_/${upper}_/g" \
            -e "s/starter_/${underscore}_/g" \
            -e "s/starter-desktop-cli/${slug}/g" \
            -e "s/starter-/${slug}-/g" \
            -e "s/de\.kleineloesungen\.starter/de.example.${underscore}/g" \
            -e "s/Starter/${display}/g" \
            -e "s/starter/${slug}/g" \
            "$file" > "$tmp"
        # Nur schreiben, wenn sich etwas geändert hat — sonst wird bei jedem
        # Lauf die Änderungszeit jeder Datei neu gesetzt.
        if ! cmp -s "$tmp" "$file"; then
            cat "$tmp" > "$file"
            count=$((count + 1))
        fi
    done
    rm -f "$tmp"

    # Das Symbol heißt nach dem Projekt.
    if [ -f assets/icons/starter.icns ]; then
        mv assets/icons/starter.icns "assets/icons/${slug}.icns"
    fi

    # Die Umbenennung verschiebt die alphabetische Reihenfolge der `use`-Zeilen:
    # "starter_core" sortiert vor "time" und "uuid", ein Name wie
    # "vorlagentest_core" danach. Ohne diesen Lauf scheitert `just check` beim
    # allerersten Mal an einem reinen Formatierungsunterschied — der denkbar
    # schlechteste erste Eindruck für ein frisches Projekt.
    if command -v cargo >/dev/null 2>&1; then
        cargo fmt --all
        echo "$count Dateien geändert, Importe neu sortiert."
    else
        echo "$count Dateien geändert."
        echo "Hinweis: cargo nicht gefunden — einmal 'cargo fmt --all' nachholen,"
        echo "         sonst meldet 'just check' Formatierungsunterschiede."
    fi
    echo ""
    echo "Weiter:"

    # Ob `origin` weg muss, hängt davon ab, wie dieses Projekt entstanden ist.
    # Über „Use this template" auf GitHub gehört das Repo bereits dem neuen
    # Projekt — dann wäre das Entfernen falsch. Nach einem Klon des Kits zeigt
    # origin noch auf das Kit.
    #
    # Erkannt wird das an der Anzahl der Commits: ein Template-Repo startet mit
    # genau einem, ein Klon bringt die Geschichte des Kits mit. Das überlebt
    # auch die Umbenennung oben, ein Vergleich der origin-URL täte es nicht.
    local commits
    commits=$(git rev-list --count HEAD 2>/dev/null || echo 0)
    if [ "$commits" -gt 1 ]; then
        echo "  git remote remove origin        # der Klon zeigt noch auf das Kit"
    fi

    echo "  just check                      # muss grün sein"
    echo "  just gui"
    echo ""
    echo "assets/icon.svg ist noch das Symbol des Kits — anpassen und danach"
    echo "'just icons' laufen lassen."
}

# `exit 0` MUSS auf dieselbe Zeile: Bash liest den Rest der Datei erst nach der
# Rückkehr aus main — und die Datei hat sich bis dahin selbst umgeschrieben.
# In einer Zeile werden beide Befehle zusammen gelesen, bevor der erste läuft.
main "$@"; exit 0
