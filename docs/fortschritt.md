# Fortschritt

Wird nach jedem Arbeitsschritt ergänzt. Neueste Einträge oben.

## 2026-10-25 (nachts): Neues Erscheinungsbild (Variante C «Werkbank»)

**Fertig**
- Dunkle Schiene links mit dem Ablauf von oben nach unten (Schritte 1 bis 6 mit Verbindungslinie, Haken bei erledigten, Zahlen je Schritt), hellgraue Arbeitsfläche, blaue Akzentfarbe, Hell- und Dunkelmodus
- Schmale Bildschirme: Schiene wird zur Leiste oben

## 2026-10-25 (abends): Buchstruktur wie gedruckt, Layout pro Seite wählbar

**Fertig**
- Seite 1 steht allein, danach Doppelseiten (2–3, 4–5, …), in Blättern, Übersicht und Sprung
- Pro Seite bietet die App bis zu 8 Layouts an (beste zuerst). «Layout wählen» zeigt sie als kleine Vorlagen-Vorschauen mit den echten Fotos, ein Klick wählt. Die Wahl wird im Projekt gespeichert
- Tests: Layout-Varianten in Rust, Oberflächentest für Einzelseite, Doppelseiten und Layoutwechsel

**Offen**
- Eigene Layout-Vorlagen und freies Verschieben der Fotos (bewusst später)

## 2026-10-25 (später): Buchansicht, Layouts und Begründungen (Version 0.2, Testlauf 2)

**Fertig**
- Neuer Schritt «Buch»: Doppelseiten mit automatischem Layout, Blättern (Knöpfe, Pfeiltasten, Sprung zu Seite), Übersicht mit Grössenregler, Detailansicht mit Entfernen/Zurückholen
- Layout: Schnittbaum-Suche (nebeneinander/untereinander, verschachtelt) maximiert die belegte Fläche, ohne Beschnitt. Seitenformat quer, quadratisch, hoch einstellbar. Reihenfolge nach Aufnahmezeit, Ereignisse beginnen auf neuer Seite
- Begründung zu jedem Foto (Duplikat von …, Serie mit Qualitätsvergleich, Screenshot, Ortsbegrenzung mit Rang, eigene Entscheidung), Filter nach Grund in «Aussortiert»
- Grössenregler für Bilder beim Aussortieren, grössere Standardansicht
- Tests: 9 Oberflächentests (neu: Buchansicht), Layout-Tests in Rust

**Nicht drin / offen**
- Keine echten Anbieter-Layouts, kein Umschlag, kein Panorama oder Vollseitenfoto nach Wunsch, keine Handarbeit (Fotos zwischen Seiten verschieben)
- Gleiche Fotos-pro-Seite-Zahl auf (fast) allen Seiten, daher wirken Seiten ähnlich. Abwechslung (zum Beispiel eine Seite mit nur einem Highlight-Foto) ist ein sinnvoller nächster Schritt

## 2026-10-25: Umbau auf Rust und Tauri (Version 0.2)

**Fertig**
- Kern komplett in Rust neu geschrieben (`crates/core`): Einlesen (JPEG, HEIC, PNG, TIFF/WebP/BMP, RAW über eingebettete Vorschau), Ereignisse, Orte, Serien, Seitenzähler, Export, Zwischenspeicher. Projektdatei `.fotobuch/projekt.json` ist kompatibel mit der Python-Version
- HEIC rein in Rust (Crate `heic`), an vier echten Dateien gegen pillow-heif geprüft (PSNR über 71 dB) und mit EXIF-Zeit und GPS getestet
- Eigenes App-Fenster mit Tauri 2 (`crates/app`): keine Konsole, kein Browser, nativer Ordner-Dialog, externe Links öffnen im Standardbrowser. Oberfläche unverändert (`ui/`, in das Programm eingebettet)
- Vergleich mit der Python-Version auf den 300 New-York-Demofotos: Ereignisse, Orte, exakte Duplikate und Seitenplan gleich; kleine Abweichungen bei der Schärfewertung (um 1 bis 3 Punkte), dadurch bei einzelnen Serien ein anderes Vorschlagsfoto
- Tests: Rust-Kerntests plus Ende-zu-Ende-Test, 8 Oberflächentests laufen gegen den Rust-Server. Fenster unter Xvfb gestartet und per Screenshot geprüft (Linux)
- Pipeline baut portable Programme: Windows `.exe`, Mac `.app` (Apple Silicon, ad-hoc signiert) als ZIP. Kein Installer
- Analyse von 300 Fotos (86 MB) auf 2 Kernen: etwa 2 Sekunden (Python-Version: 3,2 Sekunden)

**Nicht geprüft**
- Start unter echtem Windows und echtem macOS (Gatekeeper, WebView2, Ordner-Dialog)
- Echtes RAW (CR3 hat evtl. keine lesbare Aufnahmezeit), iPhone-HEIC mit GPS aus dem Gerät
- Der Python-Kern ist entfernt (Git-Verlauf enthält ihn)

## 2026-10-24: Erste lauffähige App (zweiter Wurf Richtung MVP 1)

**Fertig**
- App mit geführter Oberfläche (Richtung C plus A-Elemente), fünf Schritte: Fotos laden, Ereignisse (umbenennbar, Zeitlücke einstellbar), Aussortieren (Serien per Klick und Tastatur, Duplikate/Screenshots zurückholbar), Seiten (Seitenzähler, Auffüllen/Kürzen mit konkreten Fotos, Seitenregeln einstellbar), Export (nummerierte Ereignis-Ordner, `seitenliste.csv`, `LIESMICH.txt`)
- Lokaler Server nur auf 127.0.0.1 mit Token und Host-Prüfung, Oberfläche ohne externe Ressourcen
- Entscheidungen und Analyse bleiben im Fotoordner (`.fotobuch`) gespeichert, zweiter Start liest nur Neues
- Handbuch ([handbuch.md](handbuch.md)), Testkonzept ([testen.md](testen.md))
- Tests: 34 Python-Tests (Kern und App-Schicht inkl. Sicherheit) und 7 Oberflächentests, die die echte App in jsdom bedienen. Alle grün (Linux)
- Pipeline: CI führt beide Testarten aus, baut die Windows-exe und startet sie als Selbsttest

**Nicht geprüft**
- Windows-exe nie von Hand ausgeführt (Ordner-Dialog, Browserstart, SmartScreen-Hinweis)
- Aussehen im echten Browser nicht angeschaut (kein Browser in der Entwicklungsumgebung); nur Verhalten ist getestet
- Echte Fotos, echtes RAW, Anbieter-Seitenregeln wie bisher offen

**Bewusst noch nicht drin (MVP 2)**
- Buchvorschau mit Doppelseiten, Hoch/Quer-Paarung, Panorama, Seitenexport als JPEG/PDF, Ereignisse verschieben oder zusammenlegen, echte Anbieter-Importdateien

## 2026-10-24: Erste Iteration Richtung MVP 1

**Fertig**
- Lauffähiger Kern (`src/fotobuch/`): Einlesen mit Aufnahmezeit, GPS, Kamera, Format und Drehung; Ereignisse nach Zeitlücke und Ortssprung; exakte Duplikate per Datei-Hash; Serien per Differenzhash mit zeitlich begrenzter Spanne; Vorschlag nach Schärfe und Auflösung; Seitenzähler mit Profilen; Export in nummerierte Ereignis-Ordner (kopiert, Originale bleiben)
- Bericht ohne Bilder: lokal `reihenfolge.csv` und `bericht.md`, teilbar `bericht-teilbar.json` (anonymisierte Namen, keine Pfade, kein GPS, kein Ordnername), Zuordnung getrennt in `zuordnung-NICHT-TEILEN.csv`
- 17 automatische Tests mit synthetischen Bildern, alle bestanden
- Doppelklick-Modus: Ohne Argumente öffnet die `.exe` einen Ordner-Dialog und legt die Berichte im Ordner `fotobuch-bericht` ab
- Build-Pipeline für die Windows-`.exe` (GitHub Actions, PyInstaller). Eine Linux-Probe (43 MB) lief eigenständig
- Drei Oberflächen-Vorschläge zur Auswahl (siehe [design.md](design.md))

**Messwerte (nur Richtwert, Cloud-Umgebung mit 2 Kernen, synthetische 12-MP-JPEGs)**
- Etwa 45 s pro 1000 Fotos für Einlesen und Analyse. Das Ziel von 15 Minuten ist damit auf dieser Hardware weit unterschritten. Echte Werte kommen vom Windows-Test, besonders für HEIC und RAW.

**Gefundene Probleme und Behebung**
- Der Test deckte auf, dass der teilbare Bericht den Ordnernamen enthielt. Behoben, durch Test abgesichert.
- Ähnliche Bilder wurden zu einer Riesen-Serie verkettet. Serien sind jetzt zeitlich begrenzt (Standard 30 Minuten Spanne), durch Test abgesichert.

**Nicht geprüft**
- Die Windows-`.exe` wurde noch nicht gebaut und nie auf Windows ausgeführt.
- HEIC und RAW wurden nur mit erzeugten Testdateien geprüft (HEIC), RAW gar nicht (kein rawpy-Test mit echten RAW-Dateien).
- Die Seitenzahl-Regeln der Anbieter sind Annahmen (24 Seiten, Schritt 2), die Profile für Cewe/Pixum und Ifolor haben noch keine geprüften Werte.
- Die Qualität der Vorschläge ist nur an synthetischen Bildern geprüft, nicht an echten Fotos.

**Entscheidungen des Auftraggebers (heute)**
- Auslieferung als einzelne `.exe` ohne Voraussetzungen
- Formate: JPG, HEIC, RAW, PNG/Screenshots
- Projektgrösse: 500 bis 2000 Fotos; Druckdienste: Cewe/Pixum und Ifolor
- Testgerät: normaler Windows-PC ohne Grafikkarte; Tempo höchstens 15 Minuten pro 1000 Fotos
- Läufe mit echten Fotos lokal, nur der teilbare Bericht geht an den Entwickler
- Lizenz: Open-Source-Kern, Komfort-Version kostenpflichtig

**Offen**
- Oberfläche wählen: A Zeitleiste, B Buch zuerst oder C Assistent
- Genaue Open-Source-Lizenz für den Kern wählen (z. B. Apache-2.0 oder MIT). Bis dahin liegt bewusst keine LICENSE-Datei im Repository
- Windows-`.exe` aus der Pipeline bauen lassen und auf dem Testrechner ausführen
- Testordner, Buchformat und Seitenzahl bei Cewe und Ifolor, RAW-Kameramarke, endgültiger Projektname

## 2026-10-24: Repository aufgesetzt

**Fertig**
- Repository mit Grundstruktur: README, `.gitignore` (keine Bilder, RAW, Modelle oder Datenbanken), `docs/`, `profiles/`, `tests/`
- Konzept v0.2 (Windows zuerst, Export-Strategie, Technik, Risiken)
- Plan für Etappe 0

**Offen**
- Eckdaten vom Auftraggeber: Windows-Rechner, Python-Version, Testordner, Druckdienst mit Buchformat und Seitenzahl
- Test-Skript für Etappe 0 (wird geschrieben, sobald die Eckdaten da sind)

**Entscheidungen für den Auftraggeber**
1. Lizenz des Projekts (offen, privat bleiben oder später veröffentlichen?)
2. Endgültiger Projektname (Arbeitstitel: Fotobuch-Assistent)
3. Schwelle für akzeptable Analysezeit pro 1000 Fotos
4. Erfolgskriterien von Etappe 0 bestätigen

## 2026-10-23/24: Konzeptphase

- Problemverständnis und Wettbewerbsrecherche (drei Lager, keine passende Lösung gefunden)
- Export-Recherche: kein einheitliches Fotobuch-Format, PDF nur bei einem Teil der Dienste
- Technologie-Vorschlag mit drei Leistungsstufen für schwache Rechner
- Entscheidungen des Auftraggebers: Windows zuerst, Schnittstellen nur als Export zu Druckdiensten, Fotos bleiben lokal

## MVP 1 abgeschlossen (2026-10-25)
Ereignisse lassen sich von Hand anpassen: mit dem vorherigen zusammenlegen, an einem gewählten Foto teilen, Änderungen gesammelt zurücksetzen. Die Änderungen werden im Projekt gespeichert und überleben eine neue Analyse. Offen aus MVP 1: nur die Abnahme am echten Buchprojekt (Aufgabe der Anwenderin/des Anwenders).

## MVP 2, erste Runde (2026-10-25)
**Fertig**
- **Fertige Seiten:** Jede Buchseite wird als JPEG (300 dpi, 3508 × 2631 bei Quer 4:3) gerendert, dazu eine PDF mit allen Seiten (`seiten/fotobuch.pdf`). Läuft im Hintergrund mit Fortschrittsanzeige, freie Seiten gehen als weisse Seiten mit. Die Originale werden nur gelesen.
- **Panorama und ganze Seite:** Fotos ab Seitenverhältnis 2.2:1 bekommen automatisch eine eigene Seite, jedes andere Foto lässt sich in der Detailansicht als «ganze Seite» setzen. Einzelseiten bieten zusätzlich das Layout «Randlos» (füllt die Seite, beschneidet). Der Seitenzähler rechnet die Ganzseiten mit.
- **Hoch/Quer-Paarung:** An den Seitengrenzen werden Fotos getauscht, wenn beide Seiten dadurch deutlich besser gefüllt sind (z. B. ein Hochformat neben zwei Querformaten). Die Reihenfolge weicht höchstens um eine Seitengrenze von der Aufnahmezeit ab.
- Fotos ohne Aufnahmezeit bilden das Schluss-Ereignis «Ohne Aufnahmezeit».
- Test: Ende-zu-Ende-Lauf mit Bildern pro Schritt als Galerie (CI-Artefakt `album-ende-zu-ende`).

**Offen in MVP 2**
- Fotos von Hand zwischen Seiten verschieben
- Panorama über eine ganze Doppelseite
- Echte Anbieter-Profile (Seitenregeln heute Annahmen)
- Abnahme an einem echten Buchprojekt

**Später (MMP): Gestaltung/Theming**
Kombination aus Farbtönen, Hintergründen, Deckblättern und weiteren Gestaltungsvorlagen, die sich pro Buch wählen lässt.

## MVP 2, zweite Runde (2026-10-26)
**Fertig**
- **Fotos zwischen Seiten verschieben:** In der Detailansicht eines Fotos im Buch: auf die vorherige oder nächste Seite, auf eine gewählte Seitennummer, oder zurück an den Ursprung. «Alle Verschiebungen zurücksetzen» steht unter dem Buch. Verschiebungen werden im Projekt gespeichert, erscheinen auch im Export (Seitenliste, JPEG, PDF) und lassen sich nicht auf Seiten mit ganzseitigem Foto legen. Leere Seiten fallen weg.
- **Seitenart je Foto:** «Mit anderen Fotos auf einer Seite», «Ganze Seite» oder «Über die ganze Doppelseite». Automatisch: ab 2.2:1 eigene Seite, ab 2.8:1 über die Doppelseite. Ein Doppelseiten-Foto steht immer auf einer linken Seite (gerade Seitennummer); passt das nicht, rückt die nächste Seite davor. Der Seitenzähler rechnet zwei Seiten.
- Export und Vorschau zeigen das Foto über beide Seiten ohne Bruch in der Mitte.

**Offen in MVP 2**
- Abnahme an einem echten Buchprojekt (Anwender)

**In die MMP-Phase verschoben (2026-10-26):** Neue Seite einfügen und Seiten umordnen, echte Anbieter-Profile, Feinschliff der Oberfläche, Theming.

## 2026-10-26: Gestaltung (Theming) umgesetzt

Neuer Schritt «Gestaltung» (zwischen Seiten und Buch, Export ist jetzt Schritt 7). Sechs Themen: Klar, Leinen, Nacht, Kraft, Meer, Galerie. Jedes Thema bringt Farben, Seitenhintergrund (flach, Kraftpapier, Welle), Fotorahmen (keiner, weisser Rand, dünner Strich, Sofortbild leicht schief), Schrift und ein eigenes Deckblatt mit. Anpassbar sind Farbton, Seitenhintergrund, Fotorahmen, Unterschrift und Seitenzahl sowie Titel, Untertitel (Standard: Zeitraum aus den Aufnahmezeiten), Name und Titelfoto. Die Wahl gilt pro Buch und wird im Projekt gespeichert.
Der Export zeichnet Deckblatt (`deckblatt.jpg`, `deckblatt.pdf`) und alle Seiten im Thema (300 dpi). Das Deckblatt ist eine eigene Datei, die Seitenzahl des Buchs ändert sich nicht. Schriften sind eingebunden (siehe `docs/schriften.md`), alles bleibt offline.
Tests: Rust (Themen, Zeichnen, Export), jsdom (Themenwahl, Anpassen), Browser-Durchlauf mit Export aller sechs Themen und Pixelprüfung. Bilder: [Galerie Runde 3](galerie/runde-3.md).
Noch offen: Thema pro Ereignis, weitere Themen (Polaroid, Hochzeit, Reise-Tagebuch), eigene Hintergrund- und Deckblattbilder, Deckblatt-Vorgaben der echten Druckdienste.
Aufklappfenster (Layout wählen) hängen am Knopf, wählen oben oder unten nach Platz, werden bei Platzmangel scrollbar und bleiben immer im Fenster, ohne Fenster oder Seite zu vergrössern. Der Browser-Durchlauf prüft das bei normalem und kleinem Fenster. Auswahllisten (Seitenformat, Rahmen usw.) und Kontextmenüs zeichnet das Betriebssystem selbst im Fenster.
