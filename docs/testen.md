# Testkonzept

Ziel: Jede Änderung wird automatisch geprüft, ohne echte Fotos. Was automatisch nicht prüfbar ist, steht unten ehrlich als offen.

## Ebenen

| Ebene | Wo | Was wird geprüft |
|---|---|---|
| Kern | `tests/test_kern.py` (17 Tests) | EXIF, Drehung, HEIC, Ereignisse, Duplikate, Serien (inkl. Unschärfe, Kettenbildung), Seitenmathematik, Profile, teilbarer Bericht ohne personenbezogene Daten, Export |
| App-Logik | `tests/test_app.py` | Entscheidungen und Neustart, Zwischenspeicher, defekter Zustand, Einstellungsprüfung, Auffüllen/Kürzen auf die Seitenstufe genau, Export ohne Überschreiben, defekte Bilder, leerer Ordner |
| Sicherheit | `tests/test_app.py` | Token-Pflicht, Host-Prüfung (nur 127.0.0.1/localhost), JSON-Pflicht, kein Ausbruch aus dem Web-Ordner, Pfadeingaben |
| Oberfläche | `tests/ui/app.test.mjs` (Node, jsdom) | Die echte App wird gestartet und wie ein Nutzer bedient: Ordner öffnen, Analyse, Ereignisse benennen, Serien per Klick und Tastatur, Zähler, Seitenregeln, Export |
| Ende-zu-Ende | `tests/e2e/test_album.py` (pytest + Chromium) | Demo-Reise (~300 Fotos mit EXIF/GPS, Serien, Duplikate, Screenshots) durch die ganze App: Analyse, Ereignisse benennen/zusammenlegen/teilen, Foto entfernen, jede Buch-Doppelseite blättern (alle Bilder geladen, jedes Buchfoto genau einmal), Export, Originale unverändert (Prüfsummen), Neustart behält Änderungen. Seiten- und PDF-Export, Ganzseiten, Layout-Vorlagen. Ergebnis: Bild je Doppelseite, Galerie (`galerie/`: ein Bild je Schritt der App) und `album-kontaktbogen.png` (CI-Artefakt `album-ende-zu-ende`) |
| Paket | CI-Schritt «Selbsttest der exe» | Die gebaute Windows-exe startet und liefert die Oberfläche aus |

Testbilder sind synthetisch (`tests/conftest.py`) und enthalten bekannte Eigenschaften (Zeit, Unschärfe, Duplikate), damit Erwartungen exakt sind.

## Ausführen
```
pip install -e ".[raw,dev]"
cargo test -p fotobuch-core   # Kern; Oberfläche: cd tests/ui && npm ci && node --test app.test.mjs
cd tests/ui && npm ci && node --test app.test.mjs
```

## Bekannte Lücken (offen)
- **Echte Fotos:** Die Qualität der Serien- und Bestenauswahl ist nur an synthetischen Bildern geprüft. Die Abnahme an einem echten Ordner ist der wichtigste offene Test.
- **Optik:** jsdom rendert nicht. Layout, Abstände und Lesbarkeit sind nicht automatisch geprüft und müssen angeschaut werden.
- **Windows:** Die Tests laufen in CI unter Windows, die Bedienung der exe (Ordner-Dialog, Browser-Start, SmartScreen-Hinweis) ist noch nicht von Hand geprüft.
- **RAW:** Nur der Codepfad, keine echte RAW-Datei.
- **Anbieter-Regeln:** Seitenzahlen von Cewe/Pixum und Ifolor sind Annahmen.
