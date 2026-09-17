# Festgehaltene Antworten der Notion-API

Echte Antworten von `api.notion.com`, aufgenommen am 2026-09-16 von drei
Testdatenbanken, die ein Skript unter einer eigenen Testseite angelegt hat.
Die Inhalte sind ausgedacht. Alle IDs — Seiten, Datenbanken, Datenquellen,
Nutzer — sind durch fortlaufende Ersatz-IDs ersetzt (`00000000-…-000000000001`),
**durchgängig über alle Dateien**: Eine Relation zeigt auf dieselbe Ersatz-ID
wie die Seite, auf die sie verweist. `request_id` ist entfernt.

Damit laufen die Tests für `core::notion` und `core::rows` ohne Netz und
ohne Token.

| Ordner | Inhalt |
|---|---|
| `2025-09-03/` | Der Weg dieser Anwendung: `GET /databases/{id}` (`*.database.json`), `GET /data_sources/{id}` (`*.data_source.json`, das Schema), `POST /data_sources/{id}/query` (`*.query.N.json`, je Stapel eine Datei) |
| `2022-06-28/` | Der Weg der Webapp: `POST /databases/{id}/query`. Eingabe für die Node-Referenz in Phase 2 |
| `ids.json` | Welche Ersatz-ID zu welcher Datenbank und Datenquelle gehört |

## Die Datenbanken

| Datenbank | Seiten | Spalten | Besonderheiten |
|---|---|---|---|
| **vizu Ziele** | 5 | `Name` (title), `Beschreibung` (rich_text) | „Reichweite **stärken**": Titel aus zwei Textstücken |
| **vizu Projekte** | 12 | `Name`, `Start` (date), `Tags` (multi_select), `Status` (status), `Phase` (formula), `Ziel` (relation → Ziele), `Nächstes` (relation → Projekte) | zwei Projekte „Website"; `App 🚀`; `Q3 "Review" & [Plan]`; Projekt ohne Ziel; Projekt mit drei Zielen; `Nächstes` mit zwei Zielen; zwei Projekte ohne Datum |
| **vizu Aufgaben** | 130 | `Name`, `Projekt` (relation → Projekte), `Erledigt` (checkbox), `Punkte` (number) | **zwei Stapel** (Blättern); jede 10. heißt „Doppelt"; jede 13. ohne Projekt; einige mit zwei Projekten |

### vizu Roadmap — von Hand gebaut

`roadmap.*.json` ist die eine Ausnahme: **keine Aufnahme, sondern absichtlich
gebaute Daten** in derselben Form wie die echten Antworten. Die drei
aufgenommenen Datenbanken reichen nicht, um die Metro-Karte zu beurteilen —
in ihnen hat nur eine einzige Seite zwei Nachfolger, und der fehlt das Datum.

17 Seiten, Spalten wie bei „vizu Projekte" ohne `Ziel` und `Phase`, Zeitraum
2026-01 bis 2027-03. Was darin steckt:

| Fall | Wo |
|---|---|
| **Abzweigung** | „Architektur" → „Datenmodell" **und** „Design-System"; „Beta" → „Rollout EU" **und** „Rollout US" |
| **Einmündung** | „Betriebshandbuch" und „Rollout US" münden in „Version 1.0" |
| **Umsteigestation** | „Architektur", „Beta", „Version 1.0" |
| **Vier Bänder** | Tags `Plattform`, `Produkt`, `Betrieb`, `Markt` |
| **Einzelne Station** | „Messeauftritt" — ohne Vorgänger und ohne Nachfolger |
| **Ohne Datum** | „Ideensammlung" — steht unter der Karte, nicht in ihr |
| **Jahreswechsel** | „Mobil-App" 2027-01, „Version 1.1" 2027-03 |

Die aufgenommenen Dateien werden nicht von Hand geändert. Neu aufnehmen
heißt: Testdaten neu anlegen, Antworten abrufen, erneut anonymisieren.
