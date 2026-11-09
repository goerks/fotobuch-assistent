# Übergabe: Stand und offene Punkte

Stand: 9. November 2026. Lies zuerst `docs/arbeitsweise.md`.

## Zugänge

- Repository: https://github.com/goerks/fotobuch-assistent (öffentlich), Branch `main`.
- Webseite: https://goerks.github.io/fotobuch-assistent/de/ und `/en/` (GitHub Pages über Actions).
- Release v0.1.0 ist veröffentlicht. Alles seit Schritt 1 ist noch **nicht** veröffentlicht (nächste Version v0.2.0, nur mit Zustimmung des Auftraggebers).
- Design-Artifact «Fotobuch: Ablauf verbessern» (Boards je Schritt) und die Rahmen-Auswahl liegen im Claude-Konto des Auftraggebers. Die Quelltexte sind in `docs/design/` gesichert. Mit einem neuen Konto die Boards daraus neu veröffentlichen.

## Umgesetzt (Details in `docs/fortschritt.md`)

| Schritt | Inhalt | Bild |
|---|---|---|
| Basis | Rust-Kern, Tauri-Fenster, Oberfläche, 7 Schritte, Export, Themen (Leinen, Nacht u. a.), Webseite de/en, Release v0.1.0 | |
| 1 Umfang | Bereich «Fotos pro Seite» statt fester Zahl, Seitenplanung (`umfang.rs`), Ziel-Seitenzahl | `01-umfang.png` |
| 2 Ereignisse | Regler für Feinheit, Zeitleiste, automatische Namen, Galerie je Ereignis (überschreibbar) | `02-ereignisse.png` |
| 3 Seitenband | Seiten wählen, verschieben, leere Seiten, Seite entfernen, ganze Ereignisse verschieben, Zurücknehmen | `03-seitenband.png` |
| 4 Aussortieren | Reiter, Serienband, Fortschritt, Qualitätsbalken mit Begründung | `04-aussortieren.png` |
| 5 Druckdienst | Profile Cewe, Pixum, Ifolor (feste Seitenzahlen), Saal Digital, eigene Regeln | `05-druckdienst.png` |
| Webseite | Texte und Bilder auf Stand Schritt 1 bis 5 | |

Tests: Kern (26 Unit, 3 Integration), jsdom (14), E2E. CI war nach Schritt 5 grün.

## Offen, in dieser Reihenfolge

1. **Schritt 6 Rahmen** (Design-Vorschlag gezeigt, Bestätigung steht aus): siehe `docs/ideen/rahmen.md`. Etappe A (Santorini, Bogenfenster, Goldrahmen, Eichenrahmen, Schwarz mit Passepartout), danach Etappe B. Später Rahmen pro Ereignis oder Seite.
2. **Release v0.2.0** veröffentlichen, wenn der Auftraggeber zustimmt.
3. **Test mit echten Fotos** des Auftraggebers (bisher nur synthetische Demo-Fotos).
4. E2E-Lauf nach den letzten Änderungen (Ereignis verschieben, Druckdienst) nochmals ausführen.
5. **Druckdienst-Regeln prüfen:** Werte stammen von Anbieter-Seiten und einem Testbericht (Stand 30.10.2026). Cewe-Schrittweite nur aus dem Testbericht.
6. Robustheit bei über 10'000 Fotos (Tempo, Speicher).
7. Installer und Signierung (Windows- und Mac-Warnungen beim Start).
8. Weitere Themen und Anpassungen in der Gestaltung.
9. Kleiner Fehler: Blättern-Knopf ist an einer Stelle abgeschnitten.
10. Lokale KI-Modelle: nur auf Auftrag.

## Bekannte Schwächen

- Ein Seitenschlüssel hängt am ersten Foto der Seite. Von Hand verschobene Seiten können sich lösen, wenn sich das erste Foto ändert.
- Die Doppelseiten-Ausrichtung (Foto über zwei Seiten) kann eine Verschiebung von Hand überstimmen.
