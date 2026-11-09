# Arbeitsweise (Vorgehensmodell)

So arbeiten Auftraggeber und Claude an diesem Projekt zusammen. Wer das Projekt mit einem neuen Claude-Konto weiterführt, liest zuerst diese Datei, dann `docs/uebergabe.md` (Stand und offene Punkte).

## Rahmenbedingungen des Produkts

- Lokal und offline: Kein Hochladen, keine Cloud. Nichts verlässt den Rechner.
- Originale werden nie verändert. Fotos werden nie beschnitten (Rahmen liegen um das Foto).
- Läuft auch auf schwachen PCs. Kern ist Open Source (Rust). Portable Programme: Windows exe, Mac zip (Apple Silicon), Linux tar.gz.
- Oberfläche in Schweizer Hochdeutsch (ss statt ß), Bedienung ohne Fachwörter.
- Ein Ereignis soll möglichst auf eine Doppelseite passen, darf aber eine oder mehrere (auch ungerade) Seiten belegen.
- Pro Ereignis gibt es eine Galerie, die zeigt, welche Fotos es ins Buch geschafft haben. Der Nutzer kann das überschreiben.
- Lokale KI-Modelle: Die Recherche liegt in `docs/ideen/lokale-modelle.md`. Sie wird nur gemerkt und **nicht umgesetzt**, bis der Auftraggeber es sagt.

## Der Ablauf je Schritt

Jede Verbesserung wird als nummerierter Schritt gemacht:

1. **Analyse.** Den heutigen Ablauf durchgehen: Was passt zur Vision, was ist umständlich? Bei grösseren Themen 3 bis 4 Vorschläge machen.
2. **Design-Vorschlag.** Vor jedem Schritt ein Board im Design-Artifact (`docs/design/canvas/`): Vorher links, Erklärung in der Mitte (Was ist besser? Was bleibt? Was ist zu entscheiden?), Nachher rechts. Der Auftraggeber bestätigt. Ohne Bestätigung wird nicht gebaut.
3. **Umsetzen.** Nach der Bestätigung direkt umsetzen und am Ende melden. Nicht umkehrbare Entscheidungen (zum Beispiel ein Release) nur mit ausdrücklicher Zustimmung.
4. **Tests.** Kern (`cargo test -p fotobuch-core`), Oberfläche (jsdom) und bei grösseren Änderungen Ende-zu-Ende (siehe `docs/testen.md`). Neue Funktion heisst neuer Test.
5. **Vorher/Nachher-Bild.** `tools/vorher_nachher.py` erzeugt `docs/galerie/schritte/NN-name.png` (Vorher links, Erklärung in der Mitte, Nachher rechts). Das Bild wird angeschaut, bevor es abgegeben wird.
6. **Doku.** Abschnitt in `docs/fortschritt.md`, bei Bedarf `docs/handbuch.md` und die Webseitentexte (`tools/web_build.py`).
7. **Commit und Push** auf `main`, dann die CI starten und prüfen (siehe unten).
8. **Bericht.** Kurz, auf Deutsch: Was ist neu, was wurde geprüft, was nicht, Link zum Vorher/Nachher-Bild, der nächste sinnvolle Schritt.

## Kommunikation

- Der Auftraggeber liest meist auf dem Handy: kurze Antworten, Links statt Dateipfade, Bilder über GitHub verlinken.
- Ehrlich berichten: Was geprüft wurde und was nicht, auch Quellen der Daten (zum Beispiel bei Druckdienst-Regeln).
- Bei langen Arbeiten zwischendurch kurz melden, was gerade läuft.

## Werkzeuge und Befehle

```
cargo test -p fotobuch-core                                   # Kerntests
cargo build --release -p fotobuch-core --bin fotobuch-dev     # nötig vor Oberflächen- und E2E-Tests (die Oberfläche ist ins Programm eingebettet)
cd tests/ui && npm ci && node --test app.test.mjs             # Oberfläche (jsdom)
FOTOBUCH_E2E_THEMEN=leinen,nacht python3 -m pytest tests/e2e  # Ende-zu-Ende, rund 2 Minuten
python3 tools/vorher_nachher.py vorher.png nachher.png ziel.png "Titel" "Punkt 1" "Punkt 2"
python3 tools/web_screens.py <demo-ordner> <ziel>              # Webseiten-Screenshots
python3 tools/web_build.py <screenshot-ordner>                 # Webseite neu bauen (website/de, website/en)
```

CI (`.github/workflows/build-exe.yml`) startet nicht bei jedem Push. Manuell starten und abwarten:

```
gh api -X POST repos/goerks/fotobuch-assistent/actions/workflows/build-exe.yml/dispatches -f ref=main
gh api repos/goerks/fotobuch-assistent/actions/runs/<id>/jobs --jq '.jobs[]|[.name,.status,.conclusion]'
```

Release nur mit Zustimmung: derselbe Aufruf mit `-f 'inputs[veroeffentlichen]=vX.Y.Z'` (der Release-Job legt Tag und Release an; ein direkter Tag-Push wird blockiert). Die Webseite (`.github/workflows/webseite.yml`) wird bei Änderungen unter `website/` automatisch veröffentlicht: https://goerks.github.io/fotobuch-assistent/de/

## Fallstricke (bereits erlebt)

- Nach Änderungen an `ui/` den Release-Build neu machen, sonst testen jsdom und E2E die alte Oberfläche.
- Im E2E-Test und bei Screenshots nur Bilder im sichtbaren Bereich abwarten (lazy loading). Für frische Starts den Ordner `.fotobuch` im Demo-Ordner löschen.
- `pkill -f "fotobuch-dev serve"` beendet auch den eigenen Befehl: anders beenden.
- Der Mac-Job der CI wird manchmal abgebrochen: `gh api -X POST repos/goerks/fotobuch-assistent/actions/runs/<id>/rerun-failed-jobs`.
- `.gitignore` schliesst Bilder aus. Ausnahmen stehen am Dateiende (App-Icons, Webseitenbilder, Vorher/Nachher-Bilder). Neue Bildordner dort eintragen.
- Echte Fotos gehören nie ins Repository.

## Design-Artifacts

Boards liegen als Quelltext in `docs/design/canvas/`. Neues Board: Datei `SchrittN.dc.html` nach dem Muster eines bestehenden, in `canvas.json` eintragen, mit dem Artifact-Werkzeug ins Design-Artifact veröffentlichen (`root` = Ordner mit `project/`). Bilder dafür vorher als Asset hochladen. Für reine Auswahl-Aufgaben des Auftraggebers (zum Beispiel Rahmen) hat sich eine eigene interaktive HTML-Seite mit «Text kopieren» bewährt: `docs/design/rahmen-auswahl.html`.
