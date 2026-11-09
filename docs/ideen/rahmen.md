# Rahmen fürs Fotobuch (Ideen und Entscheidung)

Auftrag: Themen mit Rahmen um die Fotos, zum Beispiel Fensterrahmen im Santorini-Stil, dazu Bilderrahmen in verschiedenen Stilen. Ein Fotobuch **ohne Rahmen** muss immer möglich bleiben. Fotos werden nie beschnitten: Bogen und Läden liegen als Rahmen um das Foto.

Die Auswahl entstand in der interaktiven Seite `docs/design/rahmen-auswahl.html` (17 Stile). Die Rangliste des Auftraggebers (1 = zuerst umsetzen):

1. Santorini (Kalkweiss, Rundbogen, blaue Läden)
2. Bogenfenster (Terrakotta, weisser Rundbogen)
3. Goldrahmen mit Passepartout
4. Eichenrahmen
5. Schwarz mit Passepartout
6. Schwebendes Foto (Schattenfuge)
7. Skandi-Weiss
8. Polaroid
9. Washi-Tape
10. Filmstreifen

Gültigkeit: ganzes Buch. Rahmenlos bei jedem Thema wählbar: ja.

Nicht gewählt, aber als Ideen vorhanden: Riviera (Läden und Wäscheleine), Altes Holzfenster, Loft-Stahlfenster, Shoji, Barock-Zierrahmen, Messing mit Nieten (maritim), Briefmarke.

## Umsetzungsplan (Schritt 6)

- Rahmen unabhängig vom Thema wählbar (Gestaltung), Vorschau je Rahmen als Kachel, «Ohne Rahmen» als erste Wahl.
- Etappe A: Plätze 1 bis 5. Etappe B: Plätze 6 bis 10.
- Technik: Rahmen sind Zeichenfunktionen in `crates/core/src/thema_render.rs` (Export, Pixel) und CSS-Klassen `rm-*` in `ui/app.css` (Vorschau). Heute gibt es `Rahmen::{Keiner, Weiss, Linie, Polaroid}` in `gestaltung.rs`.
- Später: Rahmen pro Ereignis oder pro Seite wählbar.
- Stand: Design-Vorschlag gezeigt (Board «Schritt 6»), Bestätigung des Auftraggebers für Etappe A steht aus.
