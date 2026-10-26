# Fotobuch-Assistent: Konzept (Version 0.3)

Stand: 24.10.2026. Ein erster lauffähiger Kern existiert (siehe `src/`), die Oberfläche noch nicht.

Änderungen gegenüber v0.2:
- **Auslieferung als einzelne `.exe` ohne Voraussetzungen** (Python wird eingepackt, PyInstaller). Auf dem Zielrechner muss nichts installiert werden.
- **Lizenzmodell: Open-Source-Kern**, Komfort-Version kostenpflichtig. Alle Abhängigkeiten müssen dafür lizenzverträglich sein.
- **Formate:** JPG, HEIC (pillow-heif) und PNG laufen. RAW zuerst über die eingebettete Vorschau (rawpy), keine volle RAW-Entwicklung.
- **Tempoziel:** höchstens 15 Minuten pro 1000 Fotos auf einem normalen PC ohne Grafikkarte.
- **Erste Anbieter-Profile** für Cewe/Pixum und Ifolor angelegt, noch ohne geprüfte Werte (`verifiziert: false`).
- **Teilbarer Bericht:** Läufe mit echten Fotos erzeugen einen Bericht ohne Bilder, Namen, Pfade und GPS.

## 1. Problemstellung

Der Aufwand eines Fotobuchs steckt in der Vorarbeit, nicht im Gestalten:

1. **Reihenfolge:** Fotos kommen von verschiedenen Geräten und Ordnern und müssen zeitlich und thematisch sortiert werden.
2. **Ereignisse:** Fotos eines Tages oder Ausflugs sollen zusammenbleiben, auf einer Seite oder in räumlicher Nähe im Buch.
3. **Reduktion:** Doppelte, dreifache und fast identische Bilder werden auf das beste reduziert.
4. **Hoch/Quer:** Bei gemischten Formaten wird entschieden, welches Bild ganz und welches als Ausschnitt gezeigt wird.
5. **Seitenplanung:** Fotozahl, Seitenzahl, «es fehlen N Fotos bis zur nächsten Seitenzahl», maximale Fotos pro Seite, Einzel-, Doppel- und Panoramaseiten (sparsam eingesetzt).
6. **Überblick:** Alle Fotos sehen, ordnen und gezielt eingreifen, das Organisatorische übernimmt die Software.

**Harte Randbedingung:** Alles lokal und offline. Keine Fotos ins Internet, keine Cloud-KI. Wo KI nötig ist, nur kleine Modelle auf dem eigenen Rechner.

## 2. Wettbewerb (Recherche Oktober 2026)

Drei Lager, keines deckt das Gesamtproblem:

- **Fotobuch-Anbieter** (Cewe, Ifolor, Fuji.ch, Smartphoto, Google Fotos, Shutterfly, HappyFoto): Assistenten verteilen Fotos auf Seiten, teils mit Duplikat- und Qualitätsfilter. Nachteile: Anbieterbindung, kaum steuerbare Auswahl, oft Upload, kein Seitenbudget.
- **Culling-Software für Fotografen** (Aftershoot, Narrative Select, Imagen, FilterPixel, Photo Mechanic): Gruppieren Duplikate, erkennen unscharfe Bilder und geschlossene Augen, teils lokal. Nachteile: für Profis und RAW gebaut, meist Abo, enden bei der Auswahl.
- **Duplikat-Finder und Hobby-Tools** (Immich-Deduper, DuoBolt, Best Photo Picker u. a.): Ähnliche Bilder finden, teils lokal. Nachteil: keine Buchlogik.

Ergebnis: Ich fand kein Produkt, das offline, Ereignisse, Best-of-Auswahl, Seitenplanung und anbieterunabhängigen Export kombiniert. Das ist keine Garantie. Die Suche wird vor einem Verkauf wiederholt.

## 3. Differenzierung

1. Privat by design: kein Upload, kein Konto.
2. Planung in Seiten statt nur Layout, mit laufendem Seitenzähler.
3. Vorschläge, die erklärbar und umkehrbar sind.
4. Anbieterunabhängiger Export über Profile.
5. Für Privatpersonen: kein RAW-Workflow, kein Abo.
6. Bewusste Hoch/Quer-Paarung mit motivbewusstem Zuschnitt.

## 4. Export und Anbieter-Strategie

Es gibt **kein einheitliches Fotobuch-Format**. Die grossen Anbieter sind geschlossene Systeme (Cewe-Software dient auch Pixum, WhiteWall und dm). PDF-Upload wird nur von einem Teil der Dienste angeboten, und die Anforderungen weichen ab (Einzel- oder Doppelseiten, Beschnitt, Dateien, Seitenzahl-Schritte). Ob ein bestimmter Anbieter eigene PDFs annimmt, ist pro Anbieter zu prüfen und bisher **nicht verifiziert**.

Drei Schichten:

1. **Offenes Planungsformat** (JSON plus sortierte Bilder): Ereignisse, Reihenfolge, Seiten, Doppelseiten, Foto-Zuordnung, Zuschnitt. Voll dokumentiert.
2. **Universelle Exporte** aus dem Plan: JPEG pro Seite oder Doppelseite, PDF (Einzel- oder Doppelseiten, später PDF/X-4, 300 DPI, Beschnitt einstellbar, Umschlag getrennt).
3. **Anbieter-Profile** (siehe [profiles/](../profiles/)): Format, Seitenminimum und Schrittweite, Beschnitt, Dateiart, Dateinamen. Neue Anbieter lassen sich ohne Programmieren ergänzen.

Für geschlossene Systeme gibt es einen **assistierten Weg**: Seitenbilder plus Seitenliste, die der Nutzer in der Anbieter-Software einfügt. Direkter Import von Anbieter-Projektdateien erst später und nur, wenn technisch und rechtlich sauber.

Schnittstellen für Dritte sind **kein Ziel**. Gemeint ist ausschliesslich der Export zu Druckdiensten.

## 5. Technologie-Vorschlag

| Schicht | Wahl | Begründung |
|---|---|---|
| Kern | Python, Daten in SQLite | Schnell iterierbar, Pillow, pillow-heif, imagehash, onnxruntime |
| Oberfläche MVP | Wird mit den Design-Vorschlägen entschieden (siehe [design.md](design.md)) | Zeichenfläche liegt zur Auswahl vor |
| Oberfläche MMP | Tauri 2 | Deutlich kleiner als Electron, nutzt unter Windows WebView2 |
| Seitenplanung | Eigene Logik (dynamische Programmierung, ggf. OR-Tools) | Kernprodukt |
| Export | JPEG pro Seite, dann PDF, Profile als Dateien | JPEG ist am universellsten |
| Plattform | Windows zuerst, danach macOS | Entscheidung des Auftraggebers |

### Leistungsstufen für schwache Rechner

- **Stufe 0, überall, ohne KI-Modell:** EXIF-Zeit und GPS, Ereignisse, exakte Duplikate (Hash), Ähnlichkeit über Perceptual Hash, Schärfe und Belichtung, komplette Seitenplanung, Export.
- **Stufe 1, kleines Modell auf der CPU (ONNX Runtime):** Bild-Embeddings für echte Ähnlichkeit, zum Beispiel SigLIP 2 Base (Apache 2.0). Herstellerangabe für die CPU: etwa 47 ms pro Bild, also grob 8 Minuten pro 10'000 Fotos plus Dekodierzeit (Schätzung, in Etappe 0 zu messen). Alternative MobileCLIP, Lizenz vor Verkauf prüfen.
- **Stufe 2, GPU optional (unter Windows DirectML):** Schneller, plus optional kleines Sprachmodell für Themen-Tags. Nie Pflicht.

Das Programm erkennt die Hardware und wählt die Stufe selbst. Ergebnisse werden zwischengespeichert, ein Foto wird nur einmal analysiert.

### Externe Bausteine und Lizenzen

- **Gesichter und Augen:** MediaPipe (Apache 2.0). InsightFace vermeiden, dessen Gewichte sind laut mehreren Quellen nur für nichtkommerzielle Nutzung.
- **Schönheitsbewertung:** LAION-Aesthetic-Predictor nur schwach gewichten. Stattdessen aus den Auswahlen des Nutzers lernen (kleines Modell auf Embeddings). Hypothese, in Etappe 0/MVP 2 zu prüfen.
- **Formate:** JPG zuerst, HEIC über pillow-heif, RAW später über LibRaw.

## 6. Etappenplan

### Etappe 0: Machbarkeit (ca. 1 Woche)
Siehe [etappe-0.md](etappe-0.md). Ergebnis: kurzer Bericht mit Erkennungsqualität, Tempo, Anbieter-Dateianforderungen.

### MVP 1: Ordnen und Überblick (kein KI-Modell)
Import, Sortierung nach Aufnahmezeit, Ereignisse (umbenennbar, verschiebbar), exakte Duplikate, Seitenzähler, Export als sortierte Ordner und Liste. Abnahme an einem echten Buchprojekt.

### MVP 2: Aussortieren und Seitenplan
Ähnliche Bilder gruppieren, Best-of-Vorschlag mit Bestätigung, unscharfe Bilder markieren, Seitenplan mit Einzel-, Doppel- und Panoramaseiten, Hoch/Quer-Paarung, Buchvorschau, Seitenexport, erste Anbieter-Profile. Abnahme an einem echten Projekt.

### MMP: Verkaufbares Produkt
weitere Gestaltungsvorlagen und Thema pro Ereignis (Grundlage Theming ist umgesetzt), Seiten einfügen und umordnen, echte Anbieter-Profile (Seitenregeln heute Annahmen), Feinschliff der Oberfläche (zum Beispiel abgeschnittener «Blättern»-Knopf), polierte Oberfläche, Installer, Hilfe, Hardware-Stufen, weitere Profile, Robustheit bei über 10'000 Fotos, Lizenzmodell (einmaliger Kauf), Datenschutz-Seite, Wettbewerbsprüfung wiederholen, Test mit 3 bis 5 Fremden.

## 7. Zusammenarbeit

- Pro Etappe ein lauffähiges, abnehmbares Ergebnis. Der Auftraggeber prüft fachlich.
- Pro Etappe eine Zusammenfassung «Fertig / Offen / Entscheidungen» in [fortschritt.md](fortschritt.md).
- **Fotos bleiben beim Auftraggeber.** Läufe mit echten Fotos macht er lokal und teilt nur Berichte (Zahlen, Gruppenlisten, Dateinamen).

## 8. Risiken

1. **Qualität der Bestenauswahl** ist der schwierigste Teil. Deshalb immer als Vorschlag.
2. **Anbieter-Import:** Annahme und Anforderungen nur teilweise dokumentiert. Test in Etappe 0.
3. **PDF/X-4 und Farbraum** sind technisch anspruchsvoll. JPEG pro Seite ist der Rückfall.
4. **Verpackung für mehrere Betriebssysteme** kostet im MMP mehr Zeit als erwartet.
5. **Modelle und Lizenzen ändern sich schnell.** Die Auswahl ist ein Startpunkt und wird gegen echte Fotos getestet.

## Quellen (Auswahl)

PCtipp Fotobuch-Test, Google Fotos Fotobuch, Aftershoot und Narrative Select (Herstellerseiten, Konkurrenzvergleiche mit Vorsicht lesen), Immich-Deduper, Best Photo Picker, Cloudprinter und Flipsnack (Druckdaten-Anforderungen), SigLIP 2 ONNX/Core ML (Hugging Face), MobileCLIP, Tauri-Vergleich (Better Stack), MediaPipe-Face ONNX (Hugging Face), LAION-Aesthetics-Audit (arXiv 2601.09896).
